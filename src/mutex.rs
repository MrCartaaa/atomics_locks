use atomic_wait::{wait, wake_one};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::{cell::UnsafeCell, sync::atomic::AtomicU32};

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;
const LOCKED_WAITING: u32 = 2;

pub struct Mutex<T,> {
    state: AtomicU32,
    value: UnsafeCell<T,>,
}

// SAFETY: if Mutex is Send it has to be Sync
unsafe impl<T,> Sync for Mutex<T,> where T: Send {}

impl<T,> Mutex<T,> {
    pub const fn new(value: T,) -> Self {
        Self { state: AtomicU32::new(UNLOCKED,), value: UnsafeCell::new(value,), }
    }
    #[inline]
    pub fn lock(&self,) -> MutexGuard<'_, T,> {
        if self.state.compare_exchange(UNLOCKED, LOCKED, Acquire, Relaxed,).is_err() {
            lock_contended(&self.state,);
        }
        MutexGuard { mutex: self, }
    }
}

#[cold]
fn lock_contended(state: &AtomicU32,) {
    let mut spin_count = 0;

    while state.load(Relaxed,) == LOCKED && spin_count < 100 {
        spin_count += 1;
        std::hint::spin_loop();
    }

    if state.compare_exchange(UNLOCKED, LOCKED, Acquire, Relaxed,).is_ok() {
        return;
    }

    while state.swap(LOCKED_WAITING, Acquire,) != UNLOCKED {
        wait(state, LOCKED_WAITING,);
    }
}

pub struct MutexGuard<'a, T,> {
    mutex: &'a Mutex<T,>,
}

impl<T,> Deref for MutexGuard<'_, T,> {
    type Target = T;
    fn deref(&self,) -> &T {
        // SAFETY: if the mutex exists, the UnsafeCell will exists (see Drop impl)
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T,> DerefMut for MutexGuard<'_, T,> {
    fn deref_mut(&mut self,) -> &mut T {
        // SAFETY: if the mutex exists, the UnsafeCell exists.
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T,> Drop for MutexGuard<'_, T,> {
    fn drop(&mut self,) {
        if self.mutex.state.swap(UNLOCKED, Release,) == LOCKED_WAITING {
            wake_one(&self.mutex.state,);
        }
    }
}
