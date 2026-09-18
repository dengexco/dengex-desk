//! Windows adapters. Real Windows interactive-desktop acceptance is required.
#[cfg(windows)]
pub mod capture;
#[cfg(windows)]
pub mod encoder;
#[cfg(windows)]
pub mod input;
pub mod pixels;
pub const RUNTIME_VERIFIED: bool = false;
