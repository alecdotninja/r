use core::ops::{Add, Div, Mul};
use typenum::consts::{B1, U1, U2, U4, U8};
use typenum::{op, Gcd, NonZero, PowerOfTwo, UInt, Unsigned};

pub trait Odd {}
impl<U: Unsigned> Odd for UInt<U, B1> {}

pub trait Num: Unsigned + NonZero + Odd {}
impl<T: Unsigned + NonZero + Odd> Num for T {}

pub trait Den: Unsigned + NonZero + PowerOfTwo {}
impl<T: Unsigned + NonZero + PowerOfTwo> Den for T {}

pub struct F<P: Num, Q: Den>(P, Q);

pub type Full = F<U1, U1>;
pub type Half = F<U1, U2>;
pub type Quarter = F<U1, U4>;
pub type Eighth = F<U1, U8>;

pub unsafe trait Ownership {
    const IS_FULL: bool;
}

pub unsafe trait CanSplit: Ownership {
    type Split: Ownership;
}

pub unsafe trait JoinsWith<Other: Ownership>: Ownership {
    type Joined: Ownership;
}

unsafe impl<P: Num, Q: Den> Ownership for F<P, Q> {
    const IS_FULL: bool = (P::USIZE == 1usize && Q::USIZE == 1usize);
}

unsafe impl<P: Num, Q: Den> CanSplit for F<P, Q>
where
    Q: Mul<U2>,
    op!(Q * U2): Den,
{
    type Split = F<P, op!(Q * U2)>;
}

type InproperNum<P, Q, R, S> = op!(P * S + R * Q);

type InproperDen<Q, S> = op!(Q * S);

type ReduceFactor<P, Q, R, S> = <InproperNum<P, Q, R, S> as Gcd<InproperDen<Q, S>>>::Output;

type ReducedNum<P, Q, R, S> = <InproperNum<P, Q, R, S> as Div<ReduceFactor<P, Q, R, S>>>::Output;

type ReducedDen<P, Q, R, S> = <InproperDen<Q, S> as Div<ReduceFactor<P, Q, R, S>>>::Output;

unsafe impl<P: Num, Q: Den, R: Num, S: Den> JoinsWith<F<R, S>> for F<P, Q>
where
    // It is possible to create `InproperNum`
    P: Mul<S>,
    R: Mul<Q>,
    op!(P * S): Add<op!(R * Q)>,

    // It is possible to create `InproperDen`
    Q: Mul<S>,

    // It is possible to create `ReduceFactor`
    InproperNum<P, Q, R, S>: Gcd<InproperDen<Q, S>>,

    // It is possible to create `ReducedNum` and `ReducedDen`
    InproperNum<P, Q, R, S>: Div<ReduceFactor<P, Q, R, S>>,
    InproperDen<Q, S>: Div<ReduceFactor<P, Q, R, S>>,

    // `ReducedNum` and `ReducedDen` can be used in `F`
    ReducedNum<P, Q, R, S>: Unsigned + NonZero + Odd,
    ReducedDen<P, Q, R, S>: Unsigned + NonZero + PowerOfTwo,
{
    type Joined = F<ReducedNum<P, Q, R, S>, ReducedDen<P, Q, R, S>>;
}
