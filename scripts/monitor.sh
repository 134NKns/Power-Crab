#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -f "$DIR/.env" ]; then
    # shellcheck disable=SC1091
    source "$DIR/.env"
fi

PORT_URL="${ESP_PORT:-/dev/ttyUSB0}"

echo "🦀 [Power-Crab] Monitoring ESP32 serial output at 115200 baud on ($PORT_URL)..."
echo "ℹ️  Press Ctrl+C to exit."

python3 -c "
import serial
import sys

try:
    s = serial.serial_for_url('$PORT_URL', baudrate=115200, timeout=1)
    while True:
        line = s.readline()
        if line:
            sys.stdout.write(line.decode('utf-8', errors='replace'))
            sys.stdout.flush()
except KeyboardInterrupt:
    print('\nDisconnected.')
"
