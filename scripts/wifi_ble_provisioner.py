#!/usr/bin/env python3
"""
WiFi-over-BLE provisioner for Linux: sends STA credentials and reads status notifications.
Targets ESP-IDF BluFi (service 0xFFFF, TX 0xFF01, RX 0xFF02). Uses minimal BluFi frames
(no encryption/checksum). Requires: pip install bleak
"""
import asyncio
import argparse
import struct

try:
    import bleak
except ImportError:
    raise SystemExit("Install bleak: pip install bleak")

BLUFI_SVC = "0000ffff-0000-1000-8000-00805f9b34fb"
BLUFI_TX = "0000ff01-0000-1000-8000-00805f9b34fb"
BLUFI_RX = "0000ff02-0000-1000-8000-00805f9b34fb"

# BluFi type: control=0, data=1. Subtype in upper 6 bits.
def blufi_control_frame(subtype: int, data: bytes = b"", seq: int = 1) -> bytes:
    fc, checksum = 0, 0
    return struct.pack("<BBBB", (subtype << 2), fc, seq & 0xFF, len(data)) + data + struct.pack("<H", checksum)

def blufi_data_frame(subtype: int, data: bytes, seq: int = 1) -> bytes:
    fc, checksum = 0, 0
    return struct.pack("<BBBB", (subtype << 2) | 1, fc, seq & 0xFF, len(data)) + data + struct.pack("<H", checksum)

async def main():
    p = argparse.ArgumentParser(description="BLE provisioner: send WiFi credentials, read status")
    p.add_argument("--name", default="ESP32_BLUFI", help="BLE device name to scan for")
    p.add_argument("--ssid", required=True, help="STA SSID")
    p.add_argument("--password", default="", help="STA password")
    p.add_argument("--timeout", type=float, default=30.0, help="Scan/connect timeout (s)")
    args = p.parse_args()
    ssid = args.ssid.encode("utf-8")
    password = args.password.encode("utf-8")
    if len(ssid) > 32 or len(password) > 64:
        raise SystemExit("SSID max 32 bytes, password max 64 bytes")

    print("Scanning for", args.name, "...")
    devs = await bleak.BleakScanner.discover(timeout=args.timeout)
    device = next((d for d in devs if (d.name or "").strip() == args.name.strip()), None)
    if not device:
        raise SystemExit("Device not found. Ensure it is advertising and name matches.")

    print("Connecting to", device.address)
    client = bleak.BleakClient(device.address, timeout=args.timeout)
    await client.connect()
    seq = 1

    def on_notify(handle, data):
        print("Status notification:", data.hex(), "|", bytes(b if 32 <= b < 127 else ord(".") for b in data))

    await client.start_notify(BLUFI_RX, on_notify)

    # Minimal BluFi sequence: security mode (none) -> STA mode -> SSID -> password -> connect
    await client.write_gatt_char(BLUFI_TX, blufi_control_frame(0x01, bytes([0x00]), seq), response=True)
    seq += 1
    await client.write_gatt_char(BLUFI_TX, blufi_control_frame(0x02, bytes([0x01]), seq), response=True)
    seq += 1
    await client.write_gatt_char(BLUFI_TX, blufi_data_frame(0x02, ssid, seq), response=True)
    seq += 1
    await client.write_gatt_char(BLUFI_TX, blufi_data_frame(0x03, password, seq), response=True)
    seq += 1
    await client.write_gatt_char(BLUFI_TX, blufi_control_frame(0x03, b"", seq), response=True)
    print("Credentials and connect command sent. Waiting for status (Ctrl+C to stop).")

    await asyncio.sleep(15)
    await client.disconnect()
    print("Done.")

if __name__ == "__main__":
    asyncio.run(main())
