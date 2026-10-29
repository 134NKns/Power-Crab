# 🦀 Power-Crab

Bare-metal (`no_std`) Rust firmware for ESP32 with ultra-fast builds and remote flashing bridge.

## Architecture

- **Framework**: Pure Rust `no_std` via [`esp-hal`](https://github.com/esp-rs/esp-hal) (v0.23).
- **Target**: `xtensa-esp32-none-elf` (ESP32-D0WD-V3, Dual-Core 240MHz).
- **Toolchain**: `esp` toolchain managed via `espup`.

## Remote Flashing Bridge

Flashing is managed over Tailscale to the ESP32 plugged into the Windows host (`CODER`):
- Server: `rfc2217://100.78.132.62:7777?ign_set_control`
- Helper Tool: `/Users/134nk/Code/coder-connect` (`status`, `start`, `stop`, `restart`, `test`)

## Quick Commands

### Build & Flash
```bash
./scripts/flash.sh
```

### Serial Monitor
```bash
./scripts/monitor.sh
```
