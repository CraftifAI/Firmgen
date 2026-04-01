# Build Error: Undefined Reference

## Error Pattern
```
undefined reference to `esp_wifi_init`
undefined reference to `esp_bt_controller_init`
undefined reference to `function_name`
```

## Issue Type
**build_error**

## Component
**build_system**, **esp_wifi**, **esp_bt**, or component containing missing function

## Root Cause
Missing component dependency in `CMakeLists.txt`. The component that provides the function is not included in the build.

## Solution

1. **Check component dependencies**: Ensure the component providing the function is listed in your `main/CMakeLists.txt`:
   ```cmake
   idf_component_register(
       SRCS "main.c"
       INCLUDE_DIRS "."
       REQUIRES esp_wifi esp_bt  # Add missing component here
   )
   ```

2. **Verify component exists**: Check if the component exists in `$IDF_PATH/components/`

3. **Check component dependencies**: Some components require other components. Check the component's `CMakeLists.txt` for `PRIV_REQUIRES` or `REQUIRES`

4. **Rebuild**: After adding dependencies, run `idf.py fullclean` then `idf.py build`

## Common Missing Dependencies

- `esp_wifi_init()` → Requires `esp_wifi` component
- `esp_bt_controller_init()` → Requires `esp_bt` component  
- `nvs_flash_init()` → Requires `nvs_flash` component
- `esp_http_client_init()` → Requires `esp_http_client` component

## Prevention
Always check component documentation for required dependencies before using APIs.

## Related
- ESP-IDF Build System documentation
- Component dependencies in CMakeLists.txt
