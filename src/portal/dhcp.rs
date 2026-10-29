//! Lightweight DHCP Server for Captive Portal
//!
//! Implements a minimal, robust RFC 2131 compliant DHCP server in `no_std`.
//! Listens on UDP port 67 and provides connecting Wi-Fi clients with an IP address,
//! gateway, subnet mask, and DNS server address pointing to the ESP32 (`192.168.4.1`).

pub const DHCP_SERVER_PORT: u16 = 67;
pub const DHCP_CLIENT_PORT: u16 = 68;

const MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];

const DHCP_DISCOVER: u8 = 1;
const DHCP_OFFER: u8 = 2;
const DHCP_REQUEST: u8 = 3;
const DHCP_ACK: u8 = 5;

const OPTION_PAD: u8 = 0;
const OPTION_SUBNET_MASK: u8 = 1;
const OPTION_ROUTER: u8 = 3;
const OPTION_DNS: u8 = 6;
const OPTION_LEASE_TIME: u8 = 51;
const OPTION_MSG_TYPE: u8 = 53;
const OPTION_SERVER_ID: u8 = 54;
const OPTION_END: u8 = 255;

/// Processes an incoming DHCP request and generates the corresponding response
/// (DHCPOFFER for DHCPDISCOVER, DHCPACK for DHCPREQUEST).
///
/// Returns the length of the response written to `resp`, or `None` if invalid.
pub fn process_dhcp_packet(
    req: &[u8],
    resp: &mut [u8],
    server_ip: [u8; 4],
    offered_ip: [u8; 4],
) -> Option<usize> {
    // Minimum DHCP packet size: 236 bytes header + 4 bytes magic cookie
    if req.len() < 240 || resp.len() < 300 {
        return None;
    }

    // Must be a BOOTREQUEST (op = 1)
    if req[0] != 1 {
        return None;
    }

    // Verify Magic Cookie: 99.130.83.99
    if req[236..240] != MAGIC_COOKIE {
        return None;
    }

    // Parse options to find Option 53 (DHCP Message Type)
    let mut msg_type: Option<u8> = None;
    let mut opt_idx = 240;

    while opt_idx < req.len() {
        let opt = req[opt_idx];
        if opt == OPTION_PAD {
            opt_idx += 1;
            continue;
        }
        if opt == OPTION_END {
            break;
        }
        if opt_idx + 1 >= req.len() {
            break;
        }

        let opt_len = req[opt_idx + 1] as usize;
        let opt_val_start = opt_idx + 2;
        let opt_val_end = opt_val_start + opt_len;

        if opt_val_end > req.len() {
            break;
        }

        if opt == OPTION_MSG_TYPE && opt_len == 1 {
            msg_type = Some(req[opt_val_start]);
        }

        opt_idx = opt_val_end;
    }

    let reply_type = match msg_type? {
        DHCP_DISCOVER => DHCP_OFFER,
        DHCP_REQUEST => DHCP_ACK,
        _ => return None,
    };

    // Construct BOOTREPLY (op = 2)
    resp.fill(0);
    resp[0] = 2; // BOOTREPLY
    resp[1] = req[1]; // Hardware type (Ethernet = 1)
    resp[2] = req[2]; // Hardware address length (6)
    resp[3] = 0; // Hops

    // Copy Transaction ID (xid)
    resp[4..8].copy_from_slice(&req[4..8]);

    // Flags: Broadcast flag set (0x8000) so unconfigured clients can receive it
    resp[10] = 0x80;
    resp[11] = 0x00;

    // Your (client) IP address
    resp[16..20].copy_from_slice(&offered_ip);

    // Next server IP address
    resp[20..24].copy_from_slice(&server_ip);

    // Client hardware address (MAC)
    resp[28..44].copy_from_slice(&req[28..44]);

    // Magic Cookie
    resp[236..240].copy_from_slice(&MAGIC_COOKIE);

    // Write DHCP Options
    let mut out = 240;

    // Option 53: Message Type
    resp[out] = OPTION_MSG_TYPE;
    resp[out + 1] = 1;
    resp[out + 2] = reply_type;
    out += 3;

    // Option 54: Server Identifier
    resp[out] = OPTION_SERVER_ID;
    resp[out + 1] = 4;
    resp[out + 2..out + 6].copy_from_slice(&server_ip);
    out += 6;

    // Option 51: Lease Time (86400s = 1 day: 0x00015180)
    resp[out] = OPTION_LEASE_TIME;
    resp[out + 1] = 4;
    resp[out + 2..out + 6].copy_from_slice(&[0x00, 0x01, 0x51, 0x80]);
    out += 6;

    // Option 1: Subnet Mask (255.255.255.0)
    resp[out] = OPTION_SUBNET_MASK;
    resp[out + 1] = 4;
    resp[out + 2..out + 6].copy_from_slice(&[255, 255, 255, 0]);
    out += 6;

    // Option 3: Router / Gateway
    resp[out] = OPTION_ROUTER;
    resp[out + 1] = 4;
    resp[out + 2..out + 6].copy_from_slice(&server_ip);
    out += 6;

    // Option 6: DNS Server (pointing to ESP32 for captive portal redirect)
    resp[out] = OPTION_DNS;
    resp[out + 1] = 4;
    resp[out + 2..out + 6].copy_from_slice(&server_ip);
    out += 6;

    // Option 255: End
    resp[out] = OPTION_END;
    out += 1;

    Some(out)
}
