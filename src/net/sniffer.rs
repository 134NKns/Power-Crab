//! LAN Packet Sniffer & Device Discovery Subsystem
//!
//! Wraps the PHY Device to passively and actively discover devices
//! (IP and MAC addresses) on the local area network.

use smoltcp::phy::{Device, DeviceCapabilities, PacketMeta, RxToken};
use smoltcp::time::Instant;

/// Maximum number of discovered devices tracked in memory
pub const MAX_DISCOVERED_DEVICES: usize = 20;

/// Information about an active device on the local network
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub ip: [u8; 4],
    pub mac: [u8; 6],
    pub last_seen_ms: u64,
}

/// Device wrapper that inspects incoming Ethernet frames to discover LAN hosts
pub struct SnifferDevice<D: Device> {
    inner: D,
    devices: [Option<DiscoveredDevice>; MAX_DISCOVERED_DEVICES],
}

impl<D: Device> SnifferDevice<D> {
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            devices: [None; MAX_DISCOVERED_DEVICES],
        }
    }

    pub fn inner_mut(&mut self) -> &mut D {
        &mut self.inner
    }

    pub fn devices(&self) -> &[Option<DiscoveredDevice>; MAX_DISCOVERED_DEVICES] {
        &self.devices
    }

    pub fn find_by_ip(&self, ip: [u8; 4]) -> Option<DiscoveredDevice> {
        for entry in &self.devices {
            if let Some(dev) = entry {
                if dev.ip == ip {
                    return Some(*dev);
                }
            }
        }
        None
    }

    pub fn find_by_mac(&self, mac: [u8; 6]) -> Option<DiscoveredDevice> {
        for entry in &self.devices {
            if let Some(dev) = entry {
                if dev.mac == mac {
                    return Some(*dev);
                }
            }
        }
        None
    }

    /// Checks if a device has been seen within `timeout_ms` milliseconds
    pub fn is_alive(&self, mac: [u8; 6], now_ms: u64, timeout_ms: u64) -> bool {
        if let Some(dev) = self.find_by_mac(mac) {
            now_ms.saturating_sub(dev.last_seen_ms) <= timeout_ms
        } else {
            false
        }
    }
}

impl<D: Device> Device for SnifferDevice<D> {
    type RxToken<'a> = SnifferRxToken<'a, D::RxToken<'a>> where Self: 'a;
    type TxToken<'a> = D::TxToken<'a> where Self: 'a;

    fn receive(&mut self, timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let now_ms = timestamp.total_millis().max(0) as u64;
        self.inner.receive(timestamp).map(|(rx, tx)| {
            (
                SnifferRxToken {
                    rx,
                    devices: &mut self.devices,
                    now_ms,
                },
                tx,
            )
        })
    }

    fn transmit(&mut self, timestamp: Instant) -> Option<Self::TxToken<'_>> {
        self.inner.transmit(timestamp)
    }

    fn capabilities(&self) -> DeviceCapabilities {
        self.inner.capabilities()
    }
}

pub struct SnifferRxToken<'a, R: RxToken> {
    rx: R,
    devices: &'a mut [Option<DiscoveredDevice>; MAX_DISCOVERED_DEVICES],
    now_ms: u64,
}

impl<'a, R: RxToken> RxToken for SnifferRxToken<'a, R> {
    fn consume<Res, F>(self, f: F) -> Res
    where
        F: FnOnce(&[u8]) -> Res,
    {
        self.rx.consume(|buffer| {
            inspect_ethernet_frame(buffer, self.devices, self.now_ms);
            f(buffer)
        })
    }

    fn meta(&self) -> PacketMeta {
        self.rx.meta()
    }
}

fn inspect_ethernet_frame(
    buf: &[u8],
    devices: &mut [Option<DiscoveredDevice>; MAX_DISCOVERED_DEVICES],
    now_ms: u64,
) {
    if buf.len() < 14 {
        return;
    }

    let ethertype = [buf[12], buf[13]];

    // 1. ARP Packet (EtherType 0x0806)
    if ethertype == [0x08, 0x06] && buf.len() >= 42 {
        let sender_mac: [u8; 6] = [buf[22], buf[23], buf[24], buf[25], buf[26], buf[27]];
        let sender_ip: [u8; 4] = [buf[28], buf[29], buf[30], buf[31]];
        record_device(devices, sender_ip, sender_mac, now_ms);
        return;
    }

    // 2. IPv4 Packet (EtherType 0x0800)
    if ethertype == [0x08, 0x00] && buf.len() >= 34 {
        let src_mac: [u8; 6] = [buf[6], buf[7], buf[8], buf[9], buf[10], buf[11]];
        let src_ip: [u8; 4] = [buf[26], buf[27], buf[28], buf[29]];
        record_device(devices, src_ip, src_mac, now_ms);
    }
}

fn record_device(
    devices: &mut [Option<DiscoveredDevice>; MAX_DISCOVERED_DEVICES],
    ip: [u8; 4],
    mac: [u8; 6],
    now_ms: u64,
) {
    // Ignore invalid IPs: 0.0.0.0, 255.255.255.255, multicast (224.0.0.0/4)
    if ip[0] == 0 || ip[0] >= 224 || ip == [255, 255, 255, 255] {
        return;
    }

    // Ignore multicast or broadcast MAC: 00:00:00:00:00:00, FF:FF:FF:FF:FF:FF, 01:00:5E:...
    if mac == [0; 6] || mac == [0xFF; 6] || (mac[0] & 1) != 0 {
        return;
    }

    // Update existing entry if present
    for entry in devices.iter_mut() {
        if let Some(dev) = entry {
            if dev.mac == mac || dev.ip == ip {
                dev.ip = ip;
                dev.mac = mac;
                dev.last_seen_ms = now_ms;
                return;
            }
        }
    }

    // Insert into first empty slot
    for entry in devices.iter_mut() {
        if entry.is_none() {
            *entry = Some(DiscoveredDevice {
                ip,
                mac,
                last_seen_ms: now_ms,
            });
            return;
        }
    }

    // If full, replace the oldest entry
    let mut oldest_idx = 0;
    let mut oldest_time = u64::MAX;
    for (i, entry) in devices.iter().enumerate() {
        if let Some(dev) = entry {
            if dev.last_seen_ms < oldest_time {
                oldest_time = dev.last_seen_ms;
                oldest_idx = i;
            }
        }
    }

    devices[oldest_idx] = Some(DiscoveredDevice {
        ip,
        mac,
        last_seen_ms: now_ms,
    });
}
