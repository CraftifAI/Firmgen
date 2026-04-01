#!/usr/bin/env python3
"""
Generate manifest.jsonl for n32r8v variant from 16n8r manifest.
Enhances tags with task-oriented and category tags.
"""

import json
import sys
from pathlib import Path

def enhance_tags(path, existing_tags):
    """Add task-oriented and category tags based on file path"""
    tags = list(existing_tags) if existing_tags else []
    path_lower = path.lower()
    
    # Category tags
    if 'board' in path:
        tags.append('board')
    if 'pinout' in path_lower:
        tags.append('pinout')
    if 'gpio' in path_lower:
        tags.append('gpio')
        if 'example' in path_lower or ('main' in path_lower and '.c' in path_lower):
            tags.append('task:gpio')
    if 'ledc' in path_lower or 'pwm' in path_lower:
        tags.append('pwm')
        if 'example' in path_lower or ('main' in path_lower and '.c' in path_lower):
            tags.append('task:pwm')
    if 'adc' in path_lower:
        tags.append('adc')
        if 'example' in path_lower or ('main' in path_lower and '.c' in path_lower):
            tags.append('task:adc')
    if 'wifi' in path_lower:
        tags.append('wifi')
        if 'station' in path_lower and ('example' in path_lower or ('main' in path_lower and '.c' in path_lower)):
            tags.append('task:wifi_station')
    if 'known_issues' in path or 'troubleshooting' in path_lower:
        tags.append('troubleshooting')
    if 'error' in path_lower:
        tags.append('error')
        if 'build' in path_lower:
            tags.append('build')
        if 'flash' in path_lower:
            tags.append('flash')
        if 'runtime' in path_lower:
            tags.append('runtime')
        if 'memory' in path_lower or 'heap' in path_lower or 'stack' in path_lower:
            tags.append('memory')
        if 'power' in path_lower or 'brownout' in path_lower:
            tags.append('power')
        if 'interrupt' in path_lower:
            tags.append('interrupt')
    if 'example' in path_lower:
        tags.append('example')
    if 'api-reference' in path or 'api-guides' in path:
        tags.append('api')
    if 'peripherals' in path:
        tags.append('peripherals')
    if 'machine_readable' in path_lower or 'facts' in path_lower:
        tags.append('machine_readable')
    
    return sorted(list(set(tags)))  # Remove duplicates and sort

def main():
    # Paths
    script_dir = Path(__file__).parent
    kb_dir = script_dir.parent
    source_manifest = kb_dir.parent / 'esp32-s3-devkitc-16n8r' / '00_manifest' / 'manifest.jsonl'
    target_manifest = script_dir / 'manifest.jsonl'
    
    if not source_manifest.exists():
        print(f"Error: Source manifest not found: {source_manifest}")
        sys.exit(1)
    
    print(f"Reading source manifest: {source_manifest}")
    print(f"Writing target manifest: {target_manifest}")
    
    entries_processed = 0
    entries_updated = 0
    
    with open(source_manifest, 'r') as f_in, open(target_manifest, 'w') as f_out:
        for line in f_in:
            line = line.strip()
            if not line:
                continue
                
            try:
                entry = json.loads(line)
                entries_processed += 1
                
                # Update variant and board
                entry['variant'] = 'n32r8v'
                entry['board'] = 'esp32-s3-devkitc-1'
                
                # Enhance tags
                original_tags = entry.get('tags', [])
                enhanced_tags = enhance_tags(entry['path'], original_tags)
                
                if enhanced_tags != original_tags:
                    entries_updated += 1
                
                entry['tags'] = enhanced_tags
                
                # Write updated entry
                f_out.write(json.dumps(entry) + '\n')
                
            except json.JSONDecodeError as e:
                print(f"Warning: Skipping invalid JSON line: {e}")
                continue
    
    print(f"\nDone!")
    print(f"  Processed: {entries_processed} entries")
    print(f"  Updated tags: {entries_updated} entries")
    print(f"  Output: {target_manifest}")

if __name__ == '__main__':
    main()
