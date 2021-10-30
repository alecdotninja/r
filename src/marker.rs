pub struct Full;
pub struct Partial<N: Ownership>(N);

// SAFETY: Must ensure that `IS_FULL` is set correctly.
pub unsafe trait Ownership {
    const IS_FULL: bool;
}

// SAFETY: `IS_FULL` is set correctly.
unsafe impl Ownership for Full {
    const IS_FULL: bool = true;
}

// SAFETY: `IS_FULL` is set correctly.
unsafe impl<N: Ownership> Ownership for Partial<N> {
    const IS_FULL: bool = false;
}
