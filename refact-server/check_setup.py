#!/usr/bin/env python3
"""
Quick diagnostic script to check if the Web UI setup is correct
"""
import sys
import os
import requests
from pathlib import Path

def check_file_exists(filepath, description):
    """Check if a file exists"""
    if os.path.exists(filepath):
        print(f"✓ {description}: {filepath}")
        return True
    else:
        print(f"✗ {description}: {filepath} - NOT FOUND")
        return False

def check_server_running(port=8008):
    """Check if Web UI server is running"""
    try:
        response = requests.get(f"http://127.0.0.1:{port}/ping", timeout=2)
        if response.status_code == 200:
            print(f"✓ Web UI server is running on port {port}")
            return True
        else:
            print(f"✗ Web UI server returned status {response.status_code}")
            return False
    except requests.exceptions.ConnectionError:
        print(f"✗ Web UI server is NOT running on port {port}")
        return False
    except Exception as e:
        print(f"✗ Error checking Web UI server: {e}")
        return False

def check_agent_running(port=8001):
    """Check if refact agent is running"""
    try:
        response = requests.get(f"http://127.0.0.1:{port}/v1/ping", timeout=2)
        if response.status_code == 200:
            print(f"✓ Refact agent is running on port {port}")
            return True
        else:
            print(f"✗ Refact agent returned status {response.status_code}")
            return False
    except requests.exceptions.ConnectionError:
        print(f"✗ Refact agent is NOT running on port {port}")
        return False
    except Exception as e:
        print(f"✗ Error checking refact agent: {e}")
        return False

def check_plugins():
    """Check if plugins are accessible"""
    try:
        response = requests.get("http://127.0.0.1:8008/list-plugins", timeout=2)
        if response.status_code == 200:
            plugins = response.json()
            chat_plugin = [p for p in plugins if p.get('tab') == 'chat']
            if chat_plugin:
                print(f"✓ Chat plugin is registered: {chat_plugin[0]}")
                return True
            else:
                print(f"✗ Chat plugin is NOT in plugins list")
                print(f"  Available plugins: {[p.get('tab') for p in plugins]}")
                return False
        else:
            print(f"✗ Failed to get plugins list: {response.status_code}")
            return False
    except Exception as e:
        print(f"✗ Error checking plugins: {e}")
        return False

def check_static_files():
    """Check if static files are accessible"""
    files_to_check = [
        ('/tab-chat.html', 'Chat HTML file'),
        ('/tab-chat.js', 'Chat JavaScript file'),
        ('/tab-c2000.html', 'C2000 Tools HTML file'),
        ('/tab-c2000.js', 'C2000 Tools JavaScript file'),
    ]
    
    all_ok = True
    for path, description in files_to_check:
        try:
            response = requests.get(f"http://127.0.0.1:8008{path}", timeout=2)
            if response.status_code == 200:
                print(f"✓ {description} is accessible: {path}")
            else:
                print(f"✗ {description} returned status {response.status_code}: {path}")
                all_ok = False
        except Exception as e:
            print(f"✗ {description} error: {e}")
            all_ok = False
    
    return all_ok

def main():
    print("=" * 60)
    print("Refact Web UI Setup Diagnostic")
    print("=" * 60)
    print()
    
    # Check files exist
    print("1. Checking files exist...")
    base_dir = Path(__file__).parent
    files_ok = True
    files_ok &= check_file_exists(
        base_dir / "refact_webgui" / "webgui" / "static" / "tab-chat.html",
        "Chat HTML file"
    )
    files_ok &= check_file_exists(
        base_dir / "refact_webgui" / "webgui" / "static" / "tab-chat.js",
        "Chat JavaScript file"
    )
    files_ok &= check_file_exists(
        base_dir / "refact_webgui" / "webgui" / "tab_chat.py",
        "Chat router Python file"
    )
    print()
    
    # Check server
    print("2. Checking Web UI server...")
    server_ok = check_server_running()
    print()
    
    if not server_ok:
        print("⚠ Web UI server is not running. Please start it with:")
        print("   python -m refact_webgui.webgui.webgui")
        print()
        return
    
    # Check plugins
    print("3. Checking plugins...")
    plugins_ok = check_plugins()
    print()
    
    # Check static files
    print("4. Checking static files...")
    static_ok = check_static_files()
    print()
    
    # Check agent
    print("5. Checking refact agent...")
    agent_ok = check_agent_running()
    if not agent_ok:
        print("   ⚠ Refact agent is not running. Start it with:")
        print("      refact <workspace_path>")
    print()
    
    # Summary
    print("=" * 60)
    print("Summary:")
    print("=" * 60)
    if files_ok and server_ok and plugins_ok and static_ok:
        print("✓ All checks passed! The Web UI should be working.")
        if agent_ok:
            print("✓ Refact agent is also running - Chat should work!")
        else:
            print("⚠ Refact agent is not running - Chat won't work until you start it.")
    else:
        print("✗ Some checks failed. Please fix the issues above.")
        print()
        print("Common fixes:")
        if not server_ok:
            print("  - Start the Web UI: python -m refact_webgui.webgui.webgui")
        if not plugins_ok:
            print("  - Restart the Web UI server")
        if not static_ok:
            print("  - Check file permissions and paths")
        if not agent_ok:
            print("  - Start the refact agent: refact <workspace_path>")
    print()

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\n\nError: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)










