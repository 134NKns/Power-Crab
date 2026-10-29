//! Station (STA) Mode Network Subsystem
//!
//! Connects to home Wi-Fi router, maintains DHCP lease,
//! serves LAN Web Dashboard & REST API on port 80,
//! and dispatches Wake-on-LAN Magic Packets.

pub mod web;

extern crate alloc;
use alloc::format;
use esp_hal::delay::Delay;
use esp_println::println;
use esp_wifi::wifi::{
    AuthMethod, ClientConfiguration, Configuration, WifiStaDevice,
};
use smoltcp::{
    iface::{SocketSet, SocketStorage},
    socket::{
        dhcpv4::{Event as DhcpEvent, Socket as Dhcpv4Socket},
        tcp::{Socket as TcpSocket, SocketBuffer as TcpSocketBuffer, State as TcpState},
        udp::{PacketBuffer as UdpPacketBuffer, PacketMetadata, Socket as UdpSocket},
    },
    wire::{IpAddress, IpCidr, IpEndpoint, Ipv4Address, Ipv4Cidr},
};

use crate::{
    net::now_instant,
    station::web::{
        format_http_response, generate_dashboard_html, parse_station_request,
        StationHttpRequest, ADMIN_PASSWORD,
    },
    storage::{Storage, WifiCredentials},
    wol::{create_magic_packet, format_mac_address, parse_mac_address, WOL_PORT},
};

/// Runs Station Mode with LAN Web Dashboard, REST API, and Wake-on-LAN engine.
pub fn run_station_mode(
    init: &esp_wifi::EspWifiController<'_>,
    wifi_peripheral: esp_hal::peripherals::WIFI,
    creds: &WifiCredentials,
    delay: &Delay,
    storage: &mut Storage,
) -> ! {
    println!("🚀 Starting in Station Mode (Connecting to '{}')...", creds.ssid);
    println!("ℹ️  SoftAP 'POWER-CRAB' is disabled.");

    let mut current_target_mac = creds.target_mac;
    if let Some(mac) = current_target_mac {
        println!("🖥️  Target PC MAC: {}", format_mac_address(mac));
    } else {
        println!("🖥️  Target PC MAC: Not configured");
    }

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

    // 1. DHCP Client Socket
    let dhcp_socket = Dhcpv4Socket::new();

    // 2. HTTP Server Socket (TCP port 80)
    let mut http_rx_data = [0u8; 2048];
    let mut http_tx_data = [0u8; 8192];
    let mut http_socket = TcpSocket::new(
        TcpSocketBuffer::new(&mut http_rx_data[..]),
        TcpSocketBuffer::new(&mut http_tx_data[..]),
    );
    http_socket.listen(80).unwrap();

    // 3. UDP Socket for Wake-on-LAN Broadcast (port 54321)
    let mut wol_rx_meta = [PacketMetadata::EMPTY; 2];
    let mut wol_rx_data = [0u8; 256];
    let mut wol_tx_meta = [PacketMetadata::EMPTY; 4];
    let mut wol_tx_data = [0u8; 512];
    let mut wol_socket = UdpSocket::new(
        UdpPacketBuffer::new(&mut wol_rx_meta[..], &mut wol_rx_data[..]),
        UdpPacketBuffer::new(&mut wol_tx_meta[..], &mut wol_tx_data[..]),
    );
    wol_socket.bind(54321).unwrap();

    let mut socket_storage = [SocketStorage::EMPTY; 3];
    let mut sockets = SocketSet::new(&mut socket_storage[..]);
    let dhcp_handle = sockets.add(dhcp_socket);
    let http_handle = sockets.add(http_socket);
    let wol_handle = sockets.add(wol_socket);

    let mut assigned_ip: Option<Ipv4Cidr> = None;
    let mut gateway_ip: Option<Ipv4Address> = None;
    let mut connected_logged = false;
    let mut loop_count: u32 = 0;
    let mut http_req_buf = [0u8; 2048];
    let mut http_resp_buf = [0u8; 8192];

    loop {
        let timestamp = now_instant();
        iface.poll(timestamp, &mut device, &mut sockets);

        // ----------------------------------------------------
        // Handle DHCP Link & IP Allocation
        // ----------------------------------------------------
        match controller.is_connected() {
            Ok(true) => {
                if !connected_logged {
                    println!("📶 Wi-Fi Layer Connected! Requesting DHCP IP...");
                    connected_logged = true;
                    let dhcp_sock = sockets.get_mut::<Dhcpv4Socket>(dhcp_handle);
                    dhcp_sock.reset();
                }

                let dhcp_sock = sockets.get_mut::<Dhcpv4Socket>(dhcp_handle);
                if let Some(event) = dhcp_sock.poll() {
                    match event {
                        DhcpEvent::Configured(config) => {
                            println!("🎉 IP Address Assigned by Router: {}", config.address);
                            if let Some(router) = config.router {
                                println!("   Gateway: {}", router);
                                gateway_ip = Some(router);
                            }
                            assigned_ip = Some(config.address);
                            iface.update_ip_addrs(|addrs| {
                                addrs.clear();
                                addrs.push(IpCidr::Ipv4(config.address)).ok();
                            });
                            if let Some(router) = config.router {
                                iface.routes_mut().add_default_ipv4_route(router).ok();
                            }
                            println!("🌐 LAN Web Dashboard online: http://{}", config.address.address());
                        }
                        DhcpEvent::Deconfigured => {
                            println!("⚠️ DHCP Deconfigured");
                            assigned_ip = None;
                            gateway_ip = None;
                        }
                    }
                }
            }
            Err(_) => {
                connected_logged = false;
            }
            _ => {}
        }

        // ----------------------------------------------------
        // Handle LAN HTTP Server (Dashboard & REST API)
        // ----------------------------------------------------
        let mut req_len = 0;
        {
            let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
            if http_sock.can_recv() {
                http_sock
                    .recv(|data| {
                        req_len = data.len().min(http_req_buf.len());
                        http_req_buf[..req_len].copy_from_slice(&data[..req_len]);
                        (data.len(), ())
                    })
                    .ok();
            }
        }

        if req_len > 0 {
            let req = parse_station_request(&http_req_buf[..req_len]);
            let mut should_reboot = false;

            match req {
                StationHttpRequest::GetDashboard => {
                    println!("📄 [HTTP] Serving LAN Dashboard");
                    let ip_str = assigned_ip
                        .map(|ip| format!("{}", ip.address()))
                        .unwrap_or_else(|| format!("Acquiring..."));
                    let gw_str = gateway_ip
                        .map(|gw| format!("{}", gw))
                        .unwrap_or_else(|| format!("Unknown"));

                    let html = generate_dashboard_html(
                        &creds.ssid,
                        &ip_str,
                        &gw_str,
                        current_target_mac,
                    );

                    let written = format_http_response(200, "text/html", &html, &mut http_resp_buf);
                    let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                    let _ = http_sock.send_slice(&http_resp_buf[..written]);
                    http_sock.close();
                }

                StationHttpRequest::Wake => {
                    println!("⚡ [WOL] Received Wake Request via LAN API");
                    let (status, body) = match current_target_mac {
                        Some(mac) => {
                            let packet = create_magic_packet(mac);
                            let bcast = IpEndpoint::new(
                                IpAddress::v4(255, 255, 255, 255),
                                WOL_PORT,
                            );

                            let wol_sock = sockets.get_mut::<UdpSocket>(wol_handle);
                            let _ = wol_sock.send_slice(&packet, bcast);
                            let mac_str = format_mac_address(mac);
                            println!("⚡ [WOL] Magic Packet broadcasted to {}", mac_str);

                            (
                                200,
                                format!(
                                    r#"{{"status":"ok","message":"Magic packet sent","mac":"{}"}}"#,
                                    mac_str
                                ),
                            )
                        }
                        None => (
                            400,
                            format!(
                                r#"{{"status":"error","message":"Target PC MAC address is not configured yet"}}"#
                            ),
                        ),
                    };

                    let written = format_http_response(
                        status,
                        "application/json",
                        &body,
                        &mut http_resp_buf,
                    );
                    let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                    let _ = http_sock.send_slice(&http_resp_buf[..written]);
                    http_sock.close();
                }

                StationHttpRequest::UpdateMac { mac_str } => {
                    println!("⚙️  [Config] Received Update Target MAC: '{}'", mac_str);
                    let (status, body) = match parse_mac_address(&mac_str) {
                        Some(mac) => {
                            current_target_mac = Some(mac);
                            let _ = storage.save_target_mac(Some(mac));
                            let formatted = format_mac_address(mac);
                            println!("💾 [Config] Target MAC saved: {}", formatted);
                            (
                                200,
                                format!(
                                    r#"{{"status":"ok","message":"Target MAC saved","mac":"{}"}}"#,
                                    formatted
                                ),
                            )
                        }
                        None => (
                            400,
                            format!(
                                r#"{{"status":"error","message":"Invalid MAC address format (expected XX:XX:XX:XX:XX:XX)"}}"#
                            ),
                        ),
                    };

                    let written = format_http_response(
                        status,
                        "application/json",
                        &body,
                        &mut http_resp_buf,
                    );
                    let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                    let _ = http_sock.send_slice(&http_resp_buf[..written]);
                    http_sock.close();
                }

                StationHttpRequest::ChangeWifi {
                    ssid,
                    password,
                    admin_pwd,
                } => {
                    println!("🔐 [Admin] Wi-Fi modification requested for SSID '{}'", ssid);
                    if admin_pwd == ADMIN_PASSWORD {
                        println!("✅ [Admin] Password verified! Saving new Wi-Fi credentials...");
                        if let Ok(new_creds) =
                            WifiCredentials::with_target_mac(&ssid, &password, current_target_mac)
                        {
                            let _ = storage.save_wifi(&new_creds);
                            let body = r#"{"status":"ok","message":"Wi-Fi updated! Rebooting..."}"#;
                            let written = format_http_response(
                                200,
                                "application/json",
                                body,
                                &mut http_resp_buf,
                            );
                            let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                            let _ = http_sock.send_slice(&http_resp_buf[..written]);
                            http_sock.close();
                            should_reboot = true;
                        }
                    } else {
                        println!("❌ [Admin] Invalid admin password attempt!");
                        let body = r#"{"status":"error","message":"Invalid admin password"}"#;
                        let written = format_http_response(
                            401,
                            "application/json",
                            body,
                            &mut http_resp_buf,
                        );
                        let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                        let _ = http_sock.send_slice(&http_resp_buf[..written]);
                        http_sock.close();
                    }
                }

                StationHttpRequest::ResetWifi { admin_pwd } => {
                    println!("🔐 [Admin] Factory Reset Wi-Fi requested");
                    if admin_pwd == ADMIN_PASSWORD {
                        println!("✅ [Admin] Password verified! Erasing Flash...");
                        let _ = storage.erase_wifi();
                        let body = r#"{"status":"ok","message":"Wi-Fi erased! Rebooting to Captive Portal..."}"#;
                        let written = format_http_response(
                            200,
                            "application/json",
                            body,
                            &mut http_resp_buf,
                        );
                        let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                        let _ = http_sock.send_slice(&http_resp_buf[..written]);
                        http_sock.close();
                        should_reboot = true;
                    } else {
                        println!("❌ [Admin] Invalid admin password for factory reset!");
                        let body = r#"{"status":"error","message":"Invalid admin password"}"#;
                        let written = format_http_response(
                            401,
                            "application/json",
                            body,
                            &mut http_resp_buf,
                        );
                        let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                        let _ = http_sock.send_slice(&http_resp_buf[..written]);
                        http_sock.close();
                    }
                }

                StationHttpRequest::Favicon => {
                    let written = format_http_response(204, "image/x-icon", "", &mut http_resp_buf);
                    let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                    let _ = http_sock.send_slice(&http_resp_buf[..written]);
                    http_sock.close();
                }

                StationHttpRequest::Preflight => {
                    // Respond to CORS preflight (OPTIONS) — required for Brave/Chrome private network
                    println!("🌐 [HTTP] CORS Preflight (OPTIONS) responded");
                    let written = format_http_response(204, "text/plain", "", &mut http_resp_buf);
                    let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                    let _ = http_sock.send_slice(&http_resp_buf[..written]);
                    http_sock.close();
                }

                StationHttpRequest::Other => {
                    let body = r#"{"status":"error","message":"Endpoint not found"}"#;
                    let written = format_http_response(
                        404,
                        "application/json",
                        body,
                        &mut http_resp_buf,
                    );
                    let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
                    let _ = http_sock.send_slice(&http_resp_buf[..written]);
                    http_sock.close();
                }
            }

            if should_reboot {
                delay.delay_millis(1500);
                esp_hal::reset::software_reset();
            }
        }

        // Re-arm HTTP listener immediately if closed or in lingering FIN/TIME wait states
        {
            let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
            match http_sock.state() {
                TcpState::Closed => {
                    let _ = http_sock.listen(80);
                }
                TcpState::CloseWait | TcpState::TimeWait => {
                    http_sock.abort();
                    let _ = http_sock.listen(80);
                }
                _ => {}
            }
        }

        loop_count += 1;
        if loop_count % 500 == 0 {
            if let Some(ip) = assigned_ip {
                println!(
                    "🦀 Power-Crab Active [IP: {}] - Dashboard & WOL Ready",
                    ip.address()
                );
            }
        }

        delay.delay_millis(10);
    }
}
