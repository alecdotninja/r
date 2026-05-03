//! Fractional ownership for values allocated by `Box`.
//!
//! `R<T, O>` is a pointer type that tracks ownership as a const-generic
//! fraction. A value starts with [`FULL`] ownership and can be split into
//! smaller ownership shares that all point to the same allocation. Shares can
//! later be joined back together with [`R::join`]. The allocation is dropped
//! only when a handle with [`FULL`] ownership is dropped or converted back into
//! a [`Box`].
//!
//! Shared handles dereference to `&T`. Only [`R<T, FULL>`] can produce `&mut T`
//! or consume the allocation, because only full ownership is unique.
//! Dropping a partial ownership handle without first joining it back into a
//! larger share is a logic error and will panic.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(generic_const_exprs)]

use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;

/// A const-generic ownership marker.
///
/// Lower numeric values represent larger ownership shares. [`FULL`] is the
/// complete ownership share. Splitting a share with [`split`] produces two
/// equal shares, and joining shares with [`join`] adds their ownership markers
/// together.
pub type Ownership = u128;

/// The marker for complete, unique ownership of an allocation.
pub const FULL: Ownership = 0;

/// Returns the ownership marker for each half of a split share.
///
/// # Panics
///
/// Panics if `input` cannot be split further.
pub const fn split(input: Ownership) -> Ownership {
    if 0 == input {
        return 2u128.pow(127);
    }

    if 0 != input % 2 {
        panic!("ownership can only be split 128 times");
    }

    input / 2
}

/// Returns the ownership marker produced by joining two shares.
///
/// # Panics
///
/// Panics if the joined ownership marker would overflow.
pub const fn join(a: Ownership, b: Ownership) -> Ownership {
    let output = a.wrapping_add(b);

    if output != 0 && (output < a || output < b) {
        panic!("invalid ownership join");
    }

    output
}

/// A pointer to a boxed allocation with a compile-time ownership share.
///
/// `R<T, FULL>` uniquely owns the allocation and drops it when the handle is
/// dropped. Other ownership shares are shared references to the same allocation
/// and panic if they are dropped before being joined.
#[repr(transparent)]
pub struct R<T: ?Sized, const O: Ownership> {
    ptr: NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T> R<T, FULL> {
    /// Allocates `value` on the heap and returns a handle with full ownership.
    pub fn new(value: T) -> Self {
        Self::from_box(Box::new(value))
    }

    /// Consumes a fully owned handle and returns the contained value.
    pub fn into_inner(r: Self) -> T {
        *Self::into_box(r)
    }
}

impl<T: ?Sized> R<T, FULL> {
    /// Converts a [`Box`] into a handle with full ownership.
    pub fn from_box(value: Box<T>) -> Self {
        let ptr = Box::into_raw(value);

        // SAFETY:
        //  * The `ptr` comes from `Box`.
        //  * The `ptr` is unique, and ownership is `FULL`.
        unsafe { Self::from_raw(ptr) }
    }

    /// Converts a fully owned handle back into a [`Box`].
    pub fn into_box(r: Self) -> Box<T> {
        let ptr = Self::leak(r);

        // SAFETY:
        //  * The `ptr` comes from Box
        //  * The `ptr` is unique because ownership is `FULL`
        unsafe { Box::from_raw(ptr) }
    }
}

impl<T: ?Sized, const O: Ownership> R<T, O> {
    /// Creates a handle from a raw pointer and an externally tracked ownership share.
    ///
    /// # Safety
    ///
    /// `ptr` must be a non-null pointer previously produced by [`Box::into_raw`].
    /// The ownership marker `O` must accurately describe the share represented
    /// by this handle, and all outstanding handles for the allocation must
    /// collectively obey the split/join ownership rules used by [`R::split`] and
    /// [`R::join`].
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        // SAFETY:
        //  * `ptr` is from Box which cannot be null.
        let ptr = unsafe { NonNull::new_unchecked(ptr) };

        R {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Consumes a handle without updating ownership and returns its raw pointer.
    ///
    /// The returned pointer must eventually be reconstructed into valid
    /// ownership handles or, for full ownership, a [`Box`].
    pub fn leak(r: Self) -> *mut T {
        let ptr = R::as_ptr(&r);
        core::mem::forget(r);
        ptr
    }

    /// Returns the raw pointer stored in this handle.
    pub fn as_ptr(r: &Self) -> *mut T {
        r.ptr.as_ptr()
    }

    /// Returns `true` if two handles point to the same allocation.
    pub fn ptr_eq<const P: Ownership>(r: &Self, other: &R<T, P>) -> bool {
        core::ptr::addr_eq(R::as_ptr(r), R::as_ptr(other))
    }

    /// Splits a handle into two equal ownership shares.
    ///
    /// Panics at compile time or runtime if the share cannot be split further.
    #[must_use = "partial ownership handles must be joined, consumed, or intentionally leaked"]
    pub fn split(r: Self) -> (R<T, { split(O) }>, R<T, { split(O) }>) {
        let ptr = R::leak(r);

        // SAFETY:
        //  * `ptr` comes from `self` which already satisfied requirements.
        //  * The ownership in the return type (`split(O)`) is correct.
        unsafe { (R::from_raw(ptr), R::from_raw(ptr)) }
    }

    /// Attempts to join two handles into a larger ownership share.
    ///
    /// Returns both handles unchanged if they point to different allocations.
    /// The resulting ownership marker can still panic at compile time or runtime
    /// if it would overflow.
    pub fn try_join<const P: Ownership>(
        r: Self,
        other: R<T, P>,
    ) -> Result<R<T, { join(O, P) }>, (Self, R<T, P>)> {
        if !R::ptr_eq(&r, &other) {
            return Err((r, other));
        }

        let ptr = R::leak(r);
        R::leak(other);

        // SAFETY:
        //  * `ptr` comes from `self` which already satisfied requirements.
        //  * The ownership in the return type (`join(O, P)`) is correct.
        unsafe { Ok(R::from_raw(ptr)) }
    }

    /// Joins two handles to the same allocation into a larger ownership share.
    ///
    /// # Panics
    ///
    /// Panics if the handles point to different allocations or if the resulting
    /// ownership marker would overflow. Use [`R::try_join`] if the handles may
    /// not point to the same allocation and the caller needs to recover them.
    pub fn join<const P: Ownership>(r: Self, other: R<T, P>) -> R<T, { join(O, P) }> {
        match R::try_join(r, other) {
            Ok(joined) => joined,
            Err(_) => panic!("Cannot join pointers to different values"),
        }
    }
}

impl<T: ?Sized, const O: Ownership> Drop for R<T, O> {
    fn drop(&mut self) {
        if FULL != O {
            if !std::thread::panicking() {
                panic!("partial ownership handle dropped without being joined");
            }

            return;
        }

        let ptr = R::as_ptr(self);

        // SAFETY:
        //  * The `ptr` comes from Box.
        //  * The `ptr` is unique because ownership (`O`) is `FULL`.
        let value = unsafe { Box::from_raw(ptr) };

        drop(value);
    }
}

impl<T: ?Sized, const O: Ownership> AsRef<T> for R<T, O> {
    fn as_ref(&self) -> &T {
        // SAFETY:
        //  * If ownership is FULL, then the *only* way to get a mut ref to the
        //      underlying data is via `r`, but that will not be possible since
        //      this method borrows `r` for the lifetime of ref to the data. On
        //      the other hand, if ownership is *not* FULL, then there is no
        //      way to get a mut ref to the underlying data. In either case,
        //      there is no way to get a mut ref to the data while this ref is
        //      valid.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> AsMut<T> for R<T, FULL> {
    fn as_mut(&mut self) -> &mut T {
        // SAFETY:
        //  * `self.ptr` is unique because ownership is `FULL`.
        //  * `self` will be borrowed for the lifetime of the mut reference.
        unsafe { self.ptr.as_mut() }
    }
}

impl<T: ?Sized, const O: Ownership> core::ops::Deref for R<T, O> {
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T: ?Sized> core::ops::DerefMut for R<T, FULL> {
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<T: ?Sized + fmt::Debug, const O: Ownership> fmt::Debug for R<T, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + fmt::Display, const O: Ownership> fmt::Display for R<T, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + Default> Default for R<T, FULL> {
    fn default() -> Self {
        R::new(Default::default())
    }
}

impl<T: ?Sized + Eq, const O: Ownership> PartialEq for R<T, O> {
    fn eq(&self, other: &Self) -> bool {
        R::ptr_eq(self, other) && PartialEq::eq(&**self, &**other)
    }
}

impl<T: ?Sized + Eq, const O: Ownership> Eq for R<T, O> {}

impl<T> From<T> for R<T, FULL> {
    fn from(value: T) -> Self {
        R::new(value)
    }
}

impl<T: ?Sized> From<Box<T>> for R<T, FULL> {
    fn from(value: Box<T>) -> Self {
        R::from_box(value)
    }
}

unsafe impl<T: ?Sized + Sync, const O: Ownership> Sync for R<T, O> {}
unsafe impl<T: ?Sized + Sync + Send, const O: Ownership> Send for R<T, O> {}

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

    #[test]
    fn dropping_partial_panics() {
        let value = R::new(7);
        let (left, right) = R::split(value);
        let ptr = R::as_ptr(&right);

        let result = std::panic::catch_unwind(|| drop(left));

        assert!(result.is_err());

        R::leak(right);

        // SAFETY:
        //  * `ptr` came from `Box` through `R::new`.
        //  * Both partial ownership handles have been consumed.
        unsafe { drop(Box::from_raw(ptr)) };
    }

    #[test]
    fn joining_different_allocations_returns_handles() {
        let first = R::new(1);
        let second = R::new(2);

        let (first_left, first_right) = R::split(first);
        let (second_left, second_right) = R::split(second);

        let result = R::try_join(first_left, second_left);

        let Err((first_left, second_left)) = result else {
            panic!("joining different allocations should fail");
        };

        let first = R::join(first_left, first_right);
        let second = R::join(second_left, second_right);

        assert_eq!(R::into_inner(first), 1);
        assert_eq!(R::into_inner(second), 2);
    }
}
