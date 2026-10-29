//! Station (STA) Mode Network Subsystem
//!
//! Manages connection to local Wi-Fi router, DHCP IP acquisition,
//! and network maintenance.

use esp_hal::delay::Delay;
use esp_println::println;
use esp_wifi::wifi::{
    AuthMethod, ClientConfiguration, Configuration, WifiStaDevice,
};
use smoltcp::{
    iface::{SocketSet, SocketStorage},
    socket::dhcpv4::{Event as DhcpEvent, Socket as Dhcpv4Socket},
    wire::{IpCidr, Ipv4Cidr},
};

use crate::{net::now_instant, storage::WifiCredentials};

/// Runs Station Mode: connects to home Wi-Fi and acquires IP via DHCP.
pub fn run_station_mode(
    init: &esp_wifi::EspWifiController<'_>,
    wifi_peripheral: esp_hal::peripherals::WIFI,
    creds: &WifiCredentials,
    delay: &Delay,
) -> ! {
    println!("🚀 Starting in Station Mode (Connecting to '{}')...", creds.ssid);
    println!("ℹ️  SoftAP 'POWER-CRAB' is disabled.");

    let (mut iface, mut device, mut controller) =
        esp_wifi::wifi::utils::create_network_interface(init, wifi_peripheral, WifiStaDevice)
            .unwrap();

    let sta_config = Configuration::Client(ClientConfiguration {
        ssid: creds.ssid.clone(),
        password: creds.password.clone(),
        auth_method: if creds.password.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        },
        ..Default::default()
    });

    controller.set_configuration(&sta_config).unwrap();
    controller.start().unwrap();
    println!("📡 Connecting to '{}'...", creds.ssid);
    let _ = controller.connect();

    // Setup DHCP Client
    let dhcp_socket = Dhcpv4Socket::new();
    let mut socket_storage = [SocketStorage::EMPTY; 1];
    let mut sockets = SocketSet::new(&mut socket_storage[..]);
    let dhcp_handle = sockets.add(dhcp_socket);

    let mut assigned_ip: Option<Ipv4Cidr> = None;
    let mut connected_logged = false;
    let mut loop_count: u32 = 0;

    loop {
        let timestamp = now_instant();
        iface.poll(timestamp, &mut device, &mut sockets);

        match controller.is_connected() {
            Ok(true) => {
                if !connected_logged {
                    println!("📶 Wi-Fi Layer Connected! Requesting DHCP IP...");
                    connected_logged = true;
                    // Reset DHCP state to send DISCOVER immediately now that link is up
                    let dhcp_sock = sockets.get_mut::<Dhcpv4Socket>(dhcp_handle);
                    dhcp_sock.reset();
                }

                // Poll DHCP client socket
                let dhcp_sock = sockets.get_mut::<Dhcpv4Socket>(dhcp_handle);
                if let Some(event) = dhcp_sock.poll() {
                    match event {
                        DhcpEvent::Configured(config) => {
                            println!("🎉 IP Address Assigned by Router: {}", config.address);
                            if let Some(router) = config.router {
                                println!("   Gateway: {}", router);
                            }
                            assigned_ip = Some(config.address);
                            iface.update_ip_addrs(|addrs| {
                                addrs.clear();
                                addrs.push(IpCidr::Ipv4(config.address)).ok();
                            });
                            if let Some(router) = config.router {
                                iface.routes_mut().add_default_ipv4_route(router).ok();
                            }
                        }
                        DhcpEvent::Deconfigured => {
                            println!("⚠️ DHCP Deconfigured");
                            assigned_ip = None;
                        }
                    }
                }
            }
            Err(_) => {
                connected_logged = false;
            }
            _ => {}
        }

        loop_count += 1;
        if loop_count % 100 == 0 {
            println!(
                "📡 STA Status: {:?} | Assigned IP: {:?}",
                controller.is_connected(),
                assigned_ip
            );
        }

        delay.delay_millis(10);
    }
}
