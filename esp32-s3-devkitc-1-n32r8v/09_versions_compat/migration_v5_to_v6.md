# Migration Guide: ESP-IDF v5.x to v6.0

## Overview

ESP-IDF v6.0 introduces significant changes to the build system, component APIs, and tooling. This guide covers key migration points for ESP32-S3 projects.

## Build System Changes

### CMake Requirements
- Minimum CMake version: 3.16 (was 3.5)
- New component registration syntax
- Improved dependency management

### Component Changes
- Some components renamed or restructured
- New component dependencies
- Updated component APIs

## Key Breaking Changes

### 1. Build System
- CMake minimum version increased
- Component registration syntax updated
- Dependency resolution improved

### 2. Driver APIs
- Some driver functions updated
- New error handling patterns
- Improved parameter validation

### 3. WiFi/BT
- API updates for latest features
- Improved power management
- Enhanced security options

### 4. Toolchain
- GCC version requirements updated
- New compiler optimizations
- Updated linker scripts

## Migration Steps

1. **Backup your project**
2. **Update ESP-IDF**: `git pull` or checkout v6.0 tag
3. **Update toolchain**: Run `install.sh` or `install.bat`
4. **Review migration guide**: `docs/en/migration-guides/release-6.x/6.0/`
5. **Update sdkconfig**: Run `idf.py reconfigure`
6. **Fix compilation errors**: Address API changes
7. **Test thoroughly**: Verify all functionality

## Common Issues

### CMake Errors
```
CMake Error: CMake 3.16 or higher is required
```
**Fix**: Upgrade CMake to 3.16+

### Component Not Found
```
Component 'xxx' not found
```
**Fix**: Check component name changes, update CMakeLists.txt

### API Deprecation Warnings
```
warning: 'function_name' is deprecated
```
**Fix**: Update to new API, check migration guide

## Testing Checklist

- [ ] Project builds successfully
- [ ] All features work as expected
- [ ] No new warnings or errors
- [ ] Performance is acceptable
- [ ] Memory usage is within limits

## Rollback

If migration causes issues:
```bash
git checkout v5.5  # Or your previous version
idf.py fullclean
idf.py build
```

## Related
- Full Migration Guide: `docs/en/migration-guides/release-6.x/6.0/`
- Release Notes: `docs/en/release-notes/`
- Breaking Changes: Check migration guide sections
