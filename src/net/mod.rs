//! Networking Subsystem Utilities

pub mod sniffer;
pub mod time;

pub use sniffer::{DiscoveredDevice, SnifferDevice, MAX_DISCOVERED_DEVICES};
pub use time::now_instant;
