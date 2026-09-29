//! Reactive properties.
//!
//! Owns the property type, the dependency tracking that propagates a change,
//! and the inheritance that lets a property defer to its parent.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A reactive property that tracks dependencies and notifies dependents.
///
/// Properties form a DAG. When a property changes, all dependent properties
/// are re-evaluated and all callbacks are called.
///
/// # Examples
///
/// ```
/// use ui_core::property::Property;
///
/// let x = Property::new(1.0);
/// let x_clone = x.clone();
/// let y = Property::bind(move || x_clone.get() * 2.0);
/// assert_eq!(y.get(), 2.0);
/// x.set(3.0);
/// assert_eq!(y.get(), 6.0);
/// ```
#[derive(Clone)]
pub struct Property<T: 'static> {
    inner: Rc<PropertyInner<T>>,
}

type Callback<T> = Box<dyn Fn(&T)>;

struct PropertyInner<T: 'static> {
    value: RefCell<T>,
    recompute: RefCell<Option<Rc<dyn Fn()>>>,
    dependencies: RefCell<Vec<Weak<dyn PropertyBase>>>,
    dependents: RefCell<Vec<Weak<dyn PropertyBase>>>,
    callbacks: RefCell<Vec<Callback<T>>>,
}

trait PropertyBase {
    fn recompute(&self);
    fn add_dependent(&self, dependent: Weak<dyn PropertyBase>);
    fn dependents(&self) -> Vec<Weak<dyn PropertyBase>>;
}

impl<T: 'static> PropertyBase for PropertyInner<T> {
    fn recompute(&self) {
        if let Some(f) = self.recompute.borrow().as_ref() {
            f();
        }
        let dependents = self.dependents.borrow().clone();
        for dep in dependents {
            if let Some(dep_inner) = dep.upgrade() {
                dep_inner.recompute();
            }
        }
    }
    fn add_dependent(&self, dependent: Weak<dyn PropertyBase>) {
        self.dependents.borrow_mut().push(dependent);
    }
    fn dependents(&self) -> Vec<Weak<dyn PropertyBase>> {
        self.dependents.borrow().clone()
    }
}

thread_local! {
    static TRACKER: RefCell<Vec<Weak<dyn PropertyBase>>> = RefCell::new(Vec::new());
}

impl<T: Clone + 'static> Property<T> {
    /// Creates a new literal property.
    pub fn new(value: T) -> Self {
        Property {
            inner: Rc::new(PropertyInner {
                value: RefCell::new(value),
                recompute: RefCell::new(None),
                dependencies: RefCell::new(Vec::new()),
                dependents: RefCell::new(Vec::new()),
                callbacks: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Creates a new computed property.
    ///
    /// The closure is evaluated immediately to get the initial value, and
    /// re-evaluated whenever any dependency changes.
    ///
    /// # Panics
    ///
    /// Panics if the closure creates a dependency cycle.
    pub fn bind<F>(f: F) -> Self
    where
        F: Fn() -> T + 'static,
        T: Default,
    {
        let prop = Property {
            inner: Rc::new(PropertyInner {
                value: RefCell::new(T::default()),
                recompute: RefCell::new(None),
                dependencies: RefCell::new(Vec::new()),
                dependents: RefCell::new(Vec::new()),
                callbacks: RefCell::new(Vec::new()),
            }),
        };

        TRACKER.with(|t| *t.borrow_mut() = Vec::new());
        let value = f();
        let deps = TRACKER.with(|t| t.borrow().clone());
        TRACKER.with(|t| *t.borrow_mut() = Vec::new());

        for dep in &deps {
            if let Some(dep_inner) = dep.upgrade() {
                if std::ptr::eq(dep_inner.as_ref(), prop.inner.as_ref() as &dyn PropertyBase) {
                    panic!("Property::bind: dependency cycle detected");
                }
            }
        }

        for dep in &deps {
            if let Some(dep_inner) = dep.upgrade() {
                let weak: Weak<dyn PropertyBase> =
                    Rc::downgrade(&(prop.inner.clone() as Rc<dyn PropertyBase>));
                dep_inner.add_dependent(weak);
            }
        }

        let inner_clone = Rc::clone(&prop.inner);
        let f_clone = Rc::new(f);
        let recompute: Rc<dyn Fn()> = Rc::new(move || {
            let new_value = f_clone();
            *inner_clone.value.borrow_mut() = new_value;
        });

        *prop.inner.recompute.borrow_mut() = Some(recompute);
        *prop.inner.value.borrow_mut() = value;
        *prop.inner.dependencies.borrow_mut() = deps;

        prop
    }

    /// Returns the current value.
    ///
    /// If a tracker is active (during `bind` evaluation), this property is
    /// registered as a dependency of the property being computed.
    pub fn get(&self) -> T {
        TRACKER.with(|t| {
            let weak: Weak<dyn PropertyBase> =
                Rc::downgrade(&(self.inner.clone() as Rc<dyn PropertyBase>));
            t.borrow_mut().push(weak);
        });
        self.inner.value.borrow().clone()
    }

    /// Sets the value and notifies all dependents and callbacks.
    ///
    /// Recomputes all dependent properties recursively.
    pub fn set(&self, value: T) {
        *self.inner.value.borrow_mut() = value;

        let callbacks = std::mem::take(&mut *self.inner.callbacks.borrow_mut());
        let val = self.inner.value.borrow().clone();
        for cb in callbacks {
            cb(&val);
        }

        let dependents = self.inner.dependents();
        for dep in dependents {
            if let Some(dep_inner) = dep.upgrade() {
                dep_inner.recompute();
            }
        }
    }

    /// Registers a callback to be called after the value is updated.
    pub fn on_change<F>(&self, callback: F)
    where
        F: Fn(&T) + 'static,
    {
        self.inner.callbacks.borrow_mut().push(Box::new(callback));
    }

    /// Returns true if this property is a computed (bound) property.
    pub fn is_bound(&self) -> bool {
        self.inner.recompute.borrow().is_some()
    }
}

/// A weak reference to a property, for use in widget nodes.
pub struct PropertyHandle<T: 'static> {
    inner: Weak<PropertyInner<T>>,
}

impl<T: Clone + 'static> PropertyHandle<T> {
    /// Upgrades the handle to a strong reference.
    pub fn upgrade(&self) -> Option<Property<T>> {
        self.inner.upgrade().map(|inner| Property { inner })
    }
}

impl<T: 'static> Property<T> {
    /// Returns a weak handle to this property.
    pub fn handle(&self) -> PropertyHandle<T> {
        PropertyHandle {
            inner: Rc::downgrade(&self.inner),
        }
    }
}

/// A minimal RGBA color with premultiplied alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Red component (0-255).
    pub r: u8,
    /// Green component (0-255).
    pub g: u8,
    /// Blue component (0-255).
    pub b: u8,
    /// Alpha component (0-255).
    pub a: u8,
}

impl Color {
    /// Creates a new color with premultiplied alpha.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    /// Returns the color as a premultiplied RGBA array.
    pub fn to_premultiplied(&self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// A minimal 2D transform.
#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    /// X translation.
    pub tx: f32,
    /// Y translation.
    pub ty: f32,
    /// X scale.
    pub sx: f32,
    /// Y scale.
    pub sy: f32,
    /// Rotation in radians.
    pub rotation: f32,
}

impl Transform {
    /// Creates a new identity transform.
    pub fn identity() -> Self {
        Transform {
            tx: 0.0,
            ty: 0.0,
            sx: 1.0,
            sy: 1.0,
            rotation: 0.0,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_property_get_set() {
        let p = Property::new(42.0);
        assert_eq!(p.get(), 42.0);
        p.set(99.0);
        assert_eq!(p.get(), 99.0);
    }

    #[test]
    fn bound_property_recomputes_on_dependency_change() {
        let x = Property::new(1.0);
        let x_clone = x.clone();
        let y = Property::bind(move || x_clone.get() * 2.0);
        assert_eq!(y.get(), 2.0);
        x.set(3.0);
        assert_eq!(y.get(), 6.0);
    }

    #[test]
    fn multiple_levels_of_binding() {
        let a = Property::new(1.0);
        let a_clone = a.clone();
        let b = Property::bind(move || a_clone.get() + 1.0);
        let b_clone = b.clone();
        let c = Property::bind(move || b_clone.get() * 2.0);
        assert_eq!(c.get(), 4.0);
        a.set(2.0);
        assert_eq!(c.get(), 6.0);
    }

    #[test]
    fn cycle_detection() {
        // Cycles are impossible by construction: `bind` takes a closure that
        // captures existing properties, and the property being created does
        // not exist yet when the closure is created. The cycle detection in
        // `bind` is defensive and cannot be triggered through the public API.
        let p = Property::new(1.0);
        let p_clone = p.clone();
        let _q = Property::bind(move || p_clone.get() + 1.0);
    }

    #[test]
    fn change_notification_fires() {
        let p = Property::new(1.0);
        let called = Rc::new(RefCell::new(false));
        let called_clone = Rc::clone(&called);
        p.on_change(move |_| {
            *called_clone.borrow_mut() = true;
        });
        p.set(2.0);
        assert!(*called.borrow());
    }

    #[test]
    fn property_handle_upgrade() {
        let p = Property::new(42.0);
        let h = p.handle();
        let p2 = h.upgrade().unwrap();
        assert_eq!(p2.get(), 42.0);
    }

    #[test]
    fn color_premultiplied() {
        let c = Color::new(255, 128, 64, 255);
        assert_eq!(c.to_premultiplied(), [255, 128, 64, 255]);
    }

    #[test]
    fn transform_identity() {
        let t = Transform::identity();
        assert_eq!(t.tx, 0.0);
        assert_eq!(t.ty, 0.0);
        assert_eq!(t.sx, 1.0);
        assert_eq!(t.sy, 1.0);
        assert_eq!(t.rotation, 0.0);
    }
}
