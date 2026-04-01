# ESP32-S3 ESP-IDF Version Support

## Version Compatibility Matrix

| ESP-IDF Version | ESP32-S3 Support | Status | Notes |
|----------------|------------------|--------|-------|
| v4.4.x | ✅ Yes | Stable | Initial ESP32-S3 support |
| v5.0.x | ✅ Yes | Stable | Enhanced features |
| v5.1.x | ✅ Yes | Stable | Recommended for production |
| v5.2.x | ✅ Yes | Stable | Latest v5.x |
| v5.3.x | ✅ Yes | Stable | Latest v5.x |
| v5.4.x | ✅ Yes | Stable | Latest v5.x |
| v5.5.x | ✅ Yes | Stable | Latest v5.x |
| v6.0.x | ✅ Yes | Stable | Major release |
| v6.1.x | ✅ Yes | Development | Current (as of collection) |

## ESP32-S3 Feature Support by Version

### v4.4 (Initial Support)
- Basic ESP32-S3 support
- Core peripherals (GPIO, UART, SPI, I2C)
- WiFi and Bluetooth
- USB Serial/JTAG

### v5.0+ (Enhanced)
- Improved USB support
- Better power management
- Enhanced WiFi features
- Improved Bluetooth stack

### v6.0+ (Latest)
- Latest features and optimizations
- Improved build system
- Enhanced security features
- Better tooling support

## Migration Notes

### v4.4 → v5.0
- Driver API changes
- Component updates
- Build system improvements
- See: `docs/en/migration-guides/release-5.x/5.0/`

### v5.x → v6.0
- Major build system changes
- Component API updates
- Toolchain requirements
- See: `docs/en/migration-guides/release-6.x/6.0/`

## Recommended Versions

- **Production**: v5.1+ or v5.5 (stable, well-tested)
- **Development**: v6.1+ (latest features)
- **Legacy projects**: v4.4 (if compatibility required)

## Version Detection

```bash
# Check installed version
idf.py --version

# Or
git -C $IDF_PATH describe --tags

# Or check version.cmake
cat $IDF_PATH/tools/cmake/version.cmake
```

## Component Compatibility

Some components require specific IDF versions:
- Check component README for version requirements
- Managed components may have version constraints
- See component's `idf_component.yml` for requirements

## Breaking Changes

Always check migration guides when upgrading:
- `docs/en/migration-guides/release-5.x/`
- `docs/en/migration-guides/release-6.x/`

## Related
- ESP-IDF Release Notes
- Migration Guides
- Component Compatibility
