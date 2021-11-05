pub struct Full;
pub struct SevenEighths;
pub struct ThreeQuarters;
pub struct FiveEighths;
pub struct Half;
pub struct ThreeEighths;
pub struct Quarter;
pub struct Eighth;

// SAFETY:
//  * `Split` must be a type that `JoinsWith` itself to give this type.
//  * `IS_FULL` must be `false` for all types execpt `Full`.
pub unsafe trait Ownership {
    type Split: Ownership;
    const IS_FULL: bool = false;
}

// SAFETY:
//  * The relation must be reflexive (`A: JoinsWith<B>` => `B: JoinsWith<A>` and `Joined` is the same type).
//  * The relation does make it possible to "add up" to `Full` without getting all of the pieces that were split (better explination?).
pub unsafe trait JoinsWith<Other: Ownership>: Ownership {
    type Joined: Ownership;
}

macro_rules! impl_ownership_specialized {
    (Full => $into:ty) => {
        impl_ownership_specialized!(_, Full, $into, true);
    };
    ($target:ty => $into:ty) => {
        impl_ownership_specialized!(_, $target, $into, false);
    };
    (_, $target:ty, $into:ty, $is_full:expr) => {
        // SAFETY:
        //  * The `JoinsWith` impl below ensures that `<$target::Split as JoinsWith<$target::Split>>::Joined` will be `$target`.
        //  * This macro is defined so that `$is_full` is only true for `Full`.
        unsafe impl Ownership for $target {
            type Split = $into;
            const IS_FULL: bool = $is_full;
        }

        // SAFETY:
        //  * This relation is trivially reflexive.
        //  * This relation is unqiue so it is the only way back to `$target`.
        unsafe impl JoinsWith<$into> for $into {
            type Joined = $target;
        }
    };
}

impl_ownership_specialized!(Full => Half);
impl_ownership_specialized!(Half => Quarter);
impl_ownership_specialized!(Quarter => Eighth);

pub struct Split<T: Ownership>(T);

unsafe impl<T: Ownership> Ownership for Split<T> {
    type Split = Split<Self>;
}

unsafe impl<T: Ownership> JoinsWith<Split<T>> for Split<T> {
    type Joined = T;
}

macro_rules! impl_ownership_general {
    (Full) => {
        impl_ownership_general!(_, Full, true);
    };
    ($target:ty) => {
        impl_ownership_general!(_, $target, false);
    };
    (_, $target:ty, $is_full:expr) => {
        // SAFETY:
        //  * The `Split<Self>` trivially satisfied the "Split as JoinsWith<Split>" requirement.
        //  * This macro is defined so that `$is_full` is only true for `Full`.
        unsafe impl Ownership for $target {
            type Split = Split<Self>;
            const IS_FULL: bool = $is_full;
        }
    };
}

impl_ownership_general!(SevenEighths);
impl_ownership_general!(ThreeQuarters);
impl_ownership_general!(FiveEighths);
impl_ownership_general!(ThreeEighths);
impl_ownership_general!(Eighth);

macro_rules! impl_non_trivial_join {
    ($a:ident + $b:ident = $c:ident) => {
        unsafe impl JoinsWith<$a> for $b {
            type Joined = $c;
        }

        unsafe impl JoinsWith<$b> for $a {
            type Joined = $c;
        }
    };
}

impl_non_trivial_join!(SevenEighths + Eighth = Full);
impl_non_trivial_join!(ThreeQuarters + Quarter = Full);
impl_non_trivial_join!(ThreeQuarters + Eighth = SevenEighths);
impl_non_trivial_join!(FiveEighths + ThreeEighths = Full);
impl_non_trivial_join!(FiveEighths + Quarter = SevenEighths);
impl_non_trivial_join!(FiveEighths + Eighth = ThreeQuarters);
impl_non_trivial_join!(Half + ThreeEighths = SevenEighths);
impl_non_trivial_join!(Half + Quarter = ThreeQuarters);
impl_non_trivial_join!(Half + Eighth = FiveEighths);
impl_non_trivial_join!(ThreeEighths + Quarter = FiveEighths);
impl_non_trivial_join!(ThreeEighths + Eighth = Half);
impl_non_trivial_join!(Quarter + Eighth = ThreeEighths);
