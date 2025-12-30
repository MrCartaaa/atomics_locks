use atomics_locks::arc::{Arc, Weak};
use atomics_locks::spinlock::SpinLock;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;

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
    let weak1 = Arc::downgrade(&x);
    let weak2 = Arc::downgrade(&x);

    let t = std::thread::spawn(move || {
        // Weak pointer should be upgradeable
        let y = weak1.upgrade().unwrap();
        assert_eq!(y.0, "hello world");
    });

    assert_eq!(x.0, "hello world");

    t.join().unwrap();

    // the data shouldn't be dropped yet
    assert_eq!(NUM_DROPS.load(Relaxed), 0);
    assert!(weak2.upgrade().is_some());

    drop(x);

    // Now, the data should be dropped, and the weak pointer should bo longer be upgradeable.
    assert_eq!(NUM_DROPS.load(Relaxed), 1);
    assert!(weak2.upgrade().is_none());
}

#[test]
fn arc_spinlock() {
    #[derive(Clone)]
    enum Sex {
        Male,
        Female,
    }
    struct Person {
        name: &'static str,
        sex: Sex,
        children: Vec<Arc<SpinLock<Person>>>,
        mother: Option<Weak<SpinLock<Person>>>,
        father: Option<Weak<SpinLock<Person>>>,
    }

    impl Person {
        pub fn is_born(
            mother: Option<&Arc<SpinLock<Self>>>,
            father: Option<&Arc<SpinLock<Self>>>,
            name: &'static str,
            sex: Sex,
        ) -> Result<Arc<SpinLock<Self>>, &'static str> {
            match (mother, father) {
                (Some(mother), Some(father)) => {
                    let mut mother_guard = mother.lock();
                    let mut father_guard = father.lock();
                    match (&mother_guard.sex, &father_guard.sex) {
                        (&Sex::Female, &Sex::Male) => {
                            let baby = Arc::new(SpinLock::new(Person {
                                name,
                                children: Vec::new(),
                                sex,
                                mother: Some(Arc::downgrade(mother)),
                                father: Some(Arc::downgrade(father)),
                            }));
                            {
                                mother_guard.children.push(baby.clone());
                                father_guard.children.push(baby.clone());
                            }

                            Ok(baby)
                        }
                        _ => Err("it takes a man and a woman to have a baby."),
                    }
                }
                _ => Ok(Arc::new(SpinLock::new(Person {
                    name,
                    children: Vec::new(),
                    sex,
                    mother: None,
                    father: None,
                }))),
            }
        }
    }

    let madison = Person::is_born(None, None, "madison", Sex::Female).unwrap();
    let mut carter = Person::is_born(None, None, "carter", Sex::Male).unwrap();

    let babies = vec![
        ("ayla", Sex::Female),
        ("kane", Sex::Male),
        ("kali", Sex::Male),
        ("iris", Sex::Female),
        ("nora", Sex::Female),
    ];

    thread::scope(|s| {
        for baby in babies {
            s.spawn(|| {
                let baby_result = Person::is_born(Some(&madison), Some(&carter), baby.0, baby.1);
                match baby_result {
                    Ok(baby) => {
                        assert!(
                            Weak::upgrade(baby.lock().father.as_ref().unwrap())
                                .unwrap()
                                .lock()
                                .name
                                == "carter"
                        );
                        assert_eq!(
                            Weak::upgrade(baby.lock().mother.as_ref().unwrap())
                                .unwrap()
                                .lock()
                                .name,
                            "madison"
                        );
                        println!("{} was born!", baby.lock().name);
                    }
                    Err(e) => println!("{}", e),
                }
            });
        }
    });

    assert_eq!(carter.lock().children.len(), 5);

    let cloned = carter.clone();

    let carter_mut_option = Arc::get_mut(&mut carter);
    if carter_mut_option.is_some() {
        let baby_result = Person::is_born(Some(&madison), Some(&carter), "iris", Sex::Female);
        match baby_result {
            Ok(baby) => {
                println!("{} was born!", baby.lock().name);
            }
            Err(e) => println!("{}", e),
        };
    }

    assert!(carter.lock().children.len() == 5);

    drop(cloned);

    let carter_weak = Arc::downgrade(&carter);
    let madison_weak = Arc::downgrade(&madison);

    drop(carter);

    drop(madison);

    assert!(carter_weak.upgrade().is_none());
    assert!(madison_weak.upgrade().is_none());
}
