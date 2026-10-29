//! Power-Crab Core Library
//!
//! Bare-metal (`no_std`) library for ESP32.

#![no_std]

extern crate alloc;

pub mod net;
pub mod portal;
pub mod station;
pub mod storage;
pub mod traits;
pub mod wol;

// Convenient top-level re-exports
pub use storage::{Storage, StorageError, WifiCredentials};
pub use traits::WifiStorage;
