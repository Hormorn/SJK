//! Shared main-thread stack allowance for link-time and Linux runtime setup.

pub const MAIN_STACK_BYTES: u64 = 64 * 1024 * 1024;
