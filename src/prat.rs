use core::ops::{Add, Mul, Div};
use typenum::{Unsigned, NonZero, U1, U2, Gcd, op};

pub struct Fraction<P, Q>(P, Q)
where
    P: Unsigned + NonZero,
    Q: Unsigned + NonZero;

pub type Full = Fraction<U1, U1>;

pub trait Ownership {
    const IS_FULL: bool;
}

pub trait CanSplit: Ownership {
    type Split: Ownership;
}

pub trait JoinsWith<Other: Ownership>: Ownership {
    type Joined: Ownership;
}

impl<P, Q> Ownership for Fraction<P, Q>
where
    P: Unsigned + NonZero,
    Q: Unsigned + NonZero,
{
    const IS_FULL: bool = (
        P::USIZE == 1usize &&
        Q::USIZE == 1usize
    );
}

impl<P, Q> CanSplit for Fraction<P, Q>
where
    P: Unsigned + NonZero,
    Q: Unsigned + NonZero + Mul<U2>,
    op!(Q * U2): Unsigned + NonZero,
{
    type Split = Fraction<P, op!(Q * U2)>;
}

impl<P, Q, R, S> JoinsWith<Fraction<R, S>> for Fraction<P, Q>
where
    P: Unsigned + NonZero + Mul<S>,
    Q: Unsigned + NonZero + Mul<S>,
    R: Unsigned + NonZero + Mul<Q>,
    S: Unsigned + NonZero,

    op!(P*S): Add<op!(R*Q)>,
    op!(P*S + R*Q): Gcd<op!(Q*S)> + Div<op!(gcd(P*S + R*Q, Q*S))>,
    op!(Q*S): Div<op!(gcd(P*S + R*Q, Q*S))>,

    op!((P*S + R*Q)/gcd(P*S + R*Q, Q*S)): Unsigned + NonZero,
    op!(Q*S/gcd(P*S + R*Q, Q*S)): Unsigned + NonZero
{
    type Joined = Fraction<
        op!((P*S + R*Q)/gcd(P*S + R*Q, Q*S)),
        op!(Q*S/gcd(P*S + R*Q, Q*S)),
    >;
}