#![no_std]
#![deny(unsafe_code)]
extern crate alloc;

mod access;
mod call;
pub mod constants;
pub mod error;
mod intrinsic;
mod memory;
pub mod meter;
mod sstore;
mod transient;

pub use error::GasError;
pub use meter::GasMeter;
