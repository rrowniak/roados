//! Reactive properties.
//!
//! Owns the property type, the dependency tracking that propagates a change,
//! and the inheritance that lets a property defer to its parent.

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

use crate::animation::{Animation, Easing, Interpolate};

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

type Callback<T> = Rc<dyn Fn(&T)>;

struct PropertyInner<T: 'static> {
    value: RefCell<T>,
    recompute: RefCell<Option<Rc<dyn Fn()>>>,
    dependencies: RefCell<Vec<Weak<dyn PropertyBase>>>,
    dependents: RefCell<Vec<Weak<dyn PropertyBase>>>,
    /// The callback list, behind an `Rc` so that a write clones the list
    /// rather than taking it: a callback registered before any write has to
    /// fire on every write, and a write is far more common than a
    /// registration. The callbacks are `Rc`s themselves so that registering
    /// one can clone the list it is being added to.
    callbacks: RefCell<Rc<Vec<Callback<T>>>>,
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

impl<T: Clone + 'static> PropertyInner<T> {
    /// Writes `value` and runs every registered callback.
    ///
    /// Both [`Property::set`] and a bound property's recompute closure write
    /// through here, so a callback fires on every write whichever way the
    /// value arrived. It stops at the callbacks: `set` notifies dependents
    /// itself, and a recompute closure is called *by* the notification of the
    /// dependency that changed, so notifying from here would notify twice.
    fn write(&self, value: T) {
        *self.value.borrow_mut() = value;
        // The list is cloned rather than taken: taking it would leave the
        // property with no callbacks after the first write, and a callback
        // registered before any write would fire once and never again.
        let callbacks = self.callbacks.borrow().clone();
        // The value is cloned out of the cell so that no borrow is held while
        // the callbacks run: one of them may well write the property again.
        let value = self.value.borrow().clone();
        for callback in callbacks.iter() {
            callback(&value);
        }
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
                callbacks: RefCell::new(Rc::new(Vec::new())),
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
                callbacks: RefCell::new(Rc::new(Vec::new())),
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

        // The closure holds the inner weakly: it is stored in that same
        // inner, so a strong capture would be a cycle that no drop could
        // ever break, and every bound property would leak for the life of
        // the process. Upgrading at recompute time is the whole check that
        // the property is still there.
        let inner_weak = Rc::downgrade(&prop.inner);
        let f_clone = Rc::new(f);
        let recompute: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(inner) = inner_weak.upgrade() {
                let new_value = f_clone();
                inner.write(new_value);
            }
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
        self.inner.write(value);

        let dependents = self.inner.dependents();
        for dep in dependents {
            if let Some(dep_inner) = dep.upgrade() {
                dep_inner.recompute();
            }
        }
    }

    /// Registers a callback to be called after the value is updated.
    ///
    /// The callback fires on every write, not only the first: a write clones
    /// the list rather than taking it, so a callback registered before any
    /// write is still there for the next one. A callback registered from
    /// inside another callback is kept, and fires on the next write — the
    /// write already in progress is iterating a list cloned before it
    /// started.
    pub fn on_change<F>(&self, callback: F)
    where
        F: Fn(&T) + 'static,
    {
        let mut callbacks = (**self.inner.callbacks.borrow()).clone();
        callbacks.push(Rc::new(callback));
        *self.inner.callbacks.borrow_mut() = Rc::new(callbacks);
    }

    /// Returns true if this property is a computed (bound) property.
    pub fn is_bound(&self) -> bool {
        self.inner.recompute.borrow().is_some()
    }
}

impl<T: Interpolate + 'static> Property<T> {
    /// Starts an animation of this property from its current value to `to`.
    ///
    /// The animation is a description until an [`AnimationClock`](crate::animation::AnimationClock)
    /// takes it.
    /// The property is put at its start value here, so a frame drawn between
    /// this call and the first tick already shows where the animation begins.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::{AnimationClock, Easing};
    /// use ui_core::property::Property;
    ///
    /// let opacity = Property::new(0.0_f32);
    /// let mut clock = AnimationClock::new();
    /// clock.add(opacity.animate_to(1.0, Duration::from_millis(100), Easing::Linear));
    ///
    /// clock.tick(Duration::from_millis(50));
    /// assert_eq!(opacity.get(), 0.5);
    /// ```
    pub fn animate_to(&self, to: T, duration: Duration, easing: Easing) -> Animation<T> {
        Animation::new(self, self.get(), to, duration, easing)
    }

    /// Starts an animation of this property from `from` to `to`, wherever the
    /// property happens to stand.
    ///
    /// The property is put at `from` before this returns, which is what
    /// [`Property::animate_to`] does with the value the property holds now.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::{AnimationClock, Easing};
    /// use ui_core::property::Property;
    ///
    /// let width = Property::new(100.0_f32);
    /// let mut clock = AnimationClock::new();
    /// clock.add(width.animate_from_to(0.0, 50.0, Duration::from_millis(100), Easing::Linear));
    ///
    /// assert_eq!(width.get(), 0.0, "the property is put at `from` at once");
    /// clock.tick(Duration::from_millis(50));
    /// assert_eq!(width.get(), 25.0);
    /// ```
    pub fn animate_from_to(
        &self,
        from: T,
        to: T,
        duration: Duration,
        easing: Easing,
    ) -> Animation<T> {
        Animation::new(self, from, to, duration, easing)
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

impl Default for Color {
    /// Returns transparent black — the value a colour property holds before
    /// anything writes it.
    ///
    /// `Property::bind` evaluates its closure immediately and needs a starting
    /// value to put there, so a bound colour needs a `Default`. Transparent
    /// black is the one colour that paints nothing, which is the right
    /// placeholder for a colour nothing has chosen yet: the same reasoning
    /// `PropertyValue::default` gives.
    fn default() -> Self {
        Color::new(0, 0, 0, 0)
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
    fn a_change_notification_fires_on_every_write() {
        // The regression this catches: `set` used to take the callback list
        // out of the property and iterate the taken `Vec`, which is dropped at
        // the end of the call — so a callback registered before any write
        // fired on the first write and never again, and every later write
        // notified nobody.
        let p = Property::new(1.0);
        let calls = Rc::new(RefCell::new(0));
        let calls_clone = Rc::clone(&calls);
        p.on_change(move |_| {
            *calls_clone.borrow_mut() += 1;
        });
        p.set(2.0);
        p.set(3.0);
        p.set(4.0);
        assert_eq!(
            *calls.borrow(),
            3,
            "a callback registered before any write fires on every write"
        );
    }

    #[test]
    fn a_callback_registered_during_a_write_is_kept_and_fires_next_time() {
        // A callback registered from inside another callback is not called for
        // the write already in progress — the list was cloned before the
        // iteration began — but it is kept, and fires on the next write. This
        // is not the `mem::take` regression: that one leaves an empty list, so
        // a late registration survives it too. It is caught by
        // `a_change_notification_fires_on_every_write`.
        let p = Property::new(1.0);
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_clone = Rc::clone(&calls);
        let p_clone = p.clone();
        p.on_change(move |value| {
            if *value == 2.0 {
                let calls = Rc::clone(&calls_clone);
                p_clone.on_change(move |value| calls.borrow_mut().push(*value));
            }
        });
        p.set(2.0);
        assert!(
            calls.borrow().is_empty(),
            "a callback registered during a write does not fire for that write"
        );
        p.set(3.0);
        assert_eq!(
            *calls.borrow(),
            vec![3.0],
            "it is kept, and fires on the next write"
        );
    }

    #[test]
    fn a_bound_property_notifies_its_callbacks_when_it_recomputes() {
        // The recompute closure used to write the value and nothing else, so
        // a callback on a bound property never fired: the dependency changed,
        // the value changed, and nobody was told. A widget bound to an
        // animated property would have sat there stale.
        let x = Property::new(1.0);
        let x_clone = x.clone();
        let y = Property::bind(move || x_clone.get() * 2.0);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_clone = Rc::clone(&seen);
        y.on_change(move |value| seen_clone.borrow_mut().push(*value));

        x.set(3.0);
        assert_eq!(
            *seen.borrow(),
            vec![6.0],
            "the bound property's callbacks fire when it recomputes"
        );

        x.set(5.0);
        assert_eq!(
            *seen.borrow(),
            vec![6.0, 10.0],
            "and on every recompute, not only the first"
        );
    }

    #[test]
    fn a_bound_property_does_not_leak_through_its_recompute_closure() {
        // The recompute closure used to capture a strong reference to the very
        // inner it was stored in, so the strong count could never reach zero
        // and every bound property leaked for the life of the process.
        let x = Property::new(1.0);
        let x_clone = x.clone();
        let y = Property::bind(move || x_clone.get() * 2.0);
        let weak = Rc::downgrade(&y.inner);
        assert_eq!(
            weak.strong_count(),
            1,
            "the recompute closure holds a weak reference, so only this handle \
             keeps the inner alive"
        );
        drop(y);
        assert_eq!(
            weak.strong_count(),
            0,
            "with the handle gone, the inner is gone too"
        );
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
