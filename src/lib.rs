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
//! let mut foo = city.get_or_create(|| Vec::new());
//! foo.push(10);
//!
//! // invoke the CitizenDrop trait on the value to make sure it is sanitize.
//! drop(foo);
//!
//! let foo = city.get_or_create(|| unreachable!("it will reuse foo"));
//! assert_eq!(foo.len(), 0); // returns the dropped citizen from the arena, does not create it.
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

    pub const fn create(&self, value: T) -> Citizen<T>
    where
        T: Sanitize,
    {
        Citizen {
            city: Some(self),
            value: ManuallyDrop::new(value),
        }
    }

    pub fn get_or_create<F>(&self, init: F) -> Citizen<T>
    where
        F: FnOnce() -> T,
        T: Sanitize,
    {
        let value = self.pop().unwrap_or_else(init);
        self.create(value)
    }

    pub fn get_or_default(&self) -> Citizen<T>
    where
        T: Default + Sanitize,
    {
        self.get_or_create(T::default)
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

pub struct Citizen<'a, T: Sanitize> {
    city: Option<&'a ArenaCity<T>>,
    value: ManuallyDrop<T>,
}

impl<'a, T: Sanitize> Citizen<'a, T> {
    pub fn into_inner(mut self) -> T {
        self.take().expect("value").1
    }

    fn take(&mut self) -> Option<(&'a ArenaCity<T>, T)> {
        let city = self.city.take()?;
        Some((city, unsafe { ManuallyDrop::take(&mut self.value) }))
    }
}

impl<'a, T: Sanitize> Deref for Citizen<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, T: Sanitize> DerefMut for Citizen<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<'a, T> Drop for Citizen<'a, T>
where
    T: Sanitize,
{
    fn drop(&mut self) {
        if let Some((city, value)) = self.take() {
            if let Some(value) = value.sanitize() {
                city.0.lock().push(value);
            }
        }
    }
}

fn reduce_to<T>(vec: &mut Vec<T>, new_size: usize) {
    if vec.len() > new_size {
        vec.drain(new_size..);
    }
}

/// Clean the object before putting it back into the Arena.
pub trait Sanitize: Sized {
    fn sanitize(self) -> Option<Self> {
        Some(self)
    }
}

impl<T> Sanitize for Option<T>
where
    T: Sanitize,
{
    fn sanitize(self) -> Option<Self> {
        match self {
            Some(v) => v.sanitize().map(Some),
            None => None,
        }
    }
}

macro_rules! sanitize {
    (clear impl < $($a:ident),* > $t:ty) => {
        impl <$($a),*> Sanitize for $t {
            fn sanitize(mut self) -> Option<Self> {
                self.clear();
                Some(self)
            }
        }
    };

    (($($a:ident: $t:tt),+)) => {
        impl<$($a),+> Sanitize for ($($a,)+)
        where
            $($a: Sanitize,)+
        {
            fn sanitize(self) -> Option<Self> {
                Some(($(self.$t.sanitize()?,)+))
            }
        }
    };
}

sanitize!(clear impl<> String);
sanitize!(clear impl<K, V, S> std::collections::HashMap<K, V, S>);
sanitize!(clear impl<K, V> std::collections::BTreeMap<K, V>);
sanitize!(clear impl<T, S> std::collections::HashSet<T, S>);
sanitize!(clear impl<T> Vec<T>);
sanitize!(clear impl<T> std::collections::BTreeSet<T>);
sanitize!(clear impl<T> std::collections::LinkedList<T>);
sanitize!(clear impl<T> std::collections::VecDeque<T>);

sanitize!((A:0));
sanitize!((A:0, B:1));
sanitize!((A:0, B:1, C:2));
sanitize!((A:0, B:1, C:2, D:3));
sanitize!((A:0, B:1, C:2, D:3, E:4));
sanitize!((A:0, B:1, C:2, D:3, E:4, F:5));
