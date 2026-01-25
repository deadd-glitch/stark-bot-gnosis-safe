# Debug & Logs Pages Implementation Plan

## Overview
Add Debug and Logs navigation pills and pages to StarkBot, inspired by Clawdbot's implementation. Both pages require authentication.

## Clawdbot Reference

### Debug Page Features
- **System Snapshots**: Status, health, heartbeat info displayed as JSON
- **Manual RPC**: Call gateway methods with custom JSON params
- **Models List**: Display available AI models
- **Event Log**: Real-time gateway events with timestamps

### Logs Page Features
- **Log Display**: Timestamp, level, subsystem, message
- **Level Filters**: Toggle trace/debug/info/warn/error/fatal
- **Text Search**: Filter by message content or subsystem
- **Auto-follow**: Live tail of logs
- **Export**: Download filtered logs

---

## Implementation Plan

### Phase 1: Backend - Debug Endpoints

**New file: `controllers/debug.rs`**

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/debug/status` | GET | System status snapshot (uptime, version, memory) |
| `/api/debug/health` | GET | Health check with component statuses |
| `/api/debug/stats` | GET | Database stats (sessions, messages, memories counts) |
| `/api/debug/gateway/events` | GET | Recent gateway events (last 100) |
| `/api/debug/rpc` | POST | Execute manual gateway RPC call |

**Data structures:**
```rust
// SystemStatus
struct SystemStatus {
    version: String,
    uptime_seconds: u64,
    started_at: DateTime<Utc>,
    rust_version: String,
}

// HealthCheck
struct HealthCheck {
    status: String,  // "healthy" | "degraded" | "unhealthy"
    database: ComponentHealth,
    gateway: ComponentHealth,
    ai_provider: ComponentHealth,
}

// DatabaseStats
struct DatabaseStats {
    total_sessions: i64,
    active_sessions: i64,
    total_messages: i64,
    total_memories: i64,
    total_identities: i64,
}

// GatewayEvent (already exists, reuse from gateway/events.rs)

// RpcRequest
struct RpcRequest {
    method: String,
    params: serde_json::Value,
}
```

---

### Phase 2: Backend - Logs Endpoints

**New file: `controllers/logs.rs`**

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/logs` | GET | Get recent logs with optional filters |
| `/api/logs/export` | GET | Download logs as file |

**Query parameters for `/api/logs`:**
- `limit` (default: 100, max: 1000)
- `level` (comma-separated: info,warn,error)
- `search` (text filter)
- `since` (ISO8601 timestamp)

**Log storage approach:**
- Store logs in `app_logs` SQLite table (simple approach)
- Or read from log file if using file-based logging

**New table: `app_logs`**
```sql
CREATE TABLE IF NOT EXISTS app_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    level TEXT NOT NULL,
    subsystem TEXT NOT NULL,
    message TEXT NOT NULL,
    data TEXT,  -- JSON additional data
    created_at TEXT NOT NULL
);
CREATE INDEX idx_logs_level ON app_logs(level);
CREATE INDEX idx_logs_created_at ON app_logs(created_at);
```

**Custom log handler:**
Create a database logger that writes to SQLite alongside console output.

---

### Phase 3: Backend - Database Methods

**Add to `db/sqlite.rs`:**

```rust
// Debug stats
fn get_database_stats() -> DatabaseStats
fn get_active_session_count() -> i64
fn get_total_message_count() -> i64
fn get_total_memory_count() -> i64
fn get_total_identity_count() -> i64

// Logs
fn insert_log(level, subsystem, message, data) -> Log
fn get_logs(limit, levels, search, since) -> Vec<Log>
fn get_logs_for_export(levels, search, since) -> Vec<Log>
fn cleanup_old_logs(days_to_keep) -> i64
```

---

### Phase 4: Frontend - Debug Page

**New file: `stark-frontend/debug.html`**

Layout:
```
┌─────────────────────────────────────────────────────┐
│  Sidebar (same as other pages)                      │
├─────────────────────────────────────────────────────┤
│  Header: "Debug" with Refresh button                │
├─────────────────────────────────────────────────────┤
│  ┌──────────────────┐  ┌──────────────────┐        │
│  │ System Status    │  │ Health Check     │        │
│  │ - Version        │  │ - Database: ✓    │        │
│  │ - Uptime         │  │ - Gateway: ✓     │        │
│  │ - Started at     │  │ - AI Provider: ✓ │        │
│  └──────────────────┘  └──────────────────┘        │
├─────────────────────────────────────────────────────┤
│  Database Stats                                     │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│  │Sessions │ │Messages │ │Memories │ │Identities│  │
│  │   123   │ │  4,567  │ │   890   │ │   45    │  │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘  │
├─────────────────────────────────────────────────────┤
│  Manual RPC Call                                    │
│  Method: [________________]                         │
│  Params: [________________] (JSON)                  │
│  [Call]                                             │
│  Response: {...}                                    │
├─────────────────────────────────────────────────────┤
│  Recent Gateway Events                              │
│  ┌─────────────────────────────────────────────┐   │
│  │ 12:34:56 | channel.message | {...}          │   │
│  │ 12:34:55 | agent.response  | {...}          │   │
│  │ ...                                          │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

**New file: `stark-frontend/js/debug.js`**
- Load system status, health, stats on page load
- Auto-refresh every 10 seconds (optional toggle)
- Manual RPC form handling
- Event log display with expand/collapse for JSON

---

### Phase 5: Frontend - Logs Page

**New file: `stark-frontend/logs.html`**

Layout:
```
┌─────────────────────────────────────────────────────┐
│  Sidebar (same as other pages)                      │
├─────────────────────────────────────────────────────┤
│  Header: "Logs" + [Refresh] [Export] [Auto-follow□] │
├─────────────────────────────────────────────────────┤
│  Filters:                                           │
│  Search: [__________________]                       │
│  Levels: [✓ Info] [✓ Warn] [✓ Error] [□ Debug]     │
├─────────────────────────────────────────────────────┤
│  Log Entries (scrollable)                           │
│  ┌─────────────────────────────────────────────┐   │
│  │ 2024-01-15 12:34:56 | INFO  | dispatcher    │   │
│  │ Generated response for user123              │   │
│  ├─────────────────────────────────────────────┤   │
│  │ 2024-01-15 12:34:55 | WARN  | gateway       │   │
│  │ Connection retry attempt 2                  │   │
│  ├─────────────────────────────────────────────┤   │
│  │ 2024-01-15 12:34:50 | ERROR | ai_client     │   │
│  │ API request timeout                         │   │
│  └─────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────┤
│  Showing 100 of 1,234 logs                          │
└─────────────────────────────────────────────────────┘
```

**New file: `stark-frontend/js/logs.js`**
- Load logs on page load with default filters
- Filter controls (level checkboxes, search input)
- Debounced search
- Auto-follow mode (poll every 2 seconds)
- Export to JSON file
- Level-based color coding (info=blue, warn=yellow, error=red)

---

### Phase 6: Update Navigation

**Update all HTML files** to add Debug and Logs nav items:

After "Agent Settings", before the hidden API Keys section:
```html
<a href="/debug.html" class="flex items-center gap-3 px-4 py-3 text-slate-400 hover:bg-slate-700/50 hover:text-white rounded-lg font-medium transition-colors">
    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
    </svg>
    Debug
</a>
<a href="/logs.html" class="flex items-center gap-3 px-4 py-3 text-slate-400 hover:bg-slate-700/50 hover:text-white rounded-lg font-medium transition-colors">
    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
    </svg>
    Logs
</a>
```

Files to update:
- `dashboard.html`
- `agent-chat.html`
- `channels.html`
- `agent-settings.html`
- `api-keys.html`

---

### Phase 7: Register Routes

**Update `main.rs`:**
```rust
.configure(controllers::debug::config)
.configure(controllers::logs::config)
```

**Update `controllers/mod.rs`:**
```rust
pub mod debug;
pub mod logs;
```

---

## Files to Create

| File | Purpose |
|------|---------|
| `controllers/debug.rs` | Debug API endpoints |
| `controllers/logs.rs` | Logs API endpoints |
| `models/log.rs` | Log model and types |
| `stark-frontend/debug.html` | Debug page UI |
| `stark-frontend/logs.html` | Logs page UI |
| `stark-frontend/js/debug.js` | Debug page logic |
| `stark-frontend/js/logs.js` | Logs page logic |

## Files to Modify

| File | Changes |
|------|---------|
| `db/sqlite.rs` | Add logs table, stats methods |
| `models/mod.rs` | Add log module export |
| `controllers/mod.rs` | Add debug, logs modules |
| `main.rs` | Register new routes |
| `dashboard.html` | Add nav items |
| `agent-chat.html` | Add nav items |
| `channels.html` | Add nav items |
| `agent-settings.html` | Add nav items |
| `api-keys.html` | Add nav items |

---

## Implementation Order

1. **Backend foundation**: Add log table schema and models
2. **Debug controller**: Implement status/health/stats endpoints
3. **Logs controller**: Implement log retrieval endpoints
4. **Database methods**: Add all required DB functions
5. **Debug frontend**: Create debug.html and debug.js
6. **Logs frontend**: Create logs.html and logs.js
7. **Navigation update**: Add Debug/Logs to all pages
8. **Route registration**: Wire up controllers in main.rs
9. **Testing**: Verify all endpoints and UI

---

## Optional Enhancements (Future)

- WebSocket for real-time log streaming
- Log retention policy (auto-cleanup)
- Log search with regex support
- Debug mode toggle for verbose logging
- Performance metrics in debug panel
- Database query profiling
