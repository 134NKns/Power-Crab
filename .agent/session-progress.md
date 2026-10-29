# Power-Crab Progress Log

- **Status**: Live & Verified on Hardware
- **ESP32 IP**: `192.168.1.117`
- **Features Completed & Tested**:
  1. Wi-Fi Captive Portal & Smart Boot logic
  2. Pure Rust Wake-on-LAN Magic Packet dispatcher
  3. LAN Device Discovery (Ethernet/ARP packet sniffer)
  4. Active LAN Subnet Sweep (`POST /api/scan-lan`)
  5. Live Target PC Status badge (`GET /api/status` - Online/Offline detection)
  6. Dynamic 2.4GHz Wi-Fi scanner in Station mode (`GET /api/wifi/scan`)
  7. Dropdown Wi-Fi selector in Web Dashboard
  8. One-click LAN Device Picker cards
  9. Local Shortcut & Webhook direct URL box for iOS Shortcuts/Siri
  10. Admin password (`pwd_1234`) protection for Wi-Fi change and reset
