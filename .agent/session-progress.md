# 🦀 Power-Crab: Agent Handover & Progress Notes

Last Updated: `2026-10-30 01:10:00 +07:00`

---

## 1. Project Context & Environment

- **Repository**: `Power-Crab` (`git@github.com:134NKns/Power-Crab.git`)
- **Local Directory**: `/Users/134nk/Code/Projects/free-wifi`
- **Target Hardware**: ESP32-D0WD-V3 (revision v3.1, Xtensa dual-core 240MHz, 4MB Flash)
- **Framework**: **Pure Rust `no_std`** using `esp-hal` (v0.23).
  > **Note**: Full C `esp-idf` was intentionally removed because of slow build times. `esp-hal` builds in ~1.3 seconds.
- **Xtensa Toolchain**: Sourced via `/Users/134nk/export-esp.sh` (`cargo +esp`).

---

## 2. Remote Hardware Bridge (CODER <-> Mac)

The ESP32 is physically plugged into **`COM3` on Windows PC `CODER`**, not on the Mac.
- **Tailscale IP**: `100.78.132.62`
- **RFC2217 Port**: `7777` (`rfc2217://100.78.132.62:7777?ign_set_control`)
- **Server Manager**: `/Users/134nk/Code/coder-connect`
  - Usage: `/Users/134nk/Code/coder-connect status | start | stop | restart | test`
- **Project Helper Scripts**:
  - [`./scripts/flash.sh`](file:///Users/134nk/Code/Projects/free-wifi/scripts/flash.sh): Builds release binary, merges image, and flashes via RFC2217.
  - [`./scripts/monitor.sh`](file:///Users/134nk/Code/Projects/free-wifi/scripts/monitor.sh): Reads serial output at 115200 baud.

---

## 3. Work Completed in This Session

1. **Replaced ESP-IDF with `esp-hal` (`no_std`)**:
   - Binary size reduced to ~146 KB (compressed ~35 KB flash payload).
   - Flashes across Tailscale in ~4.4 seconds.
2. **Removed Physical Motherboard Power Switch & GPIO Code**:
   - Removed `src/esp32_control/` and `src/traits/` as physical motherboard wiring (`PWR_SW`) is not needed.
   - Cleaned `src/lib.rs` and `src/main.rs`: removed all GPIO/LED code, keeping a clean baseline heartbeat loop without external pin bindings.
3. **Installed & Verified Wi-Fi, Storage & HTTP Server Dependencies**:
   - `esp-wifi = { version = "0.12", features = ["esp32", "wifi", "log"] }`
   - `esp-storage = { version = "0.4", features = ["esp32", "nor-flash"] }`
   - `esp-alloc = "0.6"`
   - `picoserve = { version = "0.20", features = ["alloc", "log"] }`
   - `heapless = "0.8"`
   - Updated `.cargo/config.toml` with `build-std = ["core", "alloc"]`.
   - Verified clean compilation with `cargo +esp check --release`.

4. **Implemented Flash Storage & Captive Portal Modules**:
   - [`src/storage.rs`](file:///Users/134nk/Code/Projects/free-wifi/src/storage.rs): Checks, saves, and erases Wi-Fi credentials in flash sector 0x3FF000 with magic header & XOR checksum.
   - [`src/web.rs`](file:///Users/134nk/Code/Projects/free-wifi/src/web.rs): Offline responsive setup HTML form (`PORTAL_HTML`), success response, and `no_std` URL/form decoder.
   - [`src/dns.rs`](file:///Users/134nk/Code/Projects/free-wifi/src/dns.rs): DNS hijacking responder on UDP 53 to redirect all domain checks to `192.168.4.1`.

5. **Full Captive Portal (DHCP + DNS + HTTP) Live on ESP32**:
   - SoftAP SSID: **`POWER-CRAB`** (Open, Channel 1)
   - Mixed AP+STA Mode: scans nearby Wi-Fi networks and renders a `<select>` dropdown dynamically.
   - Gateway & DNS: `192.168.4.1`
   - DHCP Server on UDP 67 (hands out `192.168.4.2` to connecting clients)
   - DNS Hijacker on UDP 53 (redirects all OS captive probes to `192.168.4.1`)
   - HTTP Web Server on TCP 80 (serves offline responsive dark setup form)
   - Live Connection Testing & Result Screen:
     - On form submit (`POST /save`), tests real connection to selected Wi-Fi.
     - If success: saves to flash, shows green "✅ Connected Successfully!" result page, and restarts.
     - If failed (wrong password / unreachable): shows red "❌ Connection Failed!" page with explanation and "🔄 Try Again" button.

6. **Smart Boot Lifecycle (Verified in Hardware)**:
   - On boot, checks Flash sector `0x3FF000`.
   - If valid credentials found:
     - Directly enters **Station Mode (STA)** to connect to target AP (`HELLO_WORLD_2.4G`).
     - **SoftAP `POWER-CRAB` is disabled** — no hotspot broadcast, no captive portal shown.
     - Acquires local IP via DHCP (`192.168.1.117`).
     - Ready for Wake-on-LAN operations.
   - If no credentials found (or empty):
     - Starts Captive Portal (`POWER-CRAB`), serves provisioning UI, tests credentials live, saves to flash, and reboots.

7. **Clean Modular Folder & Trait Architecture**:
   - Refactored monolith code into separate focused domain directories:
     - [`src/traits/`](file:///Users/134nk/Code/Projects/free-wifi/src/traits/): `WifiStorage` abstraction for mockability & decoupling.
     - [`src/storage/`](file:///Users/134nk/Code/Projects/free-wifi/src/storage/): `creds.rs` (data models) and `flash.rs` (ESP32 NOR flash implementation).
     - [`src/portal/`](file:///Users/134nk/Code/Projects/free-wifi/src/portal/): `dhcp.rs` (RFC 2131 server), `dns.rs` (hijacker), `web.rs` (responsive HTML & parser), `mod.rs` (CaptivePortal runner).
     - [`src/station/`](file:///Users/134nk/Code/Projects/free-wifi/src/station/): Station mode connection management & DHCP loop.
     - [`src/net/`](file:///Users/134nk/Code/Projects/free-wifi/src/net/): Network time utilities (`now_instant`).
     - [`src/main.rs`](file:///Users/134nk/Code/Projects/free-wifi/src/main.rs): Clean boot orchestrator reduced from 444 lines to ~64 lines.
   - Tested and verified on physical ESP32 hardware via RFC2217.

---

## 4. Next Steps: Wake-on-LAN (WOL) Service

1. **Station Mode Connection Timeout / Fallback**:
   - If Wi-Fi fails to connect after N attempts (e.g., router offline or SSID/pass changed), fallback to `POWER-CRAB` Captive Portal so the device is never bricked.
2. **Wake-on-LAN (WOL) Dispatcher**:
   - Construct UDP Magic Packet (`0xFF * 6` + `Target_MAC * 16`).
   - Broadcast to `255.255.255.255:9` or subnet directed broadcast `192.168.1.255:9`.
3. **HTTP REST Trigger**:
   - Listen on port 80 or 8080 when connected to home LAN (`http://192.168.1.117/wake`) to trigger WOL on demand.
