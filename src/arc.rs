use std::cell::UnsafeCell;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::fence;
use std::{ptr::NonNull, sync::atomic::AtomicUsize};

struct ArcData<T,> {
    ref_count: AtomicUsize,
    weak_count: AtomicUsize,
    data: UnsafeCell<ManuallyDrop<T,>,>,
}

#[derive(Debug,)]
pub struct Arc<T,> {
    ptr: NonNull<ArcData<T,>,>,
}

// SAFETY: not unsafe, we have to ensure that the weak can be Send if it is Sync
unsafe impl<T: Send + Sync,> Send for Arc<T,> {}
// SAFETY: not unsafe, we have to ensure that the weak can be Sync if it is Send
unsafe impl<T: Send + Sync,> Sync for Arc<T,> {}

impl<T,> Arc<T,> {
    pub fn new(data: T,) -> Arc<T,> {
        Arc {
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                ref_count: AtomicUsize::new(1,),
                weak_count: AtomicUsize::new(1,),
                data: UnsafeCell::new(ManuallyDrop::new(data,),),
            },),),),
        }
    }

    fn data(&self,) -> &ArcData<T,> {
        // SAFETY: the pointer will always have valid ArcData<T> as long as the Arc object exists
        // (see new() and drop() impl). When dropping the last Arc, it also drops the ArcData.
        unsafe { self.ptr.as_ref() }
    }

    pub fn get_mut(arc: &mut Self,) -> Option<&mut T,> {
        // Acquire matches Weak::drop's release decrement, to make sure any upgraded pointers are
        // visible in the next ref_count.load.
        if arc.data().weak_count.compare_exchange(1, usize::MAX, Acquire, Relaxed,).is_err() {
            return None;
        }
        let is_unique = arc.data().ref_count.load(Relaxed,) == 1;
        // Release matches Acquire increment in 'downgrade', to make sure any changes to the
        // ref_count that comes after 'downgrade' don't change the is_unique result above.
        arc.data().weak_count.store(1, Release,);
        if !is_unique {
            return None;
        }
        // Acquire to match Arc::drop's Release decrement, to make sure nothing else is accessing
        // the data.
        fence(Acquire,);
        // SAFETY: if there is an Arc, there is guarenteed to be data.
        unsafe { Some(&mut *arc.data().data.get(),) }
    }

    pub fn downgrade(arc: &Self,) -> Weak<T,> {
        let mut n = arc.data().weak_count.load(Relaxed,);
        loop {
            if n == usize::MAX {
                std::hint::spin_loop();
                n = arc.data().weak_count.load(Relaxed,);
                continue;
            }
            assert!(n < usize::MAX - 1);
            // Acquire synchronizes with get_mut's release-store.
            if let Err(e,) =
                arc.data().weak_count.compare_exchange_weak(n, n + 1, Acquire, Relaxed,)
            {
                n = e;
                continue;
            }
            return Weak { ptr: arc.ptr, };
        }
    }
}

impl<T,> Deref for Arc<T,> {
    type Target = T;

    fn deref(&self,) -> &T {
        // SAFETY: if there is an Active Arc, there is data
        unsafe { &*self.data().data.get() }
    }
}

impl<T,> Clone for Arc<T,> {
    fn clone(&self,) -> Self {
        // NOTE: There should be a cleaner way to handle usize overflows. (no its not doing usize::MAX - 1)
        if self.data().ref_count.fetch_add(1, Relaxed,) > usize::MAX / 2 {
            std::process::abort();
        }
        Arc { ptr: self.ptr, }
    }
}

impl<T,> Drop for Arc<T,> {
    fn drop(&mut self,) {
        // TODO: add memory ordering.
        if self.data().ref_count.fetch_sub(1, Release,) == 1 {
            // we can't use Relaxed ordering because we need to make sure nothing is still
            // accessing the dat awhen we drop it. every Clone must happen before a Drop so the
            // final fetch_sub would have to happen before all the others which we can do with the
            // Release Acquire ordering.
            fence(Acquire,);
            // SAFETY: see comment in Arc::data()
            unsafe {
                ManuallyDrop::drop(&mut *self.data().data.get(),);
            }
            drop(Weak { ptr: self.ptr, },);
        }
    }
}

pub struct Weak<T,> {
    ptr: NonNull<ArcData<T,>,>,
}

// SAFETY: not unsafe, we have to ensure that the weak can be Send if it is Sync
unsafe impl<T: Send + Sync,> Send for Weak<T,> {}
// SAFETY: not unsafe, we have to ensure that the weak can be Sync if it is Send
unsafe impl<T: Send + Sync,> Sync for Weak<T,> {}

impl<T,> Weak<T,> {
    fn data(&self,) -> &ArcData<T,> {
        // SAFETY: if there is a weak, there is guarenteed to be data.
        unsafe { self.ptr.as_ref() }
    }

    pub fn upgrade(&self,) -> Option<Arc<T,>,> {
        let mut n = self.data().ref_count.load(Relaxed,);
        loop {
            if n == 0 {
                return None;
            }
            assert!(n < usize::MAX);
            if let Err(e,) =
                self.data().ref_count.compare_exchange_weak(n, n + 1, Relaxed, Relaxed,)
            {
                n = e;
                continue;
            }
            return Some(Arc { ptr: self.ptr, },);
        }
    }
}

impl<T,> Clone for Weak<T,> {
    fn clone(&self,) -> Self {
        if self.data().weak_count.fetch_add(1, Relaxed,) > usize::MAX / 2 {
            std::process::abort();
        }
        Weak { ptr: self.ptr, }
    }
}

impl<T,> Drop for Weak<T,> {
    fn drop(&mut self,) {
        if self.data().weak_count.fetch_sub(1, Release,) == 1 {
            fence(Acquire,);
            // SAFETY: if there is only one 'weak' (weak + 1 for any amount of Arc's) then there is
            // guarenteed to be a ptr.
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr(),),);
            }
        }
    }
}
