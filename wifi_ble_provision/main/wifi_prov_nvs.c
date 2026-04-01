/*
 * NVS read/write for WiFi credentials.
 */
#include "wifi_prov_nvs.h"
#include "nvs_flash.h"
#include "nvs.h"
#include "esp_log.h"
#include <string.h>

static const char *TAG = "wifi_prov_nvs";

#define SSID_MAX 32
#define PASS_MAX 64

bool wifi_prov_nvs_has_creds(void)
{
    nvs_handle_t h;
    esp_err_t err = nvs_open(WIFI_PROV_NVS_NAMESPACE, NVS_READONLY, &h);
    if (err != ESP_OK) {
        return false;
    }
    size_t len = 0;
    err = nvs_get_str(h, WIFI_PROV_NVS_SSID_KEY, NULL, &len);
    nvs_close(h);
    return (err == ESP_OK && len > 0);
}

esp_err_t wifi_prov_nvs_get_creds(char *ssid, uint32_t ssid_max,
                                  char *pass, uint32_t pass_max,
                                  uint32_t *out_ssid_len, uint32_t *out_pass_len)
{
    if (!ssid || ssid_max == 0 || !pass || pass_max == 0) {
        return ESP_ERR_INVALID_ARG;
    }
    nvs_handle_t h;
    esp_err_t err = nvs_open(WIFI_PROV_NVS_NAMESPACE, NVS_READONLY, &h);
    if (err != ESP_OK) {
        return err;
    }
    size_t len = ssid_max;
    err = nvs_get_str(h, WIFI_PROV_NVS_SSID_KEY, ssid, &len);
    if (err != ESP_OK) {
        nvs_close(h);
        return err;
    }
    if (out_ssid_len) {
        *out_ssid_len = (uint32_t)(len - 1);
    }
    len = pass_max;
    err = nvs_get_str(h, WIFI_PROV_NVS_PASS_KEY, pass, &len);
    nvs_close(h);
    if (err != ESP_OK) {
        return err;
    }
    if (out_pass_len) {
        *out_pass_len = (uint32_t)(len - 1);
    }
    return ESP_OK;
}

esp_err_t wifi_prov_nvs_set_creds(const char *ssid, uint32_t ssid_len,
                                  const char *pass, uint32_t pass_len)
{
    if (!ssid || ssid_len > SSID_MAX || !pass || pass_len > PASS_MAX) {
        return ESP_ERR_INVALID_ARG;
    }
    nvs_handle_t h;
    esp_err_t err = nvs_open(WIFI_PROV_NVS_NAMESPACE, NVS_READWRITE, &h);
    if (err != ESP_OK) {
        return err;
    }
    err = nvs_set_str(h, WIFI_PROV_NVS_SSID_KEY, ssid);
    if (err != ESP_OK) {
        nvs_close(h);
        return err;
    }
    err = nvs_set_str(h, WIFI_PROV_NVS_PASS_KEY, pass);
    if (err != ESP_OK) {
        nvs_close(h);
        return err;
    }
    err = nvs_commit(h);
    nvs_close(h);
    return err;
}

esp_err_t wifi_prov_nvs_factory_reset(void)
{
    nvs_handle_t h;
    esp_err_t err = nvs_open(WIFI_PROV_NVS_NAMESPACE, NVS_READWRITE, &h);
    if (err == ESP_ERR_NVS_NOT_FOUND) {
        return ESP_OK; /* nothing to erase */
    }
    if (err != ESP_OK) {
        return err;
    }
    err = nvs_erase_all(h);
    if (err != ESP_OK) {
        nvs_close(h);
        return err;
    }
    err = nvs_commit(h);
    nvs_close(h);
    if (err == ESP_OK) {
        ESP_LOGI(TAG, "factory reset: wifi prov namespace erased");
    }
    return err;
}
