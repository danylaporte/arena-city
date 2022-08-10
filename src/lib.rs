//! ArenaCity is an arena container which create Citizen that once drop, returns automatically to the Arena.
//! This is useful to reduce allocation of same type of items.
//!
//! The Arena is safe for multi-thread context and a lock is applied only when seeking for an existing
//! Citizen or returning a Citizen to the arena.
//!
//! ```
//! use arena_city::ArenaCity;
//!
//! let city = ArenaCity::new();
//!
//! let foo = city.get_or_create(|| "Foo");
//! assert_eq!(*foo, "Foo");
//!
//! drop(foo);
//!
//! let bar = city.get_or_create(|| "Bar");
//! assert_eq!(*bar, "Foo"); // returns back the dropped Citizen.
//! ```

use parking_lot::Mutex;
use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

pub struct ArenaCity<T>(Mutex<Vec<T>>);

impl<T> ArenaCity<T> {
    pub const fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Mutex::new(Vec::with_capacity(capacity)))
    }

    pub fn clear(&self) {
        self.reduce_to(0);
    }

    pub fn clear_mut(&mut self) {
        self.reduce_to_mut(0)
    }

    pub const fn create(&self, value: T) -> Citizen<T> {
        Citizen {
            city: Some(self),
            value: ManuallyDrop::new(value),
        }
    }

    pub fn get_or_create<F>(&self, init: F) -> Citizen<T>
    where
        F: FnOnce() -> T,
    {
        let value = self.pop().unwrap_or_else(init);
        self.create(value)
    }

    fn pop(&self) -> Option<T> {
        self.0.lock().pop()
    }

    pub fn reduce_to(&self, new_size: usize) {
        reduce_to(&mut self.0.lock(), new_size);
    }

    pub fn reduce_to_mut(&mut self, new_size: usize) {
        reduce_to(self.0.get_mut(), new_size);
    }
}

impl<T> Default for ArenaCity<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Citizen<'a, T> {
    city: Option<&'a ArenaCity<T>>,
    value: ManuallyDrop<T>,
}

impl<'a, T> Citizen<'a, T> {
    pub fn into_inner(mut self) -> T {
        self.take().expect("value").1
    }

    fn take(&mut self) -> Option<(&'a ArenaCity<T>, T)> {
        let city = self.city.take()?;
        Some((city, unsafe { ManuallyDrop::take(&mut self.value) }))
    }
}

impl<'a, T> Deref for Citizen<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T> DerefMut for Citizen<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<'a, T> Drop for Citizen<'a, T> {
    fn drop(&mut self) {
        if let Some((city, value)) = self.take() {
            city.0.lock().push(value);
        }
    }
}

fn reduce_to<T>(vec: &mut Vec<T>, new_size: usize) {
    if vec.len() > new_size {
        vec.drain(new_size..);
    }
}
