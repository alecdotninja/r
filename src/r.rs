use core::marker::PhantomData;
use core::ptr::NonNull;
use core::fmt;

use crate::ownership::{CanSplit, Full, JoinsWith, Ownership};

#[repr(transparent)]
pub struct R<T: ?Sized, O: Ownership = Full> {
    ptr: NonNull<T>,
    _ownership: PhantomData<O>,
}

impl<T> R<T, Full> {
    pub fn new(value: T) -> Self {
        Self::from_box(Box::new(value))
    }

    pub fn into_inner(r: Self) -> T {
        *Self::into_box(r)
    }
}

impl<T: ?Sized> R<T, Full> {
    pub fn from_box(value: Box<T>) -> Self {
        let ptr = Box::into_raw(value);

        // SAFETY:
        //  * The `ptr` comes from `Box`.
        //  * The `ptr` is unique, and ownership is `Full`.
        unsafe { Self::from_raw(ptr) }
    }

    pub fn into_box(r: Self) -> Box<T> {
        let ptr = Self::leak(r);

        // SAFETY:
        //  * The `ptr` comes from Box
        //  * The `ptr` is unique because ownership is `Full`
        unsafe { Box::from_raw(ptr) }
    }

    pub fn as_mut(r: &mut Self) -> &mut T {
        // SAFETY:
        //  * `r.ptr` is unique because ownership is `Full`.
        //  * `r` will be borrowed for the lifetime of the mut reference. 
        unsafe { r.ptr.as_mut() }
    }
}

impl<T: ?Sized, O: Ownership> R<T, O> {
    // SAFETY:
    //  * The `ptr` must come from Box.
    //  * Ownership (`O`) must be correct.
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        // SAFETY:
        //  * `ptr` is from Box which cannot be null.
        let ptr = unsafe { NonNull::new_unchecked(ptr) };

        R {
            ptr,
            _ownership: PhantomData,
        }
    }

    pub fn leak(r: Self) -> *mut T {
        let ptr = R::as_ptr(&r);
        core::mem::forget(r);
        ptr
    }

    pub fn as_ptr(r: &Self) -> *mut T {
        r.ptr.as_ptr()
    }

    pub fn ptr_eq<P: Ownership>(r: &Self, other: &R<T, P>) -> bool {
        R::as_ptr(r) == R::as_ptr(other)
    }

    pub fn as_ref(r: &Self) -> &T {
        // SAFETY:
        //  * If ownership is Full, then the *only* way to get a mut ref to the
        //      underlying data is via `r`, but that will not be possible since
        //      this method borrows `r` for the lifetime of ref to the data. On
        //      the other hand, if ownership is *not* Full, then there is no
        //      way to get a mut ref to the underlying data. In either case,
        //      there is no way to get a mut ref to the data while this ref is
        //      valid.
        unsafe { r.ptr.as_ref() }
    }

    pub fn split(r: Self) -> (R<T, O::Split>, R<T, O::Split>)
    where
        O: CanSplit,
    {
        let ptr = R::leak(r);

        // SAFETY:
        //  * `ptr` comes from `self` which already satisfied requirements.
        //  * The ownership (`O`) is correct.
        unsafe { (R::from_raw(ptr), R::from_raw(ptr)) }
    }

    pub fn join<P>(r: Self, other: R<T, P>) -> R<T, P::Joined>
    where
        P: JoinsWith<O>,
    {
        let ptr = R::leak(r);

        assert!(
            ptr == R::leak(other),
            "Cannot join pointers to different values",
        );

        // SAFETY:
        //  * `ptr` comes from `self` which already satisfied requirements.
        //  * The ownership (`O::Joined`) in the return type is correct.
        unsafe { R::from_raw(ptr) }
    }
}

impl<T: ?Sized, O: Ownership> Drop for R<T, O> {
    fn drop(&mut self) {
        if !O::IS_FULL {
            return;
        }

        let ptr = R::as_ptr(self);

        // SAFETY:
        //  * The `ptr` comes from Box
        //  * The `ptr` is unique because ownership (`O`) is `Full`
        let value = unsafe { Box::from_raw(ptr) };

        drop(value);
    }
}

impl<T: ?Sized, O: Ownership> core::ops::Deref for R<T, O> {
    type Target = T;

    fn deref(&self) -> &T {
        R::as_ref(self)
    }
}

impl<T: ?Sized> core::ops::DerefMut for R<T, Full> {
    fn deref_mut(&mut self) -> &mut T {
        R::as_mut(self)
    }
}

impl<T: ?Sized, O: Ownership> AsRef<T> for R<T, O> {
    fn as_ref(&self) -> &T {
        R::as_ref(self)
    }
}

impl<T: ?Sized + fmt::Debug, O: Ownership> fmt::Debug for R<T, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + fmt::Display, O: Ownership> fmt::Display for R<T, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + Default> Default for R<T, Full> {
    fn default() -> Self {
        R::new(Default::default())
    }
}

impl<T: ?Sized + Eq, O: Ownership> PartialEq for R<T, O> {
    fn eq(&self, other: &Self) -> bool {
        R::ptr_eq(self, other) && PartialEq::eq(&**self, &**other)
    }
}

impl<T: ?Sized + Eq, O: Ownership> Eq for R<T, O> {}

impl<T> From<T> for R<T, Full> {
    fn from(value: T) -> R<T, Full> {
        R::new(value)
    }
}

impl<T: ?Sized> From<Box<T>> for R<T, Full> {
    fn from(value: Box<T>) -> R<T, Full> {
        R::from_box(value)
    }
}

unsafe impl<T: ?Sized + Sync + Send, O: Ownership> Sync for R<T, O> {}
unsafe impl<T: ?Sized + Sync + Send, O: Ownership> Send for R<T, O> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_works() {
        let v = R::new(7);

        assert_eq!(*v, 7);

        let (x, y) = R::split(v);

        assert_eq!(*x, 7);
        assert_eq!(*y, 7);

        let v = R::join(x, y);

        assert_eq!(*v, 7);
    }

    #[test]
    fn join_in_any_oreder() {
        let full = R::new(7);

        let (half_1, half_2) = R::split(full);
        let (quarter_1, quarter_2) = R::split(half_2);

        let three_quarters = R::join(half_1, quarter_1);
        let full = R::join(three_quarters, quarter_2);

        assert_eq!(R::into_inner(full), 7);
    }

    #[test]
    fn share_with_thread() {
        use std::sync::Mutex;

        let mutex = Mutex::new(0);
        let v = R::new(mutex);

        let (x, y) = R::split(v);

        let a = std::thread::spawn(move || {
            *x.lock().unwrap() += 1;
            x
        });

        let b = std::thread::spawn(move || {
            *y.lock().unwrap() += 1;
            y
        });

        let x = a.join().unwrap();
        let y = b.join().unwrap();

        let v = R::join(x, y);
        let mutex = R::into_inner(v);

        assert_eq!(mutex.into_inner().unwrap(), 2);
    }
}
