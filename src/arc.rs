use std::ops::Deref;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::fence;
use std::{ptr::NonNull, sync::atomic::AtomicUsize};

struct ArcData<T> {
    ref_count: AtomicUsize,
    data: T,
}

pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                ref_count: AtomicUsize::new(1),
                data,
            }))),
        }
    }

    fn data(&self) -> &ArcData<T> {
        // SAFETY: the pointer will always have valid ArcData<T> as long as the Arc object exists
        // (see new() and drop() impl). When dropping the last Arc, it also drops the ArcData.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data().data
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // NOTE: There should be a cleaner way to handle usize overflows. (no its not doing usize::MAX - 1)
        if self.data().ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        Arc { ptr: self.ptr }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        // TODO: add memory ordering.
        if self.data().ref_count.fetch_sub(1, Release) == 1 {
            // NOTE: we can't use Relaxed ordering because we need to make sure nothing is still
            // accessing the dat awhen we drop it. every Clone must happen before a Drop so the
            // final fetch_sub would have to happen before all the others which we can do with the
            // Release Acquire ordering.
            fence(Acquire);
            // SAFETY: see comment in Arc::data()
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}
