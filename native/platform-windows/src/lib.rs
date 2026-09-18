//! Windows adapters. Real Windows interactive-desktop acceptance is required.
#[cfg(windows)]
pub mod capture;
#[cfg(windows)]
pub mod input;
pub const RUNTIME_VERIFIED: bool = false;
