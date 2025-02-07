use std::sync::RwLock;

pub trait RwLockExt<T> {
    fn with_read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R;

    fn with_write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R;
}

impl<T> RwLockExt<T> for RwLock<T> {
    fn with_read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.read().unwrap();
        f(&guard)
    }

    fn with_write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.write().unwrap();
        f(&mut guard)
    }
}
