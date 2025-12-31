use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::{cell::UnsafeCell, mem::MaybeUninit, sync::atomic::AtomicU8};

const EMPTY: u8 = 0;
const WRITING: u8 = 1;
const READY: u8 = 2;
const READING: u8 = 3;

pub struct Channel<T,> {
    message: UnsafeCell<MaybeUninit<T,>,>,
    state: AtomicU8,
}

// SAFETY: if T is Send, we have to ensure the Channel<T> is Sync
unsafe impl<T: Send,> Sync for Channel<T,> {}

impl<T,> Default for Channel<T,> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T,> Channel<T,> {
    pub const fn new() -> Self {
        Self { message: UnsafeCell::new(MaybeUninit::uninit(),), state: AtomicU8::new(EMPTY,), }
    }

    pub fn send(&self, message: T,) {
        if self.state.compare_exchange(EMPTY, WRITING, Relaxed, Relaxed,).is_err() {
            panic!("can't send more than one message!");
        }
        // SAFETY: we're accessing an UnsafeCell and writing to it. It is guarenteed to be safe to
        // write to givent the self.state = EMPTY
        unsafe { (*self.message.get()).write(message,) };
        self.state.store(READY, Release,);
    }

    pub fn is_ready(&self,) -> bool {
        self.state.load(Relaxed,) == READY
    }

    pub fn receive(&self,) -> T {
        if self.state.compare_exchange(READY, READING, Acquire, Relaxed,).is_err() {
            panic!("no message available!");
        }
        // SAFETY: we're accessing a UnsafeCell and reading it (there is something there if
        // status == READY)
        unsafe { (*self.message.get()).assume_init_read() }
    }
}

impl<T,> Drop for Channel<T,> {
    fn drop(&mut self,) {
        if *self.state.get_mut() == READY {
            // SAFETY: we're accessing a UnsafeCell and dropping it (there is something there if
            // status == READY)
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}
