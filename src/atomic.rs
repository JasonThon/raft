use std::sync::atomic;

pub struct AtomicU64 {
    _inner: atomic::AtomicU64,
}

impl AtomicU64 {
    pub fn new(value: u64) -> AtomicU64 {
        AtomicU64 {
            _inner: atomic::AtomicU64::new(value)
        }
    }

    pub fn get_and_increment(&self) -> u64 {
        let origin = self._inner.load(atomic::Ordering::Relaxed);
        self._inner.fetch_add(1, atomic::Ordering::SeqCst);

        origin
    }

    pub fn get(&self) -> u64 {
        self._inner.load(atomic::Ordering::Relaxed)
    }

    pub fn set(&self, val: u64) {
        self._inner.store(val, atomic::Ordering::SeqCst)
    }
}
