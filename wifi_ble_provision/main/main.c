/*
 * WiFi-over-BLE provisioning - minimal device example (ESP32, ESP-IDF).
 * BLE GATT: credentials write, status notify, control (connect / factory reset).
 * Connects using stored NVS credentials on boot or when requested over BLE;
 * reconnection with retries; re-provisioning via factory reset or first-time.
 */
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/event_groups.h"
#include "esp_system.h"
#include "esp_wifi.h"
#include "esp_event.h"
#include "esp_log.h"
#include "nvs_flash.h"
#include "esp_netif.h"

#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "host/ble_hs.h"
#include "host/util/util.h"
#include "services/gap/ble_svc_gap.h"
#include "services/gatt/ble_svc_gatt.h"

#include "wifi_prov_gatt.h"
#include "wifi_prov_nvs.h"

static const char *TAG = "wifi_ble_prov";

#define WIFI_MAX_RETRY    5
#define WIFI_CONNECTED_BIT  BIT0
#define WIFI_FAIL_BIT       BIT1

static EventGroupHandle_t s_wifi_event_group;
static int s_retry_num;
static bool s_wifi_connecting;

static uint8_t s_own_addr_type;
static void start_ble_advertise(void);

/* WiFi event handler: drive retries and notify status over BLE */
static void wifi_event_handler(void *arg, esp_event_base_t event_base,
                               int32_t event_id, void *event_data)
{
    if (event_base != WIFI_EVENT) {
        return;
    }
    switch (event_id) {
    case WIFI_EVENT_STA_START:
        if (s_wifi_connecting) {
            esp_wifi_connect();
        }
        break;
    case WIFI_EVENT_STA_CONNECTED:
        s_retry_num = 0;
        wifi_prov_notify_status(WIFI_PROV_STATUS_CONNECTING);
        break;
    case WIFI_EVENT_STA_DISCONNECTED: {
        if (s_wifi_connecting && s_retry_num < WIFI_MAX_RETRY) {
            s_retry_num++;
            esp_wifi_connect();
            wifi_prov_notify_status(WIFI_PROV_STATUS_CONNECTING);
        } else {
            s_wifi_connecting = false;
            xEventGroupSetBits(s_wifi_event_group, WIFI_FAIL_BIT);
            wifi_prov_notify_status(WIFI_PROV_STATUS_FAILED);
        }
        break;
    }
    default:
        break;
    }
}

static void ip_event_handler(void *arg, esp_event_base_t event_base,
                             int32_t event_id, void *event_data)
{
    if (event_base != IP_EVENT || event_id != IP_EVENT_STA_GOT_IP) {
        return;
    }
    ip_event_got_ip_t *event = (ip_event_got_ip_t *)event_data;
    ESP_LOGI(TAG, "got ip: " IPSTR, IP2STR(&event->ip_info.ip));
    s_wifi_connecting = false;
    xEventGroupSetBits(s_wifi_event_group, WIFI_CONNECTED_BIT);
    wifi_prov_notify_status(WIFI_PROV_STATUS_CONNECTED);
}

static void wifi_init_sta(void)
{
    s_wifi_event_group = xEventGroupCreate();
    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
    esp_netif_create_default_wifi_sta();

    ESP_ERROR_CHECK(esp_event_handler_instance_register(WIFI_EVENT, ESP_EVENT_ANY_ID,
                                                        &wifi_event_handler, NULL, NULL));
    ESP_ERROR_CHECK(esp_event_handler_instance_register(IP_EVENT, IP_EVENT_STA_GOT_IP,
                                                        &ip_event_handler, NULL, NULL));

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
    ESP_ERROR_CHECK(esp_wifi_start());
}

/* Connect using stored credentials from NVS */
static void wifi_connect_stored(void)
{
    char ssid[33], pass[65];
    uint32_t ssid_len, pass_len;
    if (!wifi_prov_nvs_has_creds()) {
        ESP_LOGW(TAG, "no stored credentials");
        wifi_prov_notify_status(WIFI_PROV_STATUS_FAILED);
        return;
    }
    esp_err_t err = wifi_prov_nvs_get_creds(ssid, sizeof(ssid), pass, sizeof(pass),
                                            &ssid_len, &pass_len);
    if (err != ESP_OK) {
        wifi_prov_notify_status(WIFI_PROV_STATUS_FAILED);
        return;
    }
    wifi_config_t cfg = { 0 };
    memcpy(cfg.sta.ssid, ssid, ssid_len);
    cfg.sta.ssid[ssid_len] = '\0';
    memcpy(cfg.sta.password, pass, pass_len);
    cfg.sta.password[pass_len] = '\0';
    cfg.sta.threshold.authmode = WIFI_AUTH_WPA2_PSK;
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &cfg));
    s_retry_num = 0;
    s_wifi_connecting = true;
    wifi_prov_notify_status(WIFI_PROV_STATUS_CONNECTING);
    esp_wifi_connect();
}

/* Called when app writes credentials over BLE */
static void on_creds_received(const char *ssid, uint8_t ssid_len,
                              const char *pass, uint8_t pass_len,
                              void *arg)
{
    (void)arg;
    esp_err_t err = wifi_prov_nvs_set_creds(ssid, ssid_len, pass, pass_len);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "nvs_set_creds failed: %s", esp_err_to_name(err));
        wifi_prov_notify_status(WIFI_PROV_STATUS_FAILED);
        return;
    }
    ESP_LOGI(TAG, "credentials saved, connecting to %.32s", ssid);
    wifi_config_t cfg = { 0 };
    memcpy(cfg.sta.ssid, ssid, ssid_len);
    cfg.sta.ssid[ssid_len] = '\0';
    memcpy(cfg.sta.password, pass, pass_len);
    cfg.sta.password[pass_len] = '\0';
    cfg.sta.threshold.authmode = WIFI_AUTH_WPA2_PSK;
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &cfg));
    s_retry_num = 0;
    s_wifi_connecting = true;
    wifi_prov_notify_status(WIFI_PROV_STATUS_CONNECTING);
    esp_wifi_connect();
}

/* Called when app writes control command */
static void on_ctrl_cmd(uint8_t cmd, void *arg)
{
    (void)arg;
    if (cmd == WIFI_PROV_CTRL_CONNECT) {
        wifi_connect_stored();
    } else if (cmd == WIFI_PROV_CTRL_FACTORY_RESET) {
        ESP_LOGI(TAG, "factory reset");
        esp_wifi_disconnect();
        wifi_prov_nvs_factory_reset();
        wifi_prov_notify_status(WIFI_PROV_STATUS_PROVISIONING_MODE);
        start_ble_advertise();
    }
}

static void on_ble_sync(void);
static void on_ble_reset(int reason);

static int ble_gap_event(struct ble_gap_event *event, void *arg);

static void start_ble_advertise(void)
{
    struct ble_gap_adv_params adv_params;
    struct ble_hs_adv_fields fields;
    int rc;

    memset(&fields, 0, sizeof(fields));
    fields.flags = BLE_HS_ADV_F_DISC_GEN | BLE_HS_ADV_F_BREDR_UNSUP;
    fields.name = (uint8_t *)ble_svc_gap_device_name();
    fields.name_len = strlen(ble_svc_gap_device_name());
    fields.name_is_complete = 1;
    fields.uuids16 = (ble_uuid16_t[]){ BLE_UUID16_INIT(0xFF00) };
    fields.num_uuids16 = 1;
    fields.uuids16_is_complete = 1;

    rc = ble_gap_adv_set_fields(&fields);
    if (rc != 0) {
        ESP_LOGE(TAG, "adv_set_fields rc=%d", rc);
        return;
    }
    memset(&adv_params, 0, sizeof(adv_params));
    adv_params.conn_mode = BLE_GAP_CONN_MODE_UND;
    adv_params.disc_mode = BLE_GAP_DISC_MODE_GEN;
    rc = ble_gap_adv_start(s_own_addr_type, NULL, BLE_HS_FOREVER,
                           &adv_params, ble_gap_event, NULL);
    if (rc != 0) {
        ESP_LOGE(TAG, "adv_start rc=%d", rc);
        return;
    }
    ESP_LOGI(TAG, "advertising started");
}

static int ble_gap_event(struct ble_gap_event *event, void *arg)
{
    switch (event->type) {
    case BLE_GAP_EVENT_CONNECT:
        if (event->connect.status != 0) {
            start_ble_advertise();
            break;
        }
        ESP_LOGI(TAG, "BLE connected");
        wifi_prov_notify_status(WIFI_PROV_STATUS_IDLE);
        break;
    case BLE_GAP_EVENT_DISCONNECT:
        ESP_LOGI(TAG, "BLE disconnected");
        wifi_prov_gatt_clear_conn_handle(event->disconnect.conn.conn_handle);
        start_ble_advertise();
        break;
    case BLE_GAP_EVENT_SUBSCRIBE:
        wifi_prov_gatt_subscribe_cb(event, NULL);
        break;
    case BLE_GAP_EVENT_ADV_COMPLETE:
        start_ble_advertise();
        break;
    default:
        break;
    }
    return 0;
}

static void on_ble_sync(void)
{
    int rc = ble_hs_util_ensure_addr(0);
    if (rc != 0) {
        ESP_LOGE(TAG, "ensure_addr rc=%d", rc);
        return;
    }
    rc = ble_hs_id_infer_auto(0, &s_own_addr_type);
    if (rc != 0) {
        ESP_LOGE(TAG, "id_infer_auto rc=%d", rc);
        return;
    }
    start_ble_advertise();
}

static void on_ble_reset(int reason)
{
    ESP_LOGI(TAG, "BLE stack reset reason=%d", reason);
}

void ble_store_config_init(void);

static void nimble_host_task(void *param)
{
    nimble_port_run();
    vTaskDelete(NULL);
}

void app_main(void)
{
    esp_err_t err;

    err = nvs_flash_init();
    if (err == ESP_ERR_NVS_NO_FREE_PAGES || err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        err = nvs_flash_init();
    }
    ESP_ERROR_CHECK(err);

    wifi_init_sta();
    s_wifi_connecting = false;

    wifi_prov_gatt_set_creds_cb(on_creds_received, NULL);
    wifi_prov_gatt_set_ctrl_cb(on_ctrl_cmd, NULL);

    err = nimble_port_init();
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "nimble_port_init failed: %s", esp_err_to_name(err));
        return;
    }

    ble_hs_cfg.sync_cb = on_ble_sync;
    ble_hs_cfg.reset_cb = on_ble_reset;
    ble_hs_cfg.gatts_register_cb = wifi_prov_gatt_register_cb;
    ble_store_config_init();

    if (wifi_prov_gatt_init() != 0) {
        ESP_LOGE(TAG, "wifi_prov_gatt_init failed");
        return;
    }

    ble_svc_gap_device_name_set("ESP32-WiFiProv");

    xTaskCreate(nimble_host_task, "nimble", 4096, NULL, 5, NULL);

    /* Optionally try to connect on boot if credentials exist */
    if (wifi_prov_nvs_has_creds()) {
        vTaskDelay(pdMS_TO_TICKS(500));
        wifi_connect_stored();
    } else {
        wifi_prov_notify_status(WIFI_PROV_STATUS_PROVISIONING_MODE);
    }
}
