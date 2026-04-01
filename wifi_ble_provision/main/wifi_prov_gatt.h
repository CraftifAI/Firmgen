/*
 * WiFi-over-BLE GATT provisioning - device side (ESP32, ESP-IDF).
 * Minimal GATT: one service, credentials (write), status (notify), control (write).
 */
#ifndef WIFI_PROV_GATT_H
#define WIFI_PROV_GATT_H

#include <stdint.h>
#include <stdbool.h>

/* Status values sent over Status characteristic (notify) */
#define WIFI_PROV_STATUS_IDLE              0
#define WIFI_PROV_STATUS_CONNECTING        1
#define WIFI_PROV_STATUS_CONNECTED         2
#define WIFI_PROV_STATUS_FAILED            3
#define WIFI_PROV_STATUS_PROVISIONING_MODE 4

/* Control commands (write to Control characteristic) */
#define WIFI_PROV_CTRL_CONNECT       0x01  /* Connect using stored credentials */
#define WIFI_PROV_CTRL_FACTORY_RESET 0x02  /* Clear NVS and enter provisioning mode */

/* Callback when credentials are received and parsed: (ssid, ssid_len, password, pass_len). */
typedef void (*wifi_prov_creds_cb_t)(const char *ssid, uint8_t ssid_len,
                                     const char *password, uint8_t pass_len,
                                     void *arg);
/* Callback when control command is received. */
typedef void (*wifi_prov_ctrl_cb_t)(uint8_t cmd, void *arg);

void wifi_prov_gatt_set_creds_cb(wifi_prov_creds_cb_t cb, void *arg);
void wifi_prov_gatt_set_ctrl_cb(wifi_prov_ctrl_cb_t cb, void *arg);

/* Notify current status to the connected BLE client. Call from WiFi event handlers. */
void wifi_prov_notify_status(uint8_t status);

/* GATT server init (call after NimBLE host init). Returns 0 on success. */
int wifi_prov_gatt_init(void);

/* Register GATT services (call from ble_hs_cfg.gatts_register_cb). */
void wifi_prov_gatt_register_cb(void *ctxt, void *arg);
/* Subscribe callback (set in ble_hs_cfg.gatts_subscribe_cb). */
void wifi_prov_gatt_subscribe_cb(void *event, void *arg);

/* Clear stored connection handle on BLE disconnect (call from GAP disconnect handler). */
void wifi_prov_gatt_clear_conn_handle(uint16_t conn_handle);

#endif /* WIFI_PROV_GATT_H */
