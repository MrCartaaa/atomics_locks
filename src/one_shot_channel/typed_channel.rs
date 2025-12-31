use negative_impl::negative_impl;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::{cell::UnsafeCell, mem::MaybeUninit, sync::atomic::AtomicBool, thread, thread::Thread};

pub struct Channel<T,> {
    message: UnsafeCell<MaybeUninit<T,>,>,
    ready: AtomicBool,
}

impl<T,> Channel<T,> {
    pub const fn new() -> Self {
        Self { message: UnsafeCell::new(MaybeUninit::uninit(),), ready: AtomicBool::new(false,), }
    }
    pub fn split<'a,>(&'a mut self,) -> (Sender<'a, T,>, Receiver<'a, T,>,) {
        *self = Self::new();
        (
            Sender { channel: self, receiving_thread: thread::current(), },
            Receiver { channel: self, },
        )
    }
}

impl<T,> Default for Channel<T,> {
    fn default() -> Self {
        Self { message: UnsafeCell::new(MaybeUninit::uninit(),), ready: AtomicBool::new(false,), }
    }
}

// SAFETY: if the Channel is Send we have to make sure it is Sync
unsafe impl<T,> Sync for Channel<T,> where T: Send {}

impl<T,> Drop for Channel<T,> {
    fn drop(&mut self,) {
        if *self.ready.get_mut() {
            // SAFETY: we are accessing a UnsafeCell, if self.ready, then there is data in there
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

pub struct Sender<'a, T,> {
    channel: &'a Channel<T,>,
    receiving_thread: Thread,
}

impl<T,> Sender<'_, T,> {
    pub fn send(self, message: T,) {
        // SAFETY: we are accessing a UnsafeCell and writing to it. the reciever is parked so there
        // is nothing accessing it. this is a Typed channel guarenteeing the send() can only be
        // called once + the happens-before pattern is being used with 'Release' on the AtomicBool
        unsafe { (*self.channel.message.get()).write(message,) };
        self.channel.ready.store(true, Release,);
        self.receiving_thread.unpark();
    }
}

pub struct Receiver<'a, T,> {
    channel: &'a Channel<T,>,
}

#[negative_impl]
impl<T,> !Send for Receiver<'_, T,> {}

impl<T,> Receiver<'_, T,> {
    pub fn is_ready(&self,) -> bool {
        self.channel.ready.load(Relaxed,)
    }

    pub fn receive(self,) -> T {
        if !self.channel.ready.swap(false, Acquire,) {
            thread::park();
        }
        // SAFETY: We've just checked (and reset) the ready flag.
        unsafe { (*self.channel.message.get()).assume_init_read() }
    }
}
