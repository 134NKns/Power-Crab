#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

# Ensure ESP toolchain environment variables are set
if [ -f "/Users/134nk/export-esp.sh" ]; then
    source "/Users/134nk/export-esp.sh"
fi

PORT_URL="rfc2217://100.78.132.62:7777?ign_set_control"

echo "🦀 [Power-Crab] Building release binary with esp-hal..."
cargo +esp build --release

echo "📦 [Power-Crab] Packaging merged firmware image..."
cargo +esp espflash save-image --chip esp32 --merge --skip-padding --ignore-app-descriptor --release target/power-crab.bin

echo "⚡ [Power-Crab] Flashing firmware to ESP32 over RFC2217 ($PORT_URL)..."
esptool.py --port "$PORT_URL" write_flash 0x0 target/power-crab.bin

echo "✅ [Power-Crab] Flash complete!"
