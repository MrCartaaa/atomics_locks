use atomics_locks::arc::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

#[test]
fn arc() {
    static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DetectDrop;

    impl Drop for DetectDrop {
        fn drop(&mut self) {
            NUM_DROPS.fetch_add(1, Relaxed);
        }
    }

    // Create two Arcs shares an object containing a string and a DetectDrop, to detect when it's
    // dropped.
    let x = Arc::new(("hello world", DetectDrop));
    let y = x.clone();

    let t = std::thread::spawn(move || {
        assert_eq!(x.0, "hello world");
    });

    assert_eq!(y.0, "hello world");

    t.join().unwrap();

    assert_eq!(NUM_DROPS.load(Relaxed), 0);

    drop(y);

    assert_eq!(NUM_DROPS.load(Relaxed), 1);
}
