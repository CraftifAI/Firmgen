# WiFi-over-BLE Provisioning (ESP32, ESP-IDF)

Minimal device-side firmware for provisioning WiFi credentials over BLE: GATT service for credentials/control, NVS storage, WiFi connect with retries, and status notifications.

**Recommended ESP-IDF:** v5.1 or v5.2 LTS (or latest stable 5.x).

---

## Build

### Prerequisites

- ESP-IDF v5.1+ (e.g. v5.1.2 or v5.2) installed and `IDF_PATH` set.
- Target: ESP32, ESP32-C3, ESP32-S3, etc.

### Steps

```bash
cd wifi_ble_provision
idf.py set-target esp32   # or esp32c3, esp32s3, etc.
idf.py build
idf.py -p /dev/ttyUSB0 flash monitor
```

### Required components (from `main/CMakeLists.txt`)

- **nvs_flash** – credential storage  
- **esp_wifi** – station mode connect  
- **esp_event**, **esp_netif** – WiFi/network events  
- **bt** – NimBLE (BLE only; Bluedroid disabled via `sdkconfig.defaults`)

---

## GATT design

| UUID  | Type    | Permissions | Description |
|-------|---------|-------------|-------------|
| 0xFF00 | Service | -           | WiFi provisioning |
| 0xFF01 | Char    | Write       | Credentials: `[ssid_len][ssid][pass_len][pass]` (max 32 + 64 bytes) |
| 0xFF02 | Char    | Read, Notify| Status: 0=idle, 1=connecting, 2=connected, 3=failed, 4=provisioning_mode |
| 0xFF03 | Char    | Write       | Control: 0x01=connect (stored), 0x02=factory reset |

---

## Key APIs (snippets)

### BLE GATT registration (NimBLE)

```c
#include "host/ble_hs.h"
#include "services/gap/ble_svc_gap.h"
#include "services/gatt/ble_svc_gatt.h"

static const struct ble_gatt_svc_def wifi_prov_svc_defs[] = {
    { .type = BLE_GATT_SVC_TYPE_PRIMARY, .uuid = &svc_uuid.u,
      .characteristics = (struct ble_gatt_chr_def[]){
          { .uuid = &cred_chr_uuid.u, .access_cb = cred_chr_access,
            .flags = BLE_GATT_CHR_F_WRITE, .val_handle = &cred_chr_val_handle },
          { .uuid = &status_chr_uuid.u, .access_cb = status_chr_access,
            .flags = BLE_GATT_CHR_F_READ | BLE_GATT_CHR_F_NOTIFY, .val_handle = &status_chr_val_handle },
          { 0 } },
    },
    { 0 },
};
ble_gatts_add_svcs(wifi_prov_svc_defs);
```

### Status notification

```c
struct os_mbuf *om = ble_hs_mbuf_from_flat(&status_byte, 1);
ble_gatts_notify_custom(conn_handle, status_chr_val_handle, om);
```

### WiFi init and connect

```c
esp_netif_init();
esp_event_loop_create_default();
esp_netif_create_default_wifi_sta();
esp_wifi_init(&WIFI_INIT_CONFIG_DEFAULT());
esp_wifi_set_mode(WIFI_MODE_STA);
esp_wifi_start();
// then:
wifi_config_t cfg = { .sta = { .threshold.authmode = WIFI_AUTH_WPA2_PSK } };
memcpy(cfg.sta.ssid, ssid, ssid_len); cfg.sta.ssid[ssid_len] = '\0';
memcpy(cfg.sta.password, pass, pass_len); cfg.sta.password[pass_len] = '\0';
esp_wifi_set_config(WIFI_IF_STA, &cfg);
esp_wifi_connect();
```

### NVS read/write credentials

```c
nvs_handle_t h;
nvs_open("wifi_prov", NVS_READWRITE, &h);
nvs_set_str(h, "ssid", ssid);
nvs_set_str(h, "pass", pass);
nvs_commit(h);
nvs_close(h);

// read:
size_t len = 0;
nvs_get_str(h, "ssid", NULL, &len);
nvs_get_str(h, "ssid", buf, &len);
```

---

## Error handling and reconnection

- **WiFi:** On `WIFI_EVENT_STA_DISCONNECTED`, retry up to `WIFI_MAX_RETRY` (5); then set `WIFI_FAIL_BIT` and notify status `WIFI_PROV_STATUS_FAILED`.
- **NVS:** `nvs_flash_init()` on `ESP_ERR_NVS_NO_FREE_PAGES` or `ESP_ERR_NVS_NEW_VERSION_FOUND`: call `nvs_flash_erase()` then re-init.
- **BLE:** On disconnect, clear the status-notify subscription and restart advertising.

---

## Re-provisioning

1. **Factory reset (over BLE):** App writes `0x02` to Control (0xFF03). Device erases NVS namespace `wifi_prov`, disconnects WiFi, notifies `WIFI_PROV_STATUS_PROVISIONING_MODE`, and restarts BLE advertising.
2. **First-time:** If no stored credentials, device advertises only; app writes credentials to 0xFF01, then device saves to NVS and connects.
3. **Re-connect with stored creds:** App writes `0x01` to Control; device reads NVS and calls `esp_wifi_connect()`.

---

## Integration checklist

- [ ] Copy `wifi_ble_provision/` (or merge `main/` into your app) and ensure `main/CMakeLists.txt` lists `main.c`, `wifi_prov_gatt.c`, `wifi_prov_nvs.c` and `PRIV_REQUIRES` as above.
- [ ] Set target: `idf.py set-target <esp32|esp32c3|esp32s3|...>`.
- [ ] Keep `sdkconfig.defaults`: BLE only, NimBLE enabled, Bluedroid disabled.
- [ ] Implement or reuse `ble_store_config_init()` (NimBLE provides it if using the standard port).
- [ ] Optional: Change device name in `ble_svc_gap_device_name_set("ESP32-WiFiProv")`.
- [ ] Optional: Add BLE security (pairing/bonding) for production.
- [ ] Test: connect with a BLE GATT client, write credentials in format `[ssid_len][ssid][pass_len][pass]`, enable notify on 0xFF02, then trigger connect (0x01) or factory reset (0x02).
