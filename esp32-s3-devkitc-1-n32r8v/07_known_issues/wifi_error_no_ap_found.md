# WiFi Error: WIFI_REASON_NO_AP_FOUND

## Error Pattern
```
W (xxxx) wifi: wifi connect failed, reason: WIFI_REASON_NO_AP_FOUND
E (xxxx) wifi: Failed to connect to AP
```

## Issue Type
**wifi**, **runtime_error**

## Component
**esp_wifi**

## Root Cause
WiFi station cannot find the access point with the specified SSID.

## Common Causes

1. **SSID mismatch**: SSID name doesn't match exactly (case-sensitive)
2. **AP not in range**: Access point too far away or not powered on
3. **Wrong WiFi band**: ESP32-S3 supports 2.4 GHz only (not 5 GHz)
4. **AP hidden**: Access point has hidden SSID (not broadcasting)
5. **Security mismatch**: WPA3-only AP (ESP32-S3 needs WPA2-PSK)
6. **Channel out of range**: AP on unsupported channel

## Solutions

### 1. Verify SSID and Password
```c
// SSID is case-sensitive!
wifi_config_t wifi_config = {
    .sta = {
        .ssid = "MyWiFi",      // Must match exactly
        .password = "password",  // Case-sensitive
    },
};
```

### 2. Check Signal Strength
```c
// Scan for available APs first
esp_wifi_scan_start(NULL, true);
// Check if your AP appears in scan results
```

### 3. Verify WiFi Band
- **ESP32-S3**: 2.4 GHz only
- **5 GHz APs**: Will not be found
- **Solution**: Use 2.4 GHz network or enable 2.4 GHz on dual-band router

### 4. Check AP Security
- **WPA2-PSK**: Supported ✓
- **WPA3-only**: Not supported ✗
- **Open (no password)**: Supported ✓
- **WEP**: Not recommended, may not work

### 5. Handle Hidden SSID
```c
wifi_config_t wifi_config = {
    .sta = {
        .ssid = "HiddenAP",
        .password = "password",
        .scan_method = WIFI_FAST_SCAN,
        .bssid_set = false,
        .channel = 0,  // Auto-scan
    },
};
```

### 6. Debug Connection Process
```c
// Enable WiFi event callbacks
esp_event_handler_instance_t instance_any_id;
esp_event_handler_instance_t instance_got_ip;
esp_event_handler_instance_register(WIFI_EVENT,
                                    ESP_EVENT_ANY_ID,
                                    &wifi_event_handler,
                                    NULL,
                                    &instance_any_id);
esp_event_handler_instance_register(IP_EVENT,
                                    IP_EVENT_STA_GOT_IP,
                                    &got_ip_event_handler,
                                    NULL,
                                    &instance_got_ip);
```

## ESP32-S3-DevKitC Specific

- **WiFi band**: 2.4 GHz only
- **Antenna**: On-board PCB antenna (no external antenna connector)
- **Range**: Typical indoor range 30-50m (depends on environment)
- **Power**: WiFi active mode draws 100-300mA

## Debugging Steps

1. **Scan for APs**: Use `esp_wifi_scan_start()` to see available networks
2. **Check signal strength**: Verify RSSI is above -70 dBm
3. **Test with phone hotspot**: Create 2.4 GHz hotspot to isolate router issues
4. **Check router settings**: Verify 2.4 GHz band is enabled
5. **Review WiFi logs**: Enable verbose logging: `esp_log_level_set("wifi", ESP_LOG_DEBUG)`

## Prevention

- Always scan for APs before connecting
- Verify SSID spelling and case
- Test with known-good AP (phone hotspot)
- Check signal strength before attempting connection
- Handle connection failures gracefully

## Related
- WiFi Station API documentation
- WiFi Event Handling
- Network Configuration
