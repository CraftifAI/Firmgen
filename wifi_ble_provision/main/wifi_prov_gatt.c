/*
 * WiFi provisioning GATT service (NimBLE).
 * Service UUID 0xFF00; Characteristics: Credentials (0xFF01 write), Status (0xFF02 notify), Control (0xFF03 write).
 */
#include "wifi_prov_gatt.h"
#include "esp_log.h"
#include "host/ble_hs.h"
#include "host/ble_uuid.h"
#include "host/util/util.h"
#include "services/gap/ble_svc_gap.h"
#include "services/gatt/ble_svc_gatt.h"
#include <string.h>

static const char *TAG = "wifi_prov_gatt";

/* UUIDs: Service 0xFF00, Credentials 0xFF01, Status 0xFF02, Control 0xFF03 */
static const ble_uuid16_t svc_uuid       = BLE_UUID16_INIT(0xFF00);
static const ble_uuid16_t cred_chr_uuid  = BLE_UUID16_INIT(0xFF01);
static const ble_uuid16_t status_chr_uuid = BLE_UUID16_INIT(0xFF02);
static const ble_uuid16_t ctrl_chr_uuid  = BLE_UUID16_INIT(0xFF03);

static uint16_t status_chr_val_handle;
static uint16_t cred_chr_val_handle;
static uint16_t ctrl_chr_val_handle;

static uint16_t status_sub_conn_handle = BLE_HS_CONN_HANDLE_NONE;
static bool status_subscribed;

static wifi_prov_creds_cb_t creds_cb;
static void *creds_cb_arg;
static wifi_prov_ctrl_cb_t ctrl_cb;
static void *ctrl_cb_arg;

/* Credential payload: [ssid_len:1][ssid:1..32][pass_len:1][pass:0..64] */
#define MAX_SSID_LEN 32
#define MAX_PASS_LEN 64
#define MIN_CRED_LEN 2  /* at least ssid_len + pass_len */

static int cred_chr_access(uint16_t conn_handle, uint16_t attr_handle,
                          struct ble_gatt_access_ctxt *ctxt, void *arg);
static int status_chr_access(uint16_t conn_handle, uint16_t attr_handle,
                             struct ble_gatt_access_ctxt *ctxt, void *arg);
static int ctrl_chr_access(uint16_t conn_handle, uint16_t attr_handle,
                          struct ble_gatt_access_ctxt *ctxt, void *arg);

static const struct ble_gatt_svc_def wifi_prov_svc_defs[] = {
    {
        .type = BLE_GATT_SVC_TYPE_PRIMARY,
        .uuid = &svc_uuid.u,
        .characteristics = (struct ble_gatt_chr_def[]) {
            {
                .uuid = &cred_chr_uuid.u,
                .access_cb = cred_chr_access,
                .flags = BLE_GATT_CHR_F_WRITE,
                .val_handle = &cred_chr_val_handle,
            },
            {
                .uuid = &status_chr_uuid.u,
                .access_cb = status_chr_access,
                .flags = BLE_GATT_CHR_F_READ | BLE_GATT_CHR_F_NOTIFY,
                .val_handle = &status_chr_val_handle,
            },
            {
                .uuid = &ctrl_chr_uuid.u,
                .access_cb = ctrl_chr_access,
                .flags = BLE_GATT_CHR_F_WRITE,
                .val_handle = &ctrl_chr_val_handle,
            },
            { 0 },
        },
    },
    { 0 },
};

static int cred_chr_access(uint16_t conn_handle, uint16_t attr_handle,
                          struct ble_gatt_access_ctxt *ctxt, void *arg)
{
    if (ctxt->op != BLE_GATT_ACCESS_OP_WRITE_CHR) {
        return BLE_ATT_ERR_UNLIKELY;
    }
    uint16_t len = OS_MBUF_PKTLEN(ctxt->om);
    if (len < MIN_CRED_LEN || len > (1 + MAX_SSID_LEN + 1 + MAX_PASS_LEN)) {
        ESP_LOGW(TAG, "credential length invalid: %u", len);
        return BLE_ATT_ERR_INVALID_ATTR_VALUE_LEN;
    }
    uint8_t buf[1 + MAX_SSID_LEN + 1 + MAX_PASS_LEN];
    if (ble_hs_mbuf_to_flat(ctxt->om, buf, sizeof(buf), NULL) != 0) {
        return BLE_ATT_ERR_UNLIKELY;
    }
    uint8_t ssid_len = buf[0];
    if (ssid_len > MAX_SSID_LEN || ssid_len == 0) {
        return BLE_ATT_ERR_INVALID_ATTR_VALUE_LEN;
    }
    uint32_t off = 1 + ssid_len;
    if (len < off + 1) {
        return BLE_ATT_ERR_INVALID_ATTR_VALUE_LEN;
    }
    uint8_t pass_len = buf[off];
    if (pass_len > MAX_PASS_LEN || (off + 1 + pass_len) > len) {
        return BLE_ATT_ERR_INVALID_ATTR_VALUE_LEN;
    }
    const char *ssid = (const char *)&buf[1];
    const char *pass = (const char *)&buf[off + 1];
    if (creds_cb) {
        creds_cb(ssid, ssid_len, pass, pass_len, creds_cb_arg);
    }
    return 0;
}

static int status_chr_access(uint16_t conn_handle, uint16_t attr_handle,
                             struct ble_gatt_access_ctxt *ctxt, void *arg)
{
    if (ctxt->op == BLE_GATT_ACCESS_OP_READ_CHR) {
        uint8_t val = WIFI_PROV_STATUS_IDLE;
        return os_mbuf_append(ctxt->om, &val, 1) == 0 ? 0 : BLE_ATT_ERR_INSUFFICIENT_RES;
    }
    return BLE_ATT_ERR_UNLIKELY;
}

static int ctrl_chr_access(uint16_t conn_handle, uint16_t attr_handle,
                          struct ble_gatt_access_ctxt *ctxt, void *arg)
{
    if (ctxt->op != BLE_GATT_ACCESS_OP_WRITE_CHR) {
        return BLE_ATT_ERR_UNLIKELY;
    }
    if (OS_MBUF_PKTLEN(ctxt->om) < 1) {
        return BLE_ATT_ERR_INVALID_ATTR_VALUE_LEN;
    }
    uint8_t cmd;
    if (ble_hs_mbuf_to_flat(ctxt->om, &cmd, 1, NULL) != 0) {
        return BLE_ATT_ERR_UNLIKELY;
    }
    if (ctrl_cb) {
        ctrl_cb(cmd, ctrl_cb_arg);
    }
    return 0;
}

void wifi_prov_gatt_set_creds_cb(wifi_prov_creds_cb_t cb, void *arg)
{
    creds_cb = cb;
    creds_cb_arg = arg;
}

void wifi_prov_gatt_set_ctrl_cb(wifi_prov_ctrl_cb_t cb, void *arg)
{
    ctrl_cb = cb;
    ctrl_cb_arg = arg;
}

void wifi_prov_notify_status(uint8_t status)
{
    if (status_sub_conn_handle == BLE_HS_CONN_HANDLE_NONE || !status_subscribed) {
        return;
    }
    struct os_mbuf *om = ble_hs_mbuf_from_flat(&status, 1);
    if (!om) {
        return;
    }
    if (ble_gatts_notify_custom(status_sub_conn_handle, status_chr_val_handle, om) != 0) {
        os_mbuf_free_chain(om);
    }
}

void wifi_prov_gatt_clear_conn_handle(uint16_t conn_handle)
{
    if (status_sub_conn_handle == conn_handle) {
        status_sub_conn_handle = BLE_HS_CONN_HANDLE_NONE;
        status_subscribed = false;
    }
}

int wifi_prov_gatt_init(void)
{
    ble_svc_gatt_init();
    int rc = ble_gatts_count_cfg(wifi_prov_svc_defs);
    if (rc != 0) {
        return rc;
    }
    rc = ble_gatts_add_svcs(wifi_prov_svc_defs);
    if (rc != 0) {
        return rc;
    }
    return 0;
}

void wifi_prov_gatt_register_cb(void *ctxt, void *arg)
{
    struct ble_gatt_register_ctxt *ctx = (struct ble_gatt_register_ctxt *)ctxt;
    char buf[BLE_UUID_STR_LEN];
    switch (ctx->op) {
    case BLE_GATT_REGISTER_OP_SVC:
        ESP_LOGD(TAG, "svc %s handle=%d", ble_uuid_to_str(ctx->svc.svc_def->uuid, buf), ctx->svc.handle);
        break;
    case BLE_GATT_REGISTER_OP_CHR:
        ESP_LOGD(TAG, "chr %s val_handle=%d", ble_uuid_to_str(ctx->chr.chr_def->uuid, buf), ctx->chr.val_handle);
        break;
    default:
        break;
    }
}

void wifi_prov_gatt_subscribe_cb(void *event, void *arg)
{
    struct ble_gap_event *ev = (struct ble_gap_event *)event;
    if (ev->subscribe.attr_handle == status_chr_val_handle) {
        status_sub_conn_handle = ev->subscribe.conn_handle;
        status_subscribed = ev->subscribe.cur_notify;
        if (!status_subscribed) {
            status_sub_conn_handle = BLE_HS_CONN_HANDLE_NONE;
        }
    }
}
