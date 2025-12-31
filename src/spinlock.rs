use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering::{Acquire, Release};
use std::{cell::UnsafeCell, sync::atomic::AtomicBool};

pub struct SpinLock<T,> {
    locked: AtomicBool,
    value: UnsafeCell<T,>,
}

// SAFETY: if the spinlock is Send, we have to make sure it is sync
unsafe impl<T,> Sync for SpinLock<T,> where T: Send {}

impl<T,> SpinLock<T,> {
    // NOTE: pub functions are protected by the Guard

    pub const fn new(value: T,) -> Self {
        Self { locked: AtomicBool::new(false,), value: UnsafeCell::new(value,), }
    }

    pub fn lock(&self,) -> Guard<'_, T,> {
        while self.locked.swap(true, Acquire,) {
            std::hint::spin_loop();
        }
        Guard::new(self,)
    }

    pub fn unlock(&self,) {
        self.locked.store(false, Release,);
    }
}

pub struct Guard<'a, T,> {
    lock: &'a SpinLock<T,>,
}

impl<'a, T,> Guard<'a, T,> {
    pub const fn new(lock: &'a SpinLock<T,>,) -> Self {
        Guard { lock, }
    }
}

impl<T,> Deref for Guard<'_, T,> {
    type Target = T;
    fn deref(&self,) -> &T {
        // SAFETY: the very existence of the guard guarentees the lock is exclusively locked.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T,> DerefMut for Guard<'_, T,> {
    fn deref_mut(&mut self,) -> &mut T {
        // SAFETY: the very existence of the guard guarentees the lock is exclusively locked.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T,> Drop for Guard<'_, T,> {
    fn drop(&mut self,) {
        self.lock.unlock();
    }
}
