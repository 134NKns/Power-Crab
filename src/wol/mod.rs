//! Wake-on-LAN (WOL) Subsystem
//!
//! Provides magic packet construction, MAC address parsing, and UDP broadcast dispatch.

pub mod packet;

pub use packet::{create_magic_packet, format_mac_address, parse_mac_address, MAGIC_PACKET_LEN, WOL_PORT};
