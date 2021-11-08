use core::any::type_name;
use core::marker::PhantomData;
use core::ptr::NonNull;

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

    pub fn into_inner(this: Self) -> T {
        *Self::into_box(this)
    }
}

impl<T: ?Sized> R<T, Full> {
    pub fn from_box(value: Box<T>) -> Self {
        let ptr = Box::into_raw(value);

        // SAFETY:
        //  * The `ptr` comes from Box.
        //  * The `ptr` is unique, and ownership is `Full`.
        unsafe { Self::from_raw(ptr) }
    }

    pub fn into_box(this: Self) -> Box<T> {
        let ptr = Self::leak(this);

        // SAFETY:
        //  * The `ptr` comes from Box
        //  * The `ptr` is unique because ownership is `Full`
        unsafe { Box::from_raw(ptr) }
    }

    pub fn as_mut(this: &mut Self) -> &mut T {
        // SAFETY:
        //  * mut access is only possible when `this: R<T, Full>`, and we have `&mut self`
        unsafe { this.ptr.as_mut() }
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

    pub fn leak(this: Self) -> *mut T {
        let ptr = R::as_ptr(&this);
        core::mem::forget(this);
        ptr
    }

    fn as_ptr(this: &Self) -> *mut T {
        this.ptr.as_ptr()
    }

    pub fn ptr_eq<P: Ownership>(this: &Self, other: &R<T, P>) -> bool {
        R::as_ptr(this) == R::as_ptr(other)
    }

    pub fn as_ref(this: &Self) -> &T {
        // SAFETY:
        //  * mut access is only possible when `this: R<T, Full>`, and we have `&self`
        unsafe { this.ptr.as_ref() }
    }

    pub fn split(this: Self) -> (R<T, O::Split>, R<T, O::Split>)
    where
        O: CanSplit,
    {
        let ptr = R::leak(this);

        // SAFETY:
        //  * `ptr` comes from `self` which already satisfied requirements.
        //  * The ownership (`O`) is correct.
        unsafe { (R::from_raw(ptr), R::from_raw(ptr)) }
    }

    pub fn join<P>(this: Self, other: R<T, P>) -> R<T, P::Joined>
    where
        P: JoinsWith<O>,
    {
        let ptr = R::leak(this);

        assert!(
            ptr == R::leak(other),
            "Cannot join pointers to different `{}`",
            type_name::<T>(),
        );

        // SAFETY:
        //  * `ptr` comes from `self` which already satisfied requirements.
        //  * The ownership (`O::Joined`) in the return type is correct.
        unsafe { R::from_raw(ptr) }
    }
}

impl<T: ?Sized, O: Ownership> Drop for R<T, O> {
    fn drop(&mut self) {
        if O::IS_FULL {
            let ptr = R::as_ptr(self);

            // SAFETY:
            //  * The `ptr` comes from Box
            //  * The `ptr` is unique because ownership (`O`) is `Full`
            let value = unsafe { Box::from_raw(ptr) };

            drop(value);
        } else {
            debug_assert!(
                false,
                "Dropping `R<_, {}>` here would leak a `{}` because it does not have `Full` ownership.",
                type_name::<O>(),
                type_name::<T>(),
            );
        }
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

impl<T: ?Sized, O: Ownership> core::convert::AsRef<T> for R<T, O> {
    fn as_ref(&self) -> &T {
        R::as_ref(self)
    }
}

impl<T: ?Sized, O: Ownership> core::borrow::Borrow<T> for R<T, O> {
    fn borrow(&self) -> &T {
        R::as_ref(self)
    }
}

impl<T: ?Sized + core::fmt::Debug, O: Ownership> core::fmt::Debug for R<T, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        R::as_ref(self).fmt(f)
    }
}

impl<T: ?Sized + core::fmt::Display, O: Ownership> core::fmt::Display for R<T, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        R::as_ref(self).fmt(f)
    }
}

impl<T: ?Sized + core::default::Default> core::default::Default for R<T, Full> {
    fn default() -> Self {
        R::new(core::default::Default::default())
    }
}

impl<T: ?Sized + Eq, O: Ownership> PartialEq for R<T, O> {
    fn eq(&self, other: &Self) -> bool {
        R::ptr_eq(self, other) && R::as_ref(self).eq(R::as_ref(other))
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
