idf_component_register(
    SRCS "main.c" "wifi_prov_gatt.c" "wifi_prov_nvs.c"
    INCLUDE_DIRS "."
    PRIV_REQUIRES nvs_flash esp_wifi esp_event esp_netif bt
)
