//! Widgets.
//!
//! Owns the concrete widgets, one module each, listed in
//! `doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Module Layout*.
//!
//! It owns what more than one widget shares as well. There is one such thing
//! so far — [`Callback`] — which was [`button::Callback`] until a second widget
//! needed it to carry a value. They share this module rather than getting one
//! each: a module per shared type would put a type in a file named after
//! itself, and there are not enough of them to be worth the module tree.
//! `doc/ui/IMPLEMENTATION_STATE.md` § *Task 14 — what it decided* says what
//! would move them out again.

use std::rc::Rc;

/// A callback a widget invokes, such as a button's click handler or a slider's
/// change notification.
///
/// A newtype rather than a bare `Rc<dyn Fn(T)>`, because a bare trait object is
/// not nameable in a struct field or a return type, and this type is one of the
/// fields a caller writes. `T` is what the widget hands the handler: nothing at
/// all for a click, which is [`button::Callback`]'s `()`, and the new value for
/// a [`slider`](crate::widgets::slider::Slider)'s change.
///
/// It is deliberately *not* the `Callback<T>` of
/// [`property`](crate::property): that one is private, it is `Fn(&T)`, and it is
/// the notification a property fires on every write, which is a different job
/// from an action a widget performs.
///
/// An unset callback is not an error and not a panic: a widget whose caller has
/// given it nothing to do is an ordinary widget to put on screen.
///
/// The two constructors are named apart, and deliberately. A closure that takes
/// no arguments is not one that takes `()`, and Rust will not quietly make it
/// one — so a single `new` could not serve both shapes. [`new`](Callback::new) is
/// the payload-free one every button's click handler is written with, and
/// [`from_fn`](Callback::from_fn) is the general one.
///
/// # Examples
///
/// ```
/// use std::cell::Cell;
/// use std::rc::Rc;
/// use ui_core::widgets::Callback;
///
/// let clicks = Rc::new(Cell::new(0));
/// let counted = Rc::clone(&clicks);
/// let on_click: Callback<()> = Callback::new(move || counted.set(counted.get() + 1));
/// assert!(on_click.is_set());
/// on_click.call(());
/// assert_eq!(clicks.get(), 1);
///
/// // And a callback that carries the value a widget is reporting.
/// let changes = Rc::new(Cell::new(0.0_f32));
/// let seen = Rc::clone(&changes);
/// let on_change = Callback::from_fn(move |value: f32| seen.set(value));
/// on_change.call(0.25);
/// assert_eq!(changes.get(), 0.25);
///
/// // A widget with no handler is not a failure.
/// let none: Callback<f32> = Callback::none();
/// assert!(!none.is_set());
/// none.call(1.0);
/// ```
pub struct Callback<T: 'static>(Option<Rc<dyn Fn(T)>>);

impl<T: 'static> Callback<T> {
    /// Returns a callback that does nothing, which is what a widget holds until
    /// a caller gives it one.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::widgets::Callback;
    ///
    /// let callback: Callback<f32> = Callback::none();
    /// assert!(!callback.is_set());
    /// ```
    #[must_use]
    pub fn none() -> Self {
        Callback(None)
    }

    /// Returns a callback that runs `f`, with the value the widget is reporting,
    /// when it is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    /// use std::rc::Rc;
    /// use ui_core::widgets::Callback;
    ///
    /// let total = Rc::new(Cell::new(0.0_f32));
    /// let summed = Rc::clone(&total);
    /// let callback: Callback<f32> =
    ///     Callback::from_fn(move |value| summed.set(summed.get() + value));
    ///
    /// callback.call(1.5);
    /// callback.call(2.0);
    /// assert_eq!(total.get(), 3.5);
    /// ```
    #[must_use]
    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(T) + 'static,
    {
        Callback(Some(Rc::new(f)))
    }

    /// Returns whether a callback is set.
    #[must_use]
    pub fn is_set(&self) -> bool {
        self.0.is_some()
    }

    /// Runs the callback with `value`, if one is set.
    pub fn call(&self, value: T) {
        if let Some(callback) = &self.0 {
            callback(value);
        }
    }
}

impl Callback<()> {
    /// Returns a callback that runs `f`, which takes nothing, when it is called.
    ///
    /// This is the constructor a payload-free widget's handler is written with —
    /// a click carries no value, and the handler closes over whatever it needs: a
    /// counter, a [`Property`](crate::property::Property), a `Weak` to something
    /// it owns. It is separate from
    /// [`from_fn`](Callback::from_fn) rather than the same function, because a
    /// closure of no arguments does not implement `Fn(())` and no bound can make
    /// it; naming them apart is what lets `Callback::new(move || …)` keep meaning
    /// what it meant to [`button::Callback`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    /// use std::rc::Rc;
    /// use ui_core::widgets::Callback;
    ///
    /// let clicks = Rc::new(Cell::new(0));
    /// let counted = Rc::clone(&clicks);
    /// let callback: Callback<()> = Callback::new(move || counted.set(counted.get() + 1));
    ///
    /// callback.call(());
    /// callback.call(());
    /// assert_eq!(clicks.get(), 2);
    /// ```
    #[must_use]
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() + 'static,
    {
        Callback(Some(Rc::new(move |()| f())))
    }
}

/// Cloning a callback shares the handler rather than copying it, and needs no
/// bound on `T`: what is cloned is the `Rc`, and the `T` it carries is consumed
/// by the call that runs it.
impl<T: 'static> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Callback(self.0.clone())
    }
}

impl<T: 'static> Default for Callback<T> {
    fn default() -> Self {
        Callback::none()
    }
}

pub mod button;
pub mod container;
pub mod gauge;
pub mod image;
pub mod keyboard;
pub mod label;
pub mod list;
pub mod progress;
pub mod scroll;
pub mod slider;
pub mod text_input;
pub mod toggle;
