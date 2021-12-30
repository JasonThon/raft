use std::borrow::Borrow;
use std::sync::atomic;

pub struct Atomic<T> {
    _inner: atomic::AtomicPtr<T>,
}

impl<T> Atomic<T> {
    pub fn set(&self, mut val: T) {
        self._inner.store(&mut val, atomic::Ordering::SeqCst);
    }

    pub fn get(&self) -> Option<T> {
        Option::Some(unsafe { self._inner.load(atomic::Ordering::Relaxed).read() })
    }

    pub fn new(mut val: T) -> Atomic<T> {
        Atomic {
            _inner: atomic::AtomicPtr::<T>::new(&mut val)
        }
    }
}

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

    pub fn increment_and_get(&self) -> u64 {
        self._inner.fetch_add(1, atomic::Ordering::SeqCst)
    }

    pub fn get(&self) -> u64 {
        self._inner.load(atomic::Ordering::Relaxed)
    }

    pub fn set(&self, val: u64) {
        self._inner.store(val, atomic::Ordering::SeqCst)
    }
}
