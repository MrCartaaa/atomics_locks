use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::{cell::UnsafeCell, sync::atomic::AtomicU32};

use atomic_wait::{wait, wake_all, wake_one};

pub struct RwLock<T,> {
    // NOTE: to prevent writer starvation:
    //          - The number of read locks increments by 2
    //          - The number of write locks (just one write lock at a time) increments by 1
    //          - therefore, if the state is odd, there is a writer waiting
    state: AtomicU32,
    value: UnsafeCell<T,>,
    writer_wake_count: AtomicU32,
}

impl<T: Default,> Default for RwLock<T,> {
    fn default() -> Self {
        Self::new(T::default(),)
    }
}

// SAFETY: we require Send if T implements Sync
unsafe impl<T,> Sync for RwLock<T,> where T: Send + Sync {}

impl<T,> RwLock<T,> {
    pub const fn new(value: T,) -> Self {
        Self {
            state: AtomicU32::new(0,),
            value: UnsafeCell::new(value,),
            writer_wake_count: AtomicU32::new(0,),
        }
    }
    pub fn read(&self,) -> ReadGuard<'_, T,> {
        let mut s = self.state.load(Relaxed,);
        loop {
            if s.is_multiple_of(2,) {
                assert!(s != u32::MAX - 1, "too many readers.");
                match self.state.compare_exchange_weak(s, s + 1, Acquire, Relaxed,) {
                    Ok(_,) => return ReadGuard { rwlock: self, },
                    Err(e,) => s = e,
                }
            }
            if s % 2 == 1 {
                wait(&self.state, s,);
                s = self.state.load(Relaxed,);
            }
        }
    }
    pub fn write(&self,) -> WriteGuard<'_, T,> {
        let mut s = self.state.load(Relaxed,);
        loop {
            if s <= 1 {
                match self.state.compare_exchange(s, u32::MAX, Acquire, Relaxed,) {
                    Ok(_,) => return WriteGuard { rwlock: self, },
                    Err(e,) => {
                        s = e;
                        continue;
                    }
                }
            }
            if s.is_multiple_of(2,) {
                match self.state.compare_exchange(s, s + 1, Relaxed, Relaxed,) {
                    Ok(_,) => {}
                    Err(e,) => {
                        s = e;
                        continue;
                    }
                }
            }
            let w = self.writer_wake_count.load(Acquire,);
            s = self.state.load(Relaxed,);
            if s <= 2 {
                wait(&self.writer_wake_count, w,);
                s = self.state.load(Relaxed,);
            }
        }
    }
}

pub struct ReadGuard<'a, T,> {
    rwlock: &'a RwLock<T,>,
}

impl<T,> Deref for ReadGuard<'_, T,> {
    type Target = T;
    fn deref(&self,) -> &T {
        // SAFETY: accessing an UnsafeCell, it is save, given the Guard
        unsafe { &*self.rwlock.value.get() }
    }
}

impl<T,> Drop for ReadGuard<'_, T,> {
    fn drop(&mut self,) {
        if self.rwlock.state.fetch_sub(2, Release,) == 3 {
            self.rwlock.writer_wake_count.fetch_add(1, Release,);
            wake_one(&self.rwlock.writer_wake_count,);
        }
    }
}

pub struct WriteGuard<'a, T,> {
    rwlock: &'a RwLock<T,>,
}

impl<T,> Deref for WriteGuard<'_, T,> {
    type Target = T;
    fn deref(&self,) -> &T {
        // SAFETY: accessing an UnsafeCell, it is save, given the Guard
        unsafe { &*self.rwlock.value.get() }
    }
}

impl<T,> DerefMut for WriteGuard<'_, T,> {
    fn deref_mut(&mut self,) -> &mut T {
        // SAFETY: see safety comment for Deref impl
        unsafe { &mut *self.rwlock.value.get() }
    }
}

impl<T,> Drop for WriteGuard<'_, T,> {
    fn drop(&mut self,) {
        self.rwlock.state.store(0, Release,);
        self.rwlock.writer_wake_count.fetch_add(1, Release,);
        wake_one(&self.rwlock.writer_wake_count,);
        wake_all(&self.rwlock.state,);
    }
}
