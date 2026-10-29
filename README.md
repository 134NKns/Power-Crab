# 🦀 Power-Crab

[![Rust](https://img.shields.io/badge/Rust-no__std-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Hardware](https://img.shields.io/badge/Target-ESP32--D0WD--V3-blue?style=flat-square&logo=espressif)](https://www.espressif.com/)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](LICENSE)
[![Framework](https://img.shields.io/badge/Framework-esp--hal%20v0.23-red?style=flat-square)](https://github.com/esp-rs/esp-hal)

**Power-Crab** is an ultra-fast, bare-metal (`no_std`) Rust firmware for the ESP32 that turns your board into a dedicated, offline-first **Wake-on-LAN (WOL) Station & Remote Power Gateway**.

It features an automated Captive Portal for Wi-Fi setup, a sleek dark-mode LAN Web Dashboard, a robust REST API for local automation, and Flash memory persistence — completely offline, private, and requiring zero external cloud services.

---

## ✨ Features

- **🦀 Pure Bare-Metal Rust (`no_std`)**: Built on `esp-hal` (v0.23) and `smoltcp`. No heavy OS overhead, compiles in ~1.3 seconds, with instant sub-second boot times.
- **📱 Zero-Touch Captive Portal (`POWER-CRAB`)**:
  - Automatically pops up setup page on iOS, Android, macOS, and Windows.
  - Built-in RFC 2131 DHCP Server (UDP 67) & DNS Hijacker (UDP 53).
  - Scans nearby Wi-Fi networks in real-time and provides a dropdown selector.
  - Verifies credentials live before concluding setup.
- **⚡ Wake-on-LAN (WOL) Engine**:
  - Generates RFC-compliant 102-byte Magic Packets (`0xFF` × 6 + `MAC` × 16).
  - Broadcasts across local subnet on UDP Port 9.
- **🌐 Responsive LAN Web Dashboard**:
  - Dark-mode minimal UI served directly on port 80.
  - One-click **⚡ WAKE UP PC** button.
  - Live status indicator (SSID, local IP, Gateway, Target PC MAC).
  - Web-based Target MAC configuration without Wi-Fi resets.
- **🔐 Protected Admin Controls**:
  - Sensitive operations (changing Wi-Fi credentials or factory reset) require the admin password (`pwd_1234`).
- **💾 Flash Persistence**:
  - Saved credentials and target MAC reside in Flash sector `0x3FF000` with magic headers and XOR checksums.
  - Survives reboots and power outages.

---

## 🏗️ Architecture

```
                                +---------------------------+
                                |      ESP32 Hardware       |
                                |  (esp-hal no_std, 240MHz) |
                                +-------------+-------------+
                                              |
                   +--------------------------+--------------------------+
                   |                                                     |
         [Flash Sector 0x3FF000]                               [Wi-Fi Radio Mixed Mode]
         - SSID & Password                                               |
         - Target PC MAC                               +-----------------+-----------------+
         - Checksum & Version                          |                                   |
                                               [Captive Portal Mode]               [Station (STA) Mode]
                                               - SoftAP "POWER-CRAB"               - Connects to Home Wi-Fi
                                               - DHCP Server (UDP 67)              - DHCP Client (Gets Router IP)
                                               - DNS Hijacker (UDP 53)             - LAN Dashboard (TCP 80)
                                               - Provisioning UI                   - WOL Dispatcher (UDP 9)
```

---

## 📡 LAN REST API Reference

When connected to your local network, Power-Crab exposes a clean JSON REST API on port 80:

### 1. Trigger Wake-on-LAN
Wakes the configured target PC by broadcasting an RFC Magic Packet.
- **Endpoint**: `POST /api/wake` or `GET /wake`
- **Response**:
  ```json
  {
    "status": "ok",
    "message": "Magic packet sent",
    "mac": "58:D3:12:AD:1C:E8"
  }
  ```
- **Example**:
  ```bash
  curl -X POST http://192.168.1.117/api/wake
  # or in browser / iOS Shortcuts:
  http://192.168.1.117/wake
  ```

---

### 2. Update Target PC MAC Address
Updates the target computer's physical MAC address in Flash storage.
- **Endpoint**: `POST /api/config`
- **Headers**: `Content-Type: application/x-www-form-urlencoded`
- **Body**: `mac=AA:BB:CC:DD:EE:FF`
- **Response**:
  ```json
  {
    "status": "ok",
    "message": "Target MAC saved",
    "mac": "AA:BB:CC:DD:EE:FF"
  }
  ```

---

### 3. Change Wi-Fi Network (Protected)
Changes the Wi-Fi network and reboots into Station Mode. Requires admin password.
- **Endpoint**: `POST /api/wifi`
- **Headers**: `Content-Type: application/x-www-form-urlencoded`
- **Body**: `admin=pwd_1234&ssid=MyNetwork&password=secretpassword`
- **Response**:
  ```json
  {
    "status": "ok",
    "message": "Wi-Fi updated! Rebooting..."
  }
  ```

---

### 4. Factory Reset (Protected)
Erases saved Wi-Fi and restarts into `POWER-CRAB` Captive Portal mode.
- **Endpoint**: `POST /api/reset`
- **Headers**: `Content-Type: application/x-www-form-urlencoded`
- **Body**: `admin=pwd_1234`
- **Response**:
  ```json
  {
    "status": "ok",
    "message": "Wi-Fi erased! Rebooting to Captive Portal..."
  }
  ```

---

## 🚀 Getting Started

### Prerequisites

1. **Rust with Xtensa support**: Install via [`espup`](https://github.com/esp-rs/espup):
   ```bash
   cargo install espup
   espup install
   source "$HOME/export-esp.sh"
   ```
2. **ESPFlash / esptool**:
   ```bash
   cargo install espflash
   ```

### Building & Flashing

```bash
# Clone the repository
git clone git@github.com:134NKns/Power-Crab.git
cd Power-Crab

# Source Xtensa environment
source ~/export-esp.sh

# Build release firmware
cargo +esp build --release

# Flash via USB serial (replace /dev/ttyUSB0 with your port)
espflash flash target/xtensa-esp32-none-elf/release/power-crab --port /dev/ttyUSB0 --monitor
```

---

## 📁 Project Structure

```
src/
├── lib.rs              # Library root and re-exports
├── main.rs             # Boot orchestrator (< 65 lines)
│
├── traits/             # Abstract interfaces
│   └── storage.rs      # WifiStorage trait for testability
│
├── storage/            # Flash memory persistence (Sector 0x3FF000)
│   ├── creds.rs        # WifiCredentials data structure
│   └── flash.rs        # NOR Flash driver implementing WifiStorage
│
├── wol/                # Wake-on-LAN subsystem
│   ├── packet.rs       # 102-byte Magic packet generator & MAC parser
│   └── mod.rs          # WOL module definitions
│
├── portal/             # Captive Portal subsystem (Hotspot)
│   ├── dhcp.rs         # RFC 2131 DHCP server (UDP 67)
│   ├── dns.rs          # DNS hijacker (UDP 53)
│   ├── web.rs          # Offline HTML provisioning UI & HTTP parser (TCP 80)
│   └── mod.rs          # Portal coordinator
│
├── station/            # Station Mode subsystem (Home Wi-Fi)
│   ├── web.rs          # LAN Web Dashboard & REST API
│   └── mod.rs          # Station connection coordinator & DHCP client
│
└── net/                # Network time utilities
    └── time.rs         # System time conversion for smoltcp
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
