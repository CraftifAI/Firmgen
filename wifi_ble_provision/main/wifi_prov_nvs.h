/*
 * NVS persistence for WiFi credentials (ESP32, ESP-IDF).
 */
#ifndef WIFI_PROV_NVS_H
#define WIFI_PROV_NVS_H

#include "esp_err.h"
#include <stdbool.h>
#include <stdint.h>

#define WIFI_PROV_NVS_NAMESPACE "wifi_prov"
#define WIFI_PROV_NVS_SSID_KEY   "ssid"
#define WIFI_PROV_NVS_PASS_KEY   "pass"

/* Check if credentials are stored (device was provisioned). */
bool wifi_prov_nvs_has_creds(void);

/* Read stored credentials. Caller provides buffers; max lengths are 32 and 64. */
esp_err_t wifi_prov_nvs_get_creds(char *ssid, uint32_t ssid_max,
                                  char *pass, uint32_t pass_max,
                                  uint32_t *out_ssid_len, uint32_t *out_pass_len);

/* Save credentials to NVS. */
esp_err_t wifi_prov_nvs_set_creds(const char *ssid, uint32_t ssid_len,
                                  const char *pass, uint32_t pass_len);

/* Erase WiFi provisioning namespace (factory reset). */
esp_err_t wifi_prov_nvs_factory_reset(void);

#endif /* WIFI_PROV_NVS_H */
