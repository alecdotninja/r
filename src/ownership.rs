use core::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Ownership(u128, u128);

pub const FULL: Ownership = Ownership(1, 1);

impl fmt::Debug for Ownership {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if FULL == *self {
            write!(f, "full ownership")
        } else {
            write!(f, "partial ownership ({}/{})", self.0, self.1)
        }
    }
}

pub const fn split(input: Ownership) -> Ownership {
    Ownership(input.0, input.1 * 2)
}

pub const fn join(a: Ownership, b: Ownership) -> Ownership {
    let improper_p = a.0 * b.1 + b.0 * a.1;
    let improper_q = a.1 * b.1;

    let reduction_factor = gcd::binary_u128(improper_p, improper_q);

    Ownership(improper_p / reduction_factor, improper_q / reduction_factor)
}
