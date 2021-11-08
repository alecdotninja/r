use core::ops::{Add, Div, Mul};

use typenum::consts::*;
use typenum::{op, Gcd, NonZero, Unsigned};

pub struct Fraction<P, Q>(P, Q)
where
    P: Unsigned + NonZero,
    Q: Unsigned + NonZero;

pub type Full       = Fraction<U1, U1>;
pub type Half       = Fraction<U1, U2>;
pub type Quarter    = Fraction<U1, U4>;
pub type Eighth     = Fraction<U1, U8>;

pub unsafe trait Ownership {
    const IS_FULL: bool;
}

pub unsafe trait CanSplit: Ownership {
    type Split: Ownership;
}

pub unsafe trait JoinsWith<Other: Ownership>: Ownership {
    type Joined: Ownership;
}

unsafe impl<P, Q> Ownership for Fraction<P, Q>
where
    P: Unsigned + NonZero,
    Q: Unsigned + NonZero,
{
    const IS_FULL: bool = (P::USIZE == 1usize && Q::USIZE == 1usize);
}

unsafe impl<P, Q> CanSplit for Fraction<P, Q>
where
    P: Unsigned + NonZero,
    Q: Unsigned + NonZero + Mul<U2>,
    op!(Q * U2): Unsigned + NonZero,
{
    type Split = Fraction<P, op!(Q * U2)>;
}

unsafe impl<P, Q, R, S> JoinsWith<Fraction<R, S>> for Fraction<P, Q>
where
    P: Unsigned + NonZero + Mul<S>,
    Q: Unsigned + NonZero + Mul<S>,
    R: Unsigned + NonZero + Mul<Q>,
    S: Unsigned + NonZero,

    op!(P * S): Add<op!(R * Q)>,
    op!(P * S + R * Q): Gcd<op!(Q * S)> + Div<op!(gcd(P * S + R * Q, Q * S))>,
    op!(Q * S): Div<op!(gcd(P * S + R * Q, Q * S))>,

    op!((P * S + R * Q) / gcd(P * S + R * Q, Q * S)): Unsigned + NonZero,
    op!(Q * S / gcd(P * S + R * Q, Q * S)): Unsigned + NonZero,
{
    type Joined = Fraction<
        op!((P * S + R * Q) / gcd(P * S + R * Q, Q * S)),
        op!(Q * S / gcd(P * S + R * Q, Q * S)),
    >;
}
