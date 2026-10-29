#!/usr/bin/env bash
set -e

PORT_URL="rfc2217://100.78.132.62:7777?ign_set_control"

echo "🦀 [Power-Crab] Monitoring ESP32 serial output at 115200 baud (Ctrl+C to exit)..."
python3 -c "
import serial
import time
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
