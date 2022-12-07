#![deny(unsafe_op_in_unsafe_fn)]
#![feature(generic_const_exprs, adt_const_params)]

pub mod ownership;
mod r;

pub use crate::r::R;
