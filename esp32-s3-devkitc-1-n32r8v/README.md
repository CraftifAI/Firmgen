# ESP32-S3-DevKitC-1 N32R8V Knowledge Base

## Quick Start

This knowledge base is ready for static VecDB creation. The structure follows the same pattern as `esp32-s3-devkitc-16n8r` but with variant-specific updates for the **32MB Flash + 8MB PSRAM** variant.

## Key Differences from 16N8R

- **Flash Size**: 32MB (vs 16MB)
- **Partition Recommendations**: Larger app partitions (4-8MB vs 2-3MB)
- **Board Facts**: New `board_facts.json` for machine-readable queries
- **Enhanced Tags**: Task-oriented tags (`task:gpio`, `task:pwm`, `task:adc`, `task:wifi_station`)

## Structure

```
esp32-s3-devkitc-1-n32r8v/
├── 00_manifest/
│   ├── manifest.jsonl          # Full manifest (generate from 16n8r)
│   ├── manifest_sample.jsonl   # Sample entries with enhanced tags
│   └── README.md               # Manifest generation guide
├── 01_board_docs/
│   ├── board_summary.md         # Variant-specific (32MB flash)
│   ├── pinout_reference.md      # Variant-specific (32MB flash)
│   ├── gpio_pinout_esp32s3.inc # Same as 16n8r
│   └── board_facts.json        # ✨ NEW: Machine-readable facts
├── 02_soc_reference/           # Same as 16n8r (chip-level)
├── 03_idf_docs/                # Same as 16n8r (with enhanced tags)
├── 04_kconfig_symbols/         # Same as 16n8r
├── 05_examples/                # Same as 16n8r (with enhanced tags)
├── 06_commands_workflows/      # Same as 16n8r
├── 07_known_issues/            # Same as 16n8r (with enhanced tags)
├── 08_logs_error_patterns/     # Same as 16n8r
├── 09_versions_compat/         # Same as 16n8r
├── 10_web_snapshots/           # Same as 16n8r
├── COLLECTION_SUMMARY.md       # Collection summary
└── README.md                   # This file
```

## Next Steps

1. **Generate Full Manifest**:
   ```bash
   cd 00_manifest
   python3 generate_manifest.py  # Use script from README.md
   ```

2. **Copy Non-Variant Files** (if not already done):
   ```bash
   # Copy IDF docs, examples, etc. from 16n8r
   # Most files are chip-level, not variant-specific
   ```

3. **Create Static VecDB**:
   ```bash
   # Use your existing VecDB creation process
   # The enhanced tags will improve retrieval quality
   ```

## Board Facts Usage

The `board_facts.json` file provides machine-readable facts for fast queries:

```json
{
  "gpio": {
    "led": {"pin": 48, "type": "rgb"},
    "button": {"pin": 0, "type": "button"}
  },
  "pwm": {
    "default_pin": 48,
    "timer": 0,
    "channel": 0
  },
  "adc": {
    "default_pin": 1,
    "channel": "ADC1_CH0"
  },
  "tasks": {
    "gpio": {"supported": true},
    "pwm": {"supported": true},
    "adc": {"supported": true},
    "wifi_station": {"supported": true}
  }
}
```

This complements the board definition JSON schema and provides a fast lookup for common queries.

## Tag Strategy

Files are tagged with:
- **Task tags**: `task:gpio`, `task:pwm`, `task:adc`, `task:wifi_station`
- **Category tags**: `board`, `gpio`, `pwm`, `adc`, `wifi`, `troubleshooting`, `error`, `example`, `api`
- **Error tags**: `build`, `flash`, `runtime`, `memory`, `power`, `interrupt`

This enables precise filtering in VecDB queries.
