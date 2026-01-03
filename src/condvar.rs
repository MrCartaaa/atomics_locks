use std::sync::atomic::AtomicU32;

pub struct CondVar {
    counter: AtomicU32,
}
