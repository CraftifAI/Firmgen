# Manifest Generation Guide

## Overview

The `manifest.jsonl` file contains metadata for all files in the knowledge base. Each line is a JSON object with file metadata.

## Enhanced Tag Strategy

### Task-Oriented Tags
- `task:gpio` - GPIO configuration and examples
- `task:pwm` - PWM/LEDC configuration and examples
- `task:adc` - ADC configuration and examples
- `task:wifi_station` - WiFi Station mode configuration and examples

### Category Tags
- `board` - Board-specific documentation
- `pinout` - Pinout references
- `gpio` - GPIO-related content
- `pwm` - PWM/LEDC-related content
- `adc` - ADC-related content
- `wifi` - WiFi-related content
- `troubleshooting` - Troubleshooting guides
- `error` - Error documentation
- `example` - Example code
- `api` - API documentation
- `peripherals` - Peripheral documentation

### Error-Specific Tags
- `build` - Build errors
- `flash` - Flash errors
- `runtime` - Runtime errors
- `memory` - Memory-related errors
- `power` - Power-related errors
- `interrupt` - Interrupt-related errors

## Generating Full Manifest

To generate the full manifest from the 16n8r collection:

```python
import json
import os
from pathlib import Path

def enhance_tags(path, existing_tags):
    """Add task-oriented and category tags based on file path"""
    tags = list(existing_tags) if existing_tags else []
    path_lower = path.lower()
    
    # Category tags
    if 'board' in path:
        tags.append('board')
    if 'gpio' in path_lower:
        tags.append('gpio')
        if 'example' in path_lower or 'main' in path_lower:
            tags.append('task:gpio')
    if 'ledc' in path_lower or 'pwm' in path_lower:
        tags.append('pwm')
        if 'example' in path_lower or 'main' in path_lower:
            tags.append('task:pwm')
    if 'adc' in path_lower:
        tags.append('adc')
        if 'example' in path_lower or 'main' in path_lower:
            tags.append('task:adc')
    if 'wifi' in path_lower:
        tags.append('wifi')
        if 'station' in path_lower and ('example' in path_lower or 'main' in path_lower):
            tags.append('task:wifi_station')
    if 'known_issues' in path or 'troubleshooting' in path_lower:
        tags.append('troubleshooting')
    if 'error' in path_lower:
        tags.append('error')
    if 'example' in path_lower:
        tags.append('example')
    if 'api-reference' in path or 'api-guides' in path:
        tags.append('api')
    if 'peripherals' in path:
        tags.append('peripherals')
    
    return list(set(tags))  # Remove duplicates

# Read 16n8r manifest
source_manifest = Path('../esp32-s3-devkitc-16n8r/00_manifest/manifest.jsonl')
target_manifest = Path('manifest.jsonl')

with open(source_manifest, 'r') as f_in, open(target_manifest, 'w') as f_out:
    for line in f_in:
        entry = json.loads(line)
        # Update variant
        entry['variant'] = 'n32r8v'
        entry['board'] = 'esp32-s3-devkitc-1'
        # Enhance tags
        entry['tags'] = enhance_tags(entry['path'], entry.get('tags', []))
        # Write updated entry
        f_out.write(json.dumps(entry) + '\n')
```

## Manual Updates

For variant-specific files (board_summary.md, pinout_reference.md, board_facts.json), use the entries in `manifest_sample.jsonl` as templates.
