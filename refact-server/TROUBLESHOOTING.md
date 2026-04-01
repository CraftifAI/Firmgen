# Troubleshooting Guide

## Issue 1: Database Connection Errors

**Error:**
```
No database available on 127.0.0.1:9042; error: Connection refused
```

**Solution:**

The Web UI requires a ScyllaDB/Cassandra database. You have two options:

### Option A: Start Database (Recommended for full features)

If you're using Docker, the database starts automatically. For local development:

1. **Install ScyllaDB/Cassandra:**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install cassandra
   
   # Or use Docker
   docker run -d --name scylla -p 9042:9042 scylladb/scylla
   ```

2. **Start the database:**
   ```bash
   sudo service cassandra start
   # Or if using Docker:
   docker start scylla
   ```

3. **Wait for database to be ready** (can take 30-60 seconds)

4. **Restart the Web UI**

### Option B: Skip Database (Limited features)

Set environment variable to use a dummy database:
```bash
export REFACT_DATABASE_HOST=""
python -m refact_webgui.webgui.webgui
```

**Note:** Some features (like Stats) won't work without a database, but basic features and Chat will work.

## Issue 2: Chat Tab Not Showing

**Symptoms:**
- Chat tab doesn't appear in navigation
- Browser console shows errors

**Solutions:**

1. **Check browser console for errors:**
   - Open browser Developer Tools (F12)
   - Check Console tab for JavaScript errors
   - Look for errors like "Failed to load plugin chat"

2. **Verify files exist:**
   ```bash
   ls -la refact-server/refact_webgui/webgui/static/tab-chat.*
   ```
   Should show:
   - `tab-chat.html`
   - `tab-chat.js`

3. **Clear browser cache:**
   - Hard refresh: Ctrl+Shift+R (Linux/Windows) or Cmd+Shift+R (Mac)
   - Or clear browser cache

4. **Check server logs:**
   - Look for errors when loading the chat tab
   - Check if static files are being served

5. **Verify plugin registration:**
   ```bash
   curl http://127.0.0.1:8008/list-plugins
   ```
   Should include `{"label": "Chat", "tab": "chat"}`

## Issue 3: Chat Tab Shows But Can't Connect to Agent

**Symptoms:**
- Chat tab loads but shows "Disconnected"
- Error: "Could not connect to refact agent"

**Solutions:**

1. **Make sure refact agent is running:**
   ```bash
   refact /path/to/workspace
   ```
   Keep this terminal open!

2. **Check agent port:**
   - Agent usually runs on port 8001
   - Check agent terminal output for: `HTTP server listening on 127.0.0.1:XXXX`
   - If different port, you may need to update `tab_chat.py`

3. **Test agent connection:**
   ```bash
   curl http://127.0.0.1:8001/v1/ping
   ```
   Should return: `{"message":"pong"}`

4. **Check firewall/network:**
   - Make sure localhost connections are allowed
   - Check if port 8001 is blocked

## Issue 4: Server Won't Start

**Symptoms:**
- Server exits immediately
- Port already in use errors

**Solutions:**

1. **Check if port 8008 is in use:**
   ```bash
   lsof -i :8008
   # Or
   netstat -tulpn | grep 8008
   ```
   Kill the process or use a different port:
   ```bash
   python -m refact_webgui.webgui.webgui --port 8080
   ```

2. **Check Python dependencies:**
   ```bash
   pip install -e refact-server/
   # Or install missing packages:
   pip install httpx fastapi uvicorn
   ```

3. **Check for import errors:**
   ```bash
   python -c "from refact_webgui.webgui.tab_chat import TabChatRouter"
   ```
   Should not show errors

## Quick Diagnostic Commands

```bash
# 1. Check if server is running
curl http://127.0.0.1:8008/ping

# 2. Check if agent is running
curl http://127.0.0.1:8001/v1/ping

# 3. Check plugins list
curl http://127.0.0.1:8008/list-plugins

# 4. Check if chat HTML is accessible
curl http://127.0.0.1:8008/tab-chat.html

# 5. Check if chat JS is accessible
curl http://127.0.0.1:8008/tab-chat.js
```

## Common Issues Summary

| Issue | Symptom | Solution |
|-------|---------|----------|
| Database errors | Connection refused on port 9042 | Start ScyllaDB/Cassandra or set `REFACT_DATABASE_HOST=""` |
| Chat tab missing | No "Chat" in navigation | Check browser console, clear cache, verify files exist |
| Can't connect to agent | "Disconnected" badge | Make sure `refact` command is running |
| Port in use | "Address already in use" | Use different port or kill existing process |
| Import errors | ModuleNotFoundError | Install dependencies: `pip install -e .` |

## Getting Help

1. Check browser console (F12) for JavaScript errors
2. Check server terminal for Python errors
3. Check agent terminal for connection issues
4. Verify all files exist and are accessible
5. Try clearing browser cache and hard refresh










