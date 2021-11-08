use core::ops::{Add, Div, Mul};

use typenum::consts::{B1, U1, U2, U4, U8};
use typenum::{op, UInt, Gcd, Unsigned, NonZero, PowerOfTwo};

pub trait Odd {}
impl<U: Unsigned> Odd for UInt<U, B1> {}

pub struct F<P, Q>(P, Q)
where
    P: Unsigned + NonZero + Odd,
    Q: Unsigned + NonZero + PowerOfTwo;

pub type Full       = F<U1, U1>;
pub type Half       = F<U1, U2>;
pub type Quarter    = F<U1, U4>;
pub type Eighth     = F<U1, U8>;

pub unsafe trait Ownership {
    const IS_FULL: bool;
}

pub unsafe trait CanSplit: Ownership {
    type Split: Ownership;
}

pub unsafe trait JoinsWith<Other: Ownership>: Ownership {
    type Joined: Ownership;
}

unsafe impl<P, Q> Ownership for F<P, Q>
where
    P: Unsigned + NonZero + Odd,
    Q: Unsigned + NonZero + PowerOfTwo,
{
    const IS_FULL: bool = (P::USIZE == 1usize && Q::USIZE == 1usize);
}

unsafe impl<P, Q> CanSplit for F<P, Q>
where
    P: Unsigned + NonZero + Odd,
    Q: Unsigned + NonZero + PowerOfTwo + Mul<U2>,
    op!(Q * U2): Unsigned + NonZero + PowerOfTwo,
{
    type Split = F<P, op!(Q * U2)>;
}

type InproperNumerator<P, Q, R, S> =
    op!(P * S + R * Q);

type InproperDenominator<Q, S> =
    op!(Q * S);

type ReducingFactor<P, Q, R, S> =
    <InproperNumerator<P, Q, R, S> as Gcd<InproperDenominator<Q, S>>>::Output;

type ReducedNumerator<P, Q, R, S> =
    <InproperNumerator<P, Q, R, S> as Div<ReducingFactor<P, Q, R, S>>>::Output;

type ReducedDenominator<P, Q, R, S> =
    <InproperDenominator<Q, S> as Div<ReducingFactor<P, Q, R, S>>>::Output;

unsafe impl<P, Q, R, S> JoinsWith<F<R, S>> for F<P, Q>
where
    P: Unsigned + NonZero + Odd + Mul<S>,
    Q: Unsigned + NonZero + PowerOfTwo + Mul<S>,
    R: Unsigned + NonZero + Odd + Mul<Q>,
    S: Unsigned + NonZero + PowerOfTwo + Mul<Q>,

    // It is possible to create `InproperNumerator`
    op!(P * S): Add<op!(R * Q)>,

    // It is possible to create `ReducingFactor`
    InproperNumerator<P, Q, R, S>: Gcd<InproperDenominator<Q, S>>,

    // It is possible to create `ReducedNumerator` and `ReducedDenominator`
    InproperNumerator<P, Q, R, S>: Div<ReducingFactor<P, Q, R, S>>,    
    InproperDenominator<Q, S>: Div<ReducingFactor<P, Q, R, S>>,

    // It is possible to use `ReducedNumerator` and `ReducedDenominator` in `F`
    ReducedNumerator<P, Q, R, S>: Unsigned + NonZero + Odd,
    ReducedDenominator<P, Q, R, S>: Unsigned + NonZero + PowerOfTwo,
{
    type Joined = F<
        ReducedNumerator<P, Q, R, S>,
        ReducedDenominator<P, Q, R, S>,
    >;
}
