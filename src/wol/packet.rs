//! Wake-on-LAN Magic Packet Generator & MAC Parser

use core::fmt::Write;
use heapless::String;

pub const MAGIC_PACKET_LEN: usize = 102;
pub const WOL_PORT: u16 = 9;

/// Generates a 102-byte Wake-on-LAN Magic Packet for the target MAC.
///
/// Format:
/// - 6 bytes of 0xFF
/// - 16 repetitions of the 6-byte MAC address
pub fn create_magic_packet(mac: [u8; 6]) -> [u8; MAGIC_PACKET_LEN] {
    let mut packet = [0u8; MAGIC_PACKET_LEN];
    packet[0..6].fill(0xFF);
    for i in 0..16 {
        let offset = 6 + (i * 6);
        packet[offset..offset + 6].copy_from_slice(&mac);
    }
    packet
}

/// Parses a MAC address from a string (e.g. "AA:BB:CC:DD:EE:FF" or "aa-bb-cc-dd-ee-ff").
pub fn parse_mac_address(s: &str) -> Option<[u8; 6]> {
    let mut mac = [0u8; 6];
    let mut byte_idx = 0;
    let mut high_nibble: Option<u8> = None;

    for b in s.bytes() {
        let nibble = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            b':' | b'-' => continue,
            _ => return None,
        };

        if let Some(high) = high_nibble {
            if byte_idx >= 6 {
                return None;
            }
            mac[byte_idx] = (high << 4) | nibble;
            byte_idx += 1;
            high_nibble = None;
        } else {
            high_nibble = Some(nibble);
        }
    }

    if byte_idx == 6 && high_nibble.is_none() {
        Some(mac)
    } else {
        None
    }
}

/// Formats a 6-byte MAC address into "XX:XX:XX:XX:XX:XX"
pub fn format_mac_address(mac: [u8; 6]) -> String<18> {
    let mut s = String::new();
    let _ = write!(
        s,
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    s
}
