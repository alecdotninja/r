use core::marker::PhantomData;
use core::ptr::NonNull;
use std::any::type_name;

use crate::ownership::*;

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

impl <T: ?Sized> R<T, Full> {
    pub fn from_box(value: Box<T>) -> Self {
        let ptr = Box::into_raw(value);

        // SAFETY:
        //  * The `ptr` comes from Box.
        //  * The `ptr` is unique, and ownership is `Full`.
        unsafe {
            Self::from_raw(ptr)
        }
    }

    pub fn into_box(this: Self) -> Box<T> {
        let ptr = Self::leak(this);

        // SAFETY:
        //  * The `ptr` comes from Box
        //  * The `ptr` is unique because ownership is `Full`
        unsafe { Box::from_raw(ptr) }
    }
}

impl <T: ?Sized, O: Ownership> R<T, O> {
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
        let ptr = R::ptr(&this);
        core::mem::forget(this);
        ptr
    }

    pub fn split(this: Self) -> (R<T, O::Split>, R<T, O::Split>) {
        let ptr = R::leak(this);

        // SAFETY:
        //  * `ptr` comes from `self` which already satisfied requirements.
        //  * The ownership (`O`) is correct.
        unsafe {
            (
                R::from_raw(ptr),
                R::from_raw(ptr),
            )
        }
    }
}

impl <T: ?Sized, N: Ownership> R<T, N> {
    fn ptr(this: &Self) -> *mut T {
        this.ptr.as_ptr()
    }

    pub fn ptr_eq<O: Ownership>(this: &Self, other: &R<T, O>) -> bool {
        R::ptr(this) == R::ptr(other)
    }

    pub fn as_ref(this: &Self) -> &T {
        // SAFETY:
        //  * mut access is only possible when `this: R<T, Full>`, and we have `&self`
        unsafe { this.ptr.as_ref() }
    }

    pub fn as_mut(this: &mut Self) -> &mut T {
        // SAFETY:
        //  * mut access is only possible when `this: R<T, Full>`, and we have `&mut self`
        unsafe { this.ptr.as_mut() }
    }

    pub fn join<O: JoinsWith<N>>(this: Self, other: R<T, O>) -> R<T, O::Joined> {
        let ptr = R::leak(this);

        assert!(
            ptr == R::leak(other),
            "Cannot join pointers to different `{}`",
            type_name::<T>(),
        );

        // SAFETY:
        //  * `ptr` comes from `self` which already satisfied requirements.
        //  * The ownership (`O::Joined`) in the return type is correct.
        unsafe {
            R::from_raw(ptr)
        }
    }
}

impl<T: ?Sized, O: Ownership> Drop for R<T, O> {
    fn drop(&mut self) {
        if O::IS_FULL {
            let ptr = R::ptr(self);

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

impl<T: ?Sized, O: Ownership> std::convert::AsRef<T> for R<T, O> {
    fn as_ref(&self) -> &T {
        R::as_ref(self)
    }
}

impl<T: ?Sized, O: Ownership> std::borrow::Borrow<T> for R<T, O> {
    fn borrow(&self) -> &T {
        R::as_ref(self)
    }
}

impl<T: ?Sized + std::fmt::Debug, O: Ownership> std::fmt::Debug for R<T, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(R::as_ref(self), f)
    }
}

impl<T: ?Sized + std::fmt::Display, O: Ownership> std::fmt::Display for R<T, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(R::as_ref(self), f)
    }
}

impl<T: ?Sized + std::default::Default> std::default::Default for R<T, Full> {
    fn default() -> Self {
        R::new(std::default::Default::default())
    }
}

impl<T: ?Sized + Eq, O: Ownership> PartialEq for R<T, O> {
    fn eq(&self, other: &Self) -> bool {
        R::ptr_eq(self, other) && PartialEq::eq(R::as_ref(self), R::as_ref(other))
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

unsafe impl<T: ?Sized + Sync + Send, N: Ownership> Sync for R<T, N> {}
unsafe impl<T: ?Sized + Sync + Send, N: Ownership> Send for R<T, N> {}

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
