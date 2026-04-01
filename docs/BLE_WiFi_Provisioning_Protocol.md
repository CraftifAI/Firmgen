# BLE-Based WiFi Provisioning Protocol (Design)

Compact specification for embedded device ↔ mobile app WiFi provisioning over GATT. Prioritizes reliability and low power; MTU 247 typical, fallback 23.

---

## 1. GATT Service and Characteristics

### Service

| Item | Value |
|------|--------|
| **Service UUID** | `0xFEED` (custom) or use 128-bit e.g. `0000feed-0000-1000-8000-00805f9b34fb` |
| **Primary** | Yes |

### Characteristics

| Name | UUID | Type | Properties | Max length (bytes) | Notify/Indicate | Notes |
|------|------|------|------------|--------------------|-----------------|-------|
| **Control** | `0xFE01` | Write (with response) + Read | Read, Write | 20 | — | Command/response; idempotent where possible |
| **Status** | `0xFE02` | Variable | Read, Notify | 64 | Notify | Provisioning state, errors; device → app |
| **WiFi SSID** | `0xFE03` | UTF-8 string | Write | 32 | — | SSID only; long SSIDs use fragmentation (see §3) |
| **WiFi Password** | `0xFE04` | UTF-8 / opaque | Write | 64 | — | Credential; fragment if > MTU-3 |
| **Security / Options** | `0xFE05` | Binary | Read, Write | 16 | — | auth (1), cipher (1), TLS/OTP flags (1), reserved |
| **Scan Results** | `0xFE06` | Binary/JSON | Read | 244 | Indicate | Device → app; chunked via Control |
| **Scan Control** | `0xFE07` | Write | Write | 8 | — | Start/stop scan, request next chunk |

- **ATT_MTU**: Negotiate 247; device and app must support 23 (20-byte payload per write/notify).
- **Max lengths** are logical; actual transfer uses chunk size `min(MTU-3, max_length)` per PDU.
- **Notifications** for Status (lower power than Indication where OK); **Indications** for Scan Results so app acknowledges and device can pace.

---

## 2. State Machines

### Device state machine

```
                    +------------------+
                    |   IDLE/ADV       |<-----------------------------------+
                    +--------+---------+                                    |
                             | connect                                      |
                             v                                              |
                    +------------------+  disconnect   +-------------+       |
                    |   CONNECTED      |-------------->| DISCONNECT  |-------+
                    +--------+---------+               +-------------+
                             | GATT ready
                             v
                    +------------------+  timeout(30s) / cancel
                    |   PROV_IDLE      |<-----------------------------------+
                    +--------+---------+                                    |
         +------------------+------------------+                           |
         |                  |                  |                            |
         v                  v                  v                            |
  +------------+   +----------------+   +-------------+                     |
  | SCAN_REQ   |   | CREDENTIALS    |   | (optional)  |                     |
  | (scanning) |   | (expect SSID/  |   | TLS/OTP     |                     |
  +-----+------+   |  password)     |   +------+------+                     |
        |          +--------+-------+          |                            |
        | scan done         |                   |                            |
        v                   v                  v                            |
  +------------+     +-------------+    +-------------+                      |
  | SCAN_SEND  |     | CONNECTING  |    | (flow)      |                      |
  | (chunks)   |     | (WiFi assoc)|    +------+------+                      |
  +-----+------+     +------+------+           |                             |
        | ack last           |                 +-----------------------------+
        v                    v success/fail
  back to PROV_IDLE    +-------------+
                       | PROV_DONE   |-----> notify status, then PROV_IDLE
                       | or PROV_ERR |       (or disconnect)
                       +-------------+
```

- **Timeouts**: Control/credential idle 30 s → PROV_IDLE; scan 15 s → PROV_IDLE; WiFi connect 45 s → PROV_ERR.
- **Retries**: Control write failure: app retries up to 3× with 500 ms backoff. Device does not retry Notify; uses Indication for Scan so delivery is confirmed.

### Mobile app state machine

```
  DISCOVERED --> BONDED/CONNECTED --> GATT_READY --> PROV_READY
       |                |                  |               |
       |                |                  |    +----------+----------+
       |                |                  |    |          |          |
       |                |                  |    v          v          v
       |                |                  | REQUEST_   SEND_     WAIT_
       |                |                  | SCAN       CREDS     STATUS
       |                |                  |    |          |          |
       |                |                  |    v          v          v
       |                |                  | SCAN_       CREDS_    PROV_
       |                |                  | RECV        SENT      DONE/ERR
       |                |                  |    |          |          |
       |                +------------------+----+----------+----------+
       |                disconnect / error      back to PROV_READY
       +-----------------+
```

- **Timeouts**: Wait for Status after credentials 45 s; wait for scan chunk 5 s.
- **Retries**: 3× for Control/Write with 500 ms backoff; on Indication timeout request same chunk again (once).

---

## 3. Packetization and MTU

- **Effective payload per ATT PDU**: `MTU - 3` (e.g. 244 for MTU 247, 20 for MTU 23).
- **Strategy**: Application-level fragmentation with a 2-byte header in the first chunk of a multi-chunk value:
  - Byte 0: `0x00` = only/first chunk, `0x01` = continuation.
  - Byte 1: chunk index (0-based) for continuation; for first chunk, total chunks (1–255) or 0 meaning “single chunk”.
- **Reassembly**: App/device buffers chunks; on last chunk (index == total-1) or single-chunk, process payload. Discard and optionally NACK on gap (e.g. via Control).
- **Recommendations**:
  - Negotiate MTU 247 as soon as connection is established; fallback to 23 if negotiation fails.
  - Use **Write with response** for Control, SSID, Password, Security, Scan Control so each PDU is acknowledged.
  - For Scan Results use **Indication** (not Notify) so device knows when to send next chunk; cap chunk size at `MTU-3`.
  - Prefer one logical “message” per characteristic write; fragment only when length > (MTU-3). No interleaving of different logical messages on the same characteristic.

---

## 4. UX and Error Handling (Mobile App)

- **Scan**: Trigger scan via Control or Scan Control; show list from Scan Results (SSID, RSSI, security). Allow refresh; show “scanning…” and timeout after 15 s.
- **Select SSID**: User picks one; if security is WPA2/WPA3 show password field; if Open, skip password. Optional: show “Security: WPA2” from Security/Options.
- **TLS/OTP**: If device indicates TLS or OTP capability in Security/Options, show optional “Certificate” or “One-time code” path; keep flow linear (e.g. credentials → then optional TLS/OTP step).
- **Errors**: Map Status codes to user messages (e.g. “Wrong password”, “Network not found”, “Timeout”). On BLE disconnect during prov, “Connection lost; try again.”
- **Message order**: (1) Ensure GATT ready and optional MTU exchange. (2) Request scan → consume Scan Results. (3) User selects SSID → write Security/Options if needed → write SSID → write Password. (4) Write Control “start connect”. (5) Subscribe/wait for Status notification → show success/failure.

---

## 5. Example Message Flows

### A. Scan request and one chunk of results (MTU 247)

- App → Device: Write Control `{"cmd":"scan_start"}` (or Scan Control 0x01).
- Device: starts scan; when done, Indicate Scan Results chunk 1/1: `[0x00, 0x01, …json…]`.
- App: ACKs Indication; parses JSON list of `{ssid, rssi, auth}`.

### B. Send credentials and connect (short SSID/password)

- App → Device: Write SSID `"MyNetwork"`.
- App → Device: Write Password `"secret123"`.
- App → Device: Write Control `{"cmd":"connect"}`.
- Device → App: Notify Status `{"state":"connecting"}`.
- Device → App: Notify Status `{"state":"connected","ip":"192.168.1.10"}`.

### C. Long password (fragmentation, MTU 23)

- Payload 30 bytes; chunk size 20. Chunk1: `[0x00, 0x02, <18 bytes>]` (total 2 chunks, 2 bytes header + 18 payload). Chunk2: `[0x01, 0x01, <12 bytes>]` (continuation, index 1, 12 bytes). Receiver reassembles then treats as single password.

---

## 6. JSON Schema for Messages

Used in Control (command/response), Status (notify), and optionally Scan Results (indicate). Binary Security/Options remains fixed-length binary.

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "definitions": {
    "ControlCommand": {
      "type": "object",
      "properties": {
        "cmd": { "enum": ["scan_start", "scan_stop", "connect", "cancel", "get_state"] },
        "id": { "type": "integer", "minimum": 0 }
      },
      "required": ["cmd"]
    },
    "ControlResponse": {
      "type": "object",
      "properties": {
        "id": { "type": "integer" },
        "ok": { "type": "boolean" },
        "err": { "type": "string" },
        "state": { "type": "string" }
      }
    },
    "StatusNotify": {
      "type": "object",
      "properties": {
        "state": { "enum": ["idle", "scanning", "connecting", "connected", "failed", "disconnected"] },
        "code": { "type": "integer" },
        "msg": { "type": "string" },
        "ip": { "type": "string", "format": "ipv4" }
      },
      "required": ["state"]
    },
    "ScanResultEntry": {
      "type": "object",
      "properties": {
        "ssid": { "type": "string" },
        "rssi": { "type": "integer" },
        "auth": { "type": "integer" }
      },
      "required": ["ssid", "rssi"]
    },
    "ScanResultsPayload": {
      "type": "object",
      "properties": {
        "chunk": { "type": "integer" },
        "total": { "type": "integer" },
        "networks": { "type": "array", "items": { "$ref": "#/definitions/ScanResultEntry" } }
      },
      "required": ["chunk", "total", "networks"]
    }
  },
  "oneOf": [
    { "$ref": "#/definitions/ControlCommand" },
    { "$ref": "#/definitions/ControlResponse" },
    { "$ref": "#/definitions/StatusNotify" },
    { "$ref": "#/definitions/ScanResultsPayload" }
  ]
}
```

- **Control**: App sends `ControlCommand`; device may respond via Control (Read) or Status (Notify) with `ControlResponse` or state in `StatusNotify`.
- **Status**: Device sends `StatusNotify` on the Status characteristic.
- **Scan Results**: Device sends `ScanResultsPayload` (possibly chunked; `chunk`/`total` for reassembly).

---

## 7. GATT Table Summary (Quick Reference)

| Char | UUID | R | W | N | I | Max len | Fragment |
|------|------|---|---|---|---|--------|----------|
| Control | 0xFE01 | ✓ | ✓ | — | — | 20 | no |
| Status | 0xFE02 | ✓ | — | ✓ | — | 64 | no |
| WiFi SSID | 0xFE03 | — | ✓ | — | — | 32 | yes |
| WiFi Password | 0xFE04 | — | ✓ | — | — | 64 | yes |
| Security/Options | 0xFE05 | ✓ | ✓ | — | — | 16 | no |
| Scan Results | 0xFE06 | ✓ | — | — | ✓ | 244 | yes (chunked) |
| Scan Control | 0xFE07 | — | ✓ | — | — | 8 | no |

R=Read, W=Write, N=Notify, I=Indicate. All writes use “Write with response”. MTU 247 preferred; design works with 23.
