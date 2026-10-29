#![no_std]
#![no_main]

extern crate alloc;

use esp_backtrace as _;
use esp_hal::{clock::CpuClock, delay::Delay, main, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;
use esp_wifi::wifi::{
    AccessPointConfiguration, AuthMethod, ClientConfiguration, Configuration, WifiStaDevice,
};
use power_crab::{
    dhcp::{self, DHCP_CLIENT_PORT},
    dns,
    storage::{Storage, WifiCredentials},
    web,
};
use smoltcp::{
    iface::{SocketSet, SocketStorage},
    socket::{
        dhcpv4::{Event as DhcpEvent, Socket as Dhcpv4Socket},
        tcp::{Socket as TcpSocket, SocketBuffer as TcpSocketBuffer},
        udp::{PacketBuffer as UdpPacketBuffer, PacketMetadata, Socket as UdpSocket},
    },
    wire::{IpAddress, IpCidr, IpEndpoint, Ipv4Address, Ipv4Cidr},
};

fn now_instant() -> smoltcp::time::Instant {
    smoltcp::time::Instant::from_micros(
        esp_hal::time::now().duration_since_epoch().to_micros() as i64,
    )
}

#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(72 * 1024);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let delay = Delay::new();

    println!();
    println!("========================================");
    println!("🦀 Hello, Power-Crab (esp-hal bare-metal)");
    println!("========================================");

    let mut storage = Storage::new();
    let saved_wifi = match storage.load_wifi() {
        Ok(Some(creds)) => {
            println!(
                "💾 Found saved Wi-Fi in Flash: '{}' (pass_len: {})",
                creds.ssid,
                creds.password.len()
            );
            Some(creds)
        }
        Ok(None) => {
            println!("⚠️ No Wi-Fi saved in Flash -> Starting Captive Portal");
            None
        }
        Err(e) => {
            println!("❌ Storage error: {:?}", e);
            None
        }
    };

    println!("📡 Initializing Wi-Fi radio...");
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let init = esp_wifi::init(
        timg0.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    // --------------------------------------------------------
    // CASE 1: Wi-Fi credentials exist -> Normal Station (STA) Mode
    // --------------------------------------------------------
    if let Some(creds) = saved_wifi {
        println!("🚀 Starting in Station Mode (Connecting to '{}')...", creds.ssid);
        println!("ℹ️  SoftAP 'POWER-CRAB' is disabled.");

        let (mut iface, mut device, mut controller) =
            esp_wifi::wifi::utils::create_network_interface(&init, peripherals.WIFI, WifiStaDevice)
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

    // --------------------------------------------------------
    // CASE 2: No Wi-Fi saved -> Captive Portal Setup Mode
    // --------------------------------------------------------
    println!("📡 Starting Captive Portal: 'POWER-CRAB'...");
    run_captive_portal(&init, peripherals.WIFI, &delay, &mut storage);
}

fn run_captive_portal(
    init: &esp_wifi::EspWifiController<'_>,
    wifi_peripheral: esp_hal::peripherals::WIFI,
    delay: &Delay,
    storage: &mut Storage,
) -> ! {
    let ap_sta =
        esp_wifi::wifi::utils::create_ap_sta_network_interface(init, wifi_peripheral).unwrap();

    let mut iface = ap_sta.ap_interface;
    let mut device = ap_sta.ap_device;
    let mut controller = ap_sta.controller;

    // Set static IP for ESP32 AP: 192.168.4.1/24
    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(IpCidr::Ipv4(Ipv4Cidr::new(
                Ipv4Address::new(192, 168, 4, 1),
                24,
            )))
            .unwrap();
    });

    println!("📡 Configuring SoftAP: 'POWER-CRAB' (Mixed AP+STA mode)...");
    let ap_config = AccessPointConfiguration {
        ssid: "POWER-CRAB".try_into().unwrap(),
        ssid_hidden: false,
        channel: 1,
        auth_method: AuthMethod::None,
        password: "".try_into().unwrap(),
        max_connections: 4,
        ..Default::default()
    };
    let sta_config = ClientConfiguration::default();
    let mixed_config = Configuration::Mixed(sta_config, ap_config.clone());

    controller.set_configuration(&mixed_config).unwrap();
    controller.start().unwrap();

    println!("✅ SoftAP 'POWER-CRAB' active! Gateway: 192.168.4.1");

    println!("🔍 Scanning for nearby Wi-Fi networks...");
    let scanned_ssids = match controller.scan_n::<15>() {
        Ok((aps, count)) => {
            println!("✅ Found {} Wi-Fi networks:", count);
            let mut list = heapless::Vec::<heapless::String<32>, 15>::new();
            for ap in &aps {
                println!("   - SSID: '{}' (RSSI: {} dBm)", ap.ssid, ap.signal_strength);
                if !ap.ssid.is_empty() && !list.contains(&ap.ssid) {
                    list.push(ap.ssid.clone()).ok();
                }
            }
            list
        }
        Err(e) => {
            println!("⚠️ Scan error: {:?}", e);
            heapless::Vec::new()
        }
    };

    let portal_html = web::generate_portal_html(&scanned_ssids);

    // Initialize Network Sockets:
    // 1. DHCP Server Socket (UDP port 67)
    let mut dhcp_rx_meta = [PacketMetadata::EMPTY; 4];
    let mut dhcp_rx_data = [0u8; 1024];
    let mut dhcp_tx_meta = [PacketMetadata::EMPTY; 4];
    let mut dhcp_tx_data = [0u8; 1024];
    let mut dhcp_socket = UdpSocket::new(
        UdpPacketBuffer::new(&mut dhcp_rx_meta[..], &mut dhcp_rx_data[..]),
        UdpPacketBuffer::new(&mut dhcp_tx_meta[..], &mut dhcp_tx_data[..]),
    );
    dhcp_socket.bind(dhcp::DHCP_SERVER_PORT).unwrap();

    // 2. DNS Hijack Socket (UDP port 53)
    let mut dns_rx_meta = [PacketMetadata::EMPTY; 4];
    let mut dns_rx_data = [0u8; 512];
    let mut dns_tx_meta = [PacketMetadata::EMPTY; 4];
    let mut dns_tx_data = [0u8; 512];
    let mut dns_socket = UdpSocket::new(
        UdpPacketBuffer::new(&mut dns_rx_meta[..], &mut dns_rx_data[..]),
        UdpPacketBuffer::new(&mut dns_tx_meta[..], &mut dns_tx_data[..]),
    );
    dns_socket.bind(dns::DNS_PORT).unwrap();

    // 3. HTTP Server Socket (TCP port 80)
    let mut http_rx_data = [0u8; 2048];
    let mut http_tx_data = [0u8; 4096];
    let mut http_socket = TcpSocket::new(
        TcpSocketBuffer::new(&mut http_rx_data[..]),
        TcpSocketBuffer::new(&mut http_tx_data[..]),
    );
    http_socket.listen(80).unwrap();

    // Allocate sockets inside a SocketSet
    let mut socket_storage = [SocketStorage::EMPTY; 3];
    let mut sockets = SocketSet::new(&mut socket_storage[..]);

    let dhcp_handle = sockets.add(dhcp_socket);
    let dns_handle = sockets.add(dns_socket);
    let http_handle = sockets.add(http_socket);

    println!("🌐 Captive Portal services ready:");
    println!("   - DHCP Server on UDP 67 (Offering 192.168.4.2)");
    println!("   - DNS Hijacker on UDP 53 (Redirecting to 192.168.4.1)");
    println!("   - HTTP Web Server on TCP 80 (Serving Setup UI)");

    let server_ip = [192, 168, 4, 1];
    let offered_ip = [192, 168, 4, 2];
    let mut dhcp_resp_buf = [0u8; 512];
    let mut dns_buf = [0u8; 512];
    let mut http_resp_buf = [0u8; 4096];

    loop {
        let timestamp = now_instant();
        iface.poll(timestamp, &mut device, &mut sockets);

        // ----------------------------------------------------
        // 1. Process DHCP requests
        // ----------------------------------------------------
        {
            let dhcp_sock = sockets.get_mut::<UdpSocket>(dhcp_handle);
            if dhcp_sock.can_recv() {
                if let Ok((data, _endpoint)) = dhcp_sock.recv() {
                    if let Some(resp_len) =
                        dhcp::process_dhcp_packet(data, &mut dhcp_resp_buf, server_ip, offered_ip)
                    {
                        println!("🏷️  [DHCP] Handing out IP 192.168.4.2 to client");
                        let bcast_endpoint =
                            IpEndpoint::new(IpAddress::v4(255, 255, 255, 255), DHCP_CLIENT_PORT);
                        let _ = dhcp_sock.send_slice(&dhcp_resp_buf[..resp_len], bcast_endpoint);
                    }
                }
            }
        }

        // ----------------------------------------------------
        // 2. Process DNS Hijack queries (UDP 53)
        // ----------------------------------------------------
        {
            let dns_sock = sockets.get_mut::<UdpSocket>(dns_handle);
            if dns_sock.can_recv() {
                if let Ok((data, endpoint)) = dns_sock.recv() {
                    let len = data.len().min(dns_buf.len());
                    dns_buf[..len].copy_from_slice(&data[..len]);
                    if let Some(resp_len) = dns::process_dns_query(&mut dns_buf, len, server_ip) {
                        println!("🌐 [DNS] Hijacked probe query -> 192.168.4.1");
                        let _ = dns_sock.send_slice(&dns_buf[..resp_len], endpoint);
                    }
                }
            }
        }

        // ----------------------------------------------------
        // 3. Process HTTP Server requests (TCP 80)
        // ----------------------------------------------------
        let mut maybe_submit_creds: Option<WifiCredentials> = None;
        {
            let http_sock = sockets.get_mut::<TcpSocket>(http_handle);
            if http_sock.can_recv() {
                let mut req_len = 0;
                http_sock
                    .recv(|data| {
                        req_len = data.len().min(http_resp_buf.len());
                        http_resp_buf[..req_len].copy_from_slice(&data[..req_len]);
                        (data.len(), ())
                    })
                    .ok();

                if req_len > 0 {
                    match web::parse_http_request(&http_resp_buf[..req_len]) {
                        web::HttpRequest::SubmitCredentials(creds) => {
                            maybe_submit_creds = Some(creds);
                        }
                        web::HttpRequest::GetPortal | web::HttpRequest::Other => {
                            println!("📄 [HTTP] Serving Portal Page");
                            let mut resp_out = [0u8; 4096];
                            let written = web::format_http_response(&portal_html, &mut resp_out);
                            let _ = http_sock.send_slice(&resp_out[..written]);
                            http_sock.close();
                        }
                    }
                }
            }

            if !http_sock.is_active() && !http_sock.is_listening() {
                let _ = http_sock.listen(80);
            }
        }

        // If credentials were submitted, test connection:
        if let Some(creds) = maybe_submit_creds {
            println!("🔄 Testing connection to '{}'...", creds.ssid);

            let sta_config = ClientConfiguration {
                ssid: creds.ssid.clone(),
                password: creds.password.clone(),
                auth_method: if creds.password.is_empty() {
                    AuthMethod::None
                } else {
                    AuthMethod::WPA2Personal
                },
                ..Default::default()
            };

            let mixed_config = Configuration::Mixed(sta_config, ap_config.clone());
            let _ = controller.set_configuration(&mixed_config);
            let _ = controller.connect();

            // Wait up to 8 seconds while polling network interface
            let start_time = esp_hal::time::now();
            let mut connected = false;

            while esp_hal::time::now().duration_since_epoch().to_millis()
                - start_time.duration_since_epoch().to_millis()
                < 8000
            {
                let timestamp = now_instant();
                iface.poll(timestamp, &mut device, &mut sockets);

                match controller.is_connected() {
                    Ok(true) => {
                        connected = true;
                        break;
                    }
                    Err(_) => {}
                    _ => {}
                }
                delay.delay_millis(50);
            }

            let mut resp_out = [0u8; 4096];
            let http_sock = sockets.get_mut::<TcpSocket>(http_handle);

            if connected {
                println!("✅ Connected to '{}' successfully!", creds.ssid);
                println!("💾 Saving credentials to Flash...");
                let _ = storage.save_wifi(&creds);

                let result_html = web::generate_result_html(&creds.ssid, true, None);
                let written = web::format_http_response(&result_html, &mut resp_out);
                let _ = http_sock.send_slice(&resp_out[..written]);
                http_sock.close();

                println!("🦀 System ready! Rebooting to Station mode in 3 seconds...");
                delay.delay_millis(3000);
                esp_hal::reset::software_reset();
            } else {
                println!(
                    "❌ Connection to '{}' failed (password incorrect or unreachable)",
                    creds.ssid
                );
                let result_html = web::generate_result_html(
                    &creds.ssid,
                    false,
                    Some("Incorrect password or Wi-Fi network unreachable"),
                );
                let written = web::format_http_response(&result_html, &mut resp_out);
                let _ = http_sock.send_slice(&resp_out[..written]);
                http_sock.close();

                // Reset STA state so user can retry
                let _ = controller.disconnect();
            }
        }
    }
}
