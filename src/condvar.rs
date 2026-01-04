use crate::mutex::MutexGuard;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicU32, AtomicUsize};

use atomic_wait::{wait, wake_all, wake_one};

pub struct CondVar {
    counter: AtomicU32,
    waiters_count: AtomicUsize,
}

impl Default for CondVar {
    fn default() -> Self {
        Self::new()
    }
}

impl CondVar {
    pub fn new() -> Self {
        Self { counter: AtomicU32::new(0,), waiters_count: AtomicUsize::new(0,), }
    }

    pub fn notify_one(&self,) {
        if self.waiters_count.load(Relaxed,) > 0 {
            self.counter.fetch_add(1, Relaxed,);
            wake_one(&self.counter,);
        }
    }
    pub fn notify_all(&self,) {
        if self.waiters_count.load(Relaxed,) > 0 {
            self.counter.fetch_add(1, Relaxed,);
            wake_all(&self.counter,);
        }
    }

    pub fn wait<'a, T,>(&self, guard: MutexGuard<'a, T,>,) -> MutexGuard<'a, T,> {
        self.waiters_count.fetch_add(1, Relaxed,);
        let v = self.counter.load(Relaxed,);

        let m = guard.mutex;

        drop(guard,);

        wait(&self.counter, v,);

        self.waiters_count.fetch_sub(1, Relaxed,);

        m.lock()
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;
    use crate::mutex::Mutex;

    #[test]
    fn test_condvar() {
        let c = CondVar::new();
        let m = Mutex::new(0,);

        let mut wakeups = 0;

        thread::scope(|s| {
            s.spawn(|| {
                thread::sleep(Duration::from_secs(1,),);
                *m.lock() = 123;
                c.notify_one();
            },);

            let mut mm = m.lock();
            while *mm < 100 {
                mm = c.wait(mm,);
                wakeups += 1;
            }
            assert_eq!(*mm, 123);
        },);

        assert!(wakeups < 10);
    }
}
