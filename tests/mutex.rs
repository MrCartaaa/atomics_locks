use std::{thread, time::Instant};

use atomics_locks::mutex::Mutex;

#[test]
fn mutex_attack() {
    const ATTACK: u32 = 5_000_000;
    let m = Mutex::new(0,);
    std::hint::black_box(&m,);
    let start = Instant::now();
    for _ in 0..ATTACK {
        *m.lock() += 1;
    }
    let duration = start.elapsed();
    println!("[linear] locked {} times in {:?}", *m.lock(), duration);

    let start = Instant::now();

    thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                for _ in 0..ATTACK {
                    *m.lock() += 1;
                }
            },);
        }
    },);
    let duration = start.elapsed();
    println!("[threaded] locked {} times in {:?}", *m.lock(), duration);
}
