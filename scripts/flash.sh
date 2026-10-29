#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

# Source local .env if present
if [ -f "$DIR/.env" ]; then
    # shellcheck disable=SC1091
    source "$DIR/.env"
fi

# Ensure ESP toolchain environment variables are set
if [ -f "$HOME/export-esp.sh" ]; then
    # shellcheck disable=SC1091
    source "$HOME/export-esp.sh"
elif [ -f "/export-esp.sh" ]; then
    # shellcheck disable=SC1091
    source "/export-esp.sh"
fi

PORT_URL="${ESP_PORT:-/dev/ttyUSB0}"

echo "🦀 [Power-Crab] Building release binary with esp-hal..."
cargo +esp build --release

echo "📦 [Power-Crab] Packaging merged firmware image..."
cargo +esp espflash save-image --chip esp32 --merge --skip-padding --ignore-app-descriptor --release target/power-crab.bin

echo "⚡ [Power-Crab] Flashing firmware to ESP32 over ($PORT_URL)..."
esptool.py --port "$PORT_URL" write_flash 0x0 target/power-crab.bin

echo "✅ [Power-Crab] Flash complete!"
