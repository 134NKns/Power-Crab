//! DNS Captive Hijacker Module
//!
//! Listens for incoming DNS queries on UDP Port 53 and responds to ALL queries
//! with the ESP32's Access Point IP address (`192.168.4.1`).
//! This immediately triggers the Captive Portal popup on iOS, Android, macOS, and Windows.

pub const DNS_PORT: u16 = 53;
pub const AP_IP_BYTES: [u8; 4] = [192, 168, 4, 1];

/// Processes an incoming DNS query and creates a spoofed DNS response pointing to `ap_ip`.
///
/// Returns the length of the response packet in `buf`, or `None` if the request is invalid.
pub fn process_dns_query(buf: &mut [u8], len: usize, ap_ip: [u8; 4]) -> Option<usize> {
    // Standard DNS header is 12 bytes, minimum question is ~5 bytes
    if len < 17 || len > 512 {
        return None;
    }

    // Must have at least 1 question (QDCOUNT > 0)
    let qdcount = u16::from_be_bytes([buf[4], buf[5]]);
    if qdcount == 0 {
        return None;
    }

    // 1. Modify DNS Header:
    // Flags: Response, Opcode 0, Authoritative, Recursion Desired (keep if set)
    buf[2] = 0x81;
    buf[3] = 0x80;

    // Set ANCOUNT (Answer count) = 1
    buf[6] = 0x00;
    buf[7] = 0x01;

    // Set NSCOUNT = 0, ARCOUNT = 0
    buf[8] = 0x00;
    buf[9] = 0x00;
    buf[10] = 0x00;
    buf[11] = 0x00;

    // Find the end of the Question section
    // Starts at byte 12 (QNAME: sequence of labels ending with 0x00)
    let mut pos = 12;
    while pos < len {
        let label_len = buf[pos] as usize;
        if label_len == 0 {
            pos += 1;
            break;
        }
        pos += 1 + label_len;
    }

    // After QNAME are QTYPE (2 bytes) and QCLASS (2 bytes)
    pos += 4;
    if pos > len {
        return None;
    }

    // 2. Append Answer Record (16 bytes):
    // Name: Pointer to question name at offset 12 (0xC00C)
    // Type: A (0x0001)
    // Class: IN (0x0001)
    // TTL: 60s (0x0000003C)
    // RDLENGTH: 4 (0x0004)
    // RDATA: IP bytes
    let answer: [u8; 16] = [
        0xC0, 0x0C, // Compression pointer to original domain name
        0x00, 0x01, // Type A (IPv4)
        0x00, 0x01, // Class IN
        0x00, 0x00, 0x00, 0x3C, // TTL: 60 seconds
        0x00, 0x04, // Data length: 4 bytes
        ap_ip[0], ap_ip[1], ap_ip[2], ap_ip[3], // Resolved IP
    ];

    if pos + answer.len() > buf.len() {
        return None;
    }

    buf[pos..pos + answer.len()].copy_from_slice(&answer);
    Some(pos + answer.len())
}
