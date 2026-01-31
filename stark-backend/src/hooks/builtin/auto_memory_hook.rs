//! Auto-memory hook - Automatically stores ephemeral memories when tools complete
//!
//! This hook subscribes to AfterToolCall events and creates short-lived memories
//! for tracked tools (agent_send, edit_file, write_file, web_fetch).

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::Database;
use crate::hooks::types::{Hook, HookContext, HookEvent, HookPriority, HookResult};
use crate::models::MemoryType;

/// Configuration for tracked tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedTools {
    pub agent_send: bool,
    pub edit_file: bool,
    pub write_file: bool,
    pub web_fetch: bool,
}

impl Default for TrackedTools {
    fn default() -> Self {
        Self {
            agent_send: true,
            edit_file: true,
            write_file: true,
            web_fetch: true,
        }
    }
}

/// Configuration for the auto-memory hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoMemoryConfig {
    pub enabled: bool,
    /// Time-to-live in seconds (default: 3600 = 1 hour)
    pub ttl_secs: i64,
    pub tracked_tools: TrackedTools,
}

impl Default for AutoMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_secs: 3600, // 1 hour
            tracked_tools: TrackedTools::default(),
        }
    }
}

/// Hook that automatically creates ephemeral memories for tool activity
pub struct AutoMemoryHook {
    config: AutoMemoryConfig,
    db: Arc<Database>,
}

impl AutoMemoryHook {
    /// Create with database and default configuration
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            config: AutoMemoryConfig::default(),
            db,
        }
    }

    /// Create with custom configuration
    pub fn with_config(db: Arc<Database>, config: AutoMemoryConfig) -> Self {
        Self { config, db }
    }

    /// Check if a tool should be tracked based on configuration
    fn should_track(&self, tool_name: &str) -> bool {
        match tool_name {
            "agent_send" => self.config.tracked_tools.agent_send,
            "edit_file" => self.config.tracked_tools.edit_file,
            "write_file" => self.config.tracked_tools.write_file,
            "web_fetch" => self.config.tracked_tools.web_fetch,
            _ => false,
        }
    }

    /// Extract meaningful content from tool context and format as memory
    fn format_memory_content(&self, context: &HookContext) -> Option<String> {
        let tool_name = context.tool_name.as_ref()?;
        let tool_args = context.tool_args.as_ref();

        match tool_name.as_str() {
            "agent_send" => {
                // Format: [Messaging] Sent to {channel}: "{preview}"
                let channel = tool_args
                    .and_then(|args| args.get("channel"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let message = tool_args
                    .and_then(|args| args.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                // Truncate message to first 100 chars for preview
                let preview: String = message.chars().take(100).collect();
                let ellipsis = if message.len() > 100 { "..." } else { "" };
                Some(format!(
                    "[Messaging] Sent to {}: \"{}{}\"",
                    channel, preview, ellipsis
                ))
            }
            "edit_file" => {
                // Format: [File Edit] Modified '{path}'
                let path = tool_args
                    .and_then(|args| args.get("path"))
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        tool_args
                            .and_then(|args| args.get("file_path"))
                            .and_then(|v| v.as_str())
                    })
                    .unwrap_or("unknown");
                Some(format!("[File Edit] Modified '{}'", path))
            }
            "write_file" => {
                // Format: [File Write] Wrote '{path}' ({lines} lines)
                let path = tool_args
                    .and_then(|args| args.get("path"))
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        tool_args
                            .and_then(|args| args.get("file_path"))
                            .and_then(|v| v.as_str())
                    })
                    .unwrap_or("unknown");
                let content = tool_args
                    .and_then(|args| args.get("content"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let lines = content.lines().count();
                Some(format!("[File Write] Wrote '{}' ({} lines)", path, lines))
            }
            "web_fetch" => {
                // Format: [Web Fetch] Retrieved from '{domain}'
                let url = tool_args
                    .and_then(|args| args.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                // Extract domain from URL
                let domain = url::Url::parse(url)
                    .ok()
                    .and_then(|u| u.host_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| url.to_string());
                Some(format!("[Web Fetch] Retrieved from '{}'", domain))
            }
            _ => None,
        }
    }

    /// Create the ephemeral memory in the database
    fn create_memory(&self, context: &HookContext, content: String) {
        let tool_name = context.tool_name.clone().unwrap_or_default();
        let expires_at = Utc::now() + Duration::seconds(self.config.ttl_secs);

        let result = self.db.create_memory(
            MemoryType::DailyLog,
            &content,
            Some("auto_tool_activity"),
            Some(&tool_name), // tag with tool name
            3,                // low importance (ephemeral)
            None,             // identity_id
            context.session_id,
            None, // source_channel_type
            None, // source_message_id
            Some(Utc::now().date_naive()),
            Some(expires_at),
        );

        match result {
            Ok(memory) => {
                log::info!(
                    "[AutoMemoryHook] Created ephemeral memory id={} for tool '{}', expires at {}",
                    memory.id,
                    tool_name,
                    expires_at
                );
            }
            Err(e) => {
                log::error!(
                    "[AutoMemoryHook] Failed to create memory for tool '{}': {}",
                    tool_name,
                    e
                );
            }
        }
    }
}

#[async_trait]
impl Hook for AutoMemoryHook {
    fn id(&self) -> &str {
        "builtin.auto_memory"
    }

    fn name(&self) -> &str {
        "Auto-Memory Hook"
    }

    fn description(&self) -> &str {
        "Automatically stores ephemeral memories when tracked tools complete"
    }

    fn events(&self) -> Vec<HookEvent> {
        vec![HookEvent::AfterToolCall]
    }

    fn priority(&self) -> HookPriority {
        // Run after other hooks complete
        HookPriority::Low
    }

    fn enabled(&self) -> bool {
        self.config.enabled
    }

    async fn execute(&self, context: &mut HookContext) -> HookResult {
        // Only process AfterToolCall events
        if context.event != HookEvent::AfterToolCall {
            return HookResult::Continue(None);
        }

        // Check if this tool should be tracked
        let tool_name = match &context.tool_name {
            Some(name) if self.should_track(name) => {
                log::info!("[AutoMemoryHook] Processing tracked tool: {}", name);
                name.clone()
            }
            Some(name) => {
                log::debug!("[AutoMemoryHook] Tool '{}' not tracked, skipping", name);
                return HookResult::Continue(None);
            }
            _ => return HookResult::Continue(None),
        };

        // Format the memory content
        let content = match self.format_memory_content(context) {
            Some(c) => c,
            None => {
                log::warn!(
                    "[AutoMemoryHook] Could not format content for tool '{}'",
                    tool_name
                );
                return HookResult::Continue(None);
            }
        };

        // Create the memory (synchronously, but should be fast)
        log::info!("[AutoMemoryHook] Creating ephemeral memory: {}", content);
        self.create_memory(context, content);

        HookResult::Continue(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_db() -> Arc<Database> {
        Arc::new(Database::new(":memory:").expect("Failed to create test database"))
    }

    #[tokio::test]
    async fn test_hook_tracks_agent_send() {
        let db = create_test_db();
        let hook = AutoMemoryHook::new(db);

        let mut context = HookContext::new(HookEvent::AfterToolCall)
            .with_tool(
                "agent_send".to_string(),
                json!({
                    "channel": "discord",
                    "message": "Hello world!"
                }),
            )
            .with_channel(123, Some(456));

        let result = hook.execute(&mut context).await;
        assert!(result.should_continue());
    }

    #[tokio::test]
    async fn test_hook_ignores_untracked_tools() {
        let db = create_test_db();
        let hook = AutoMemoryHook::new(db);

        let mut context = HookContext::new(HookEvent::AfterToolCall)
            .with_tool("some_other_tool".to_string(), json!({}));

        let result = hook.execute(&mut context).await;
        assert!(result.should_continue());
    }

    #[tokio::test]
    async fn test_memory_content_formatting() {
        let db = create_test_db();
        let hook = AutoMemoryHook::new(db);

        // Test agent_send formatting
        let context = HookContext::new(HookEvent::AfterToolCall).with_tool(
            "agent_send".to_string(),
            json!({
                "channel": "slack",
                "message": "Test message"
            }),
        );
        let content = hook.format_memory_content(&context);
        assert!(content.is_some());
        assert!(content.unwrap().contains("[Messaging]"));

        // Test edit_file formatting
        let context = HookContext::new(HookEvent::AfterToolCall).with_tool(
            "edit_file".to_string(),
            json!({
                "path": "/test/file.rs"
            }),
        );
        let content = hook.format_memory_content(&context);
        assert!(content.is_some());
        assert!(content.unwrap().contains("[File Edit]"));

        // Test write_file formatting
        let context = HookContext::new(HookEvent::AfterToolCall).with_tool(
            "write_file".to_string(),
            json!({
                "path": "/test/new.rs",
                "content": "line1\nline2\nline3"
            }),
        );
        let content = hook.format_memory_content(&context);
        assert!(content.is_some());
        let formatted = content.unwrap();
        assert!(formatted.contains("[File Write]"));
        assert!(formatted.contains("3 lines"));

        // Test web_fetch formatting
        let context = HookContext::new(HookEvent::AfterToolCall).with_tool(
            "web_fetch".to_string(),
            json!({
                "url": "https://example.com/api/data"
            }),
        );
        let content = hook.format_memory_content(&context);
        assert!(content.is_some());
        assert!(content.unwrap().contains("example.com"));
    }

    #[test]
    fn test_config_defaults() {
        let config = AutoMemoryConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ttl_secs, 3600);
        assert!(config.tracked_tools.agent_send);
        assert!(config.tracked_tools.edit_file);
        assert!(config.tracked_tools.write_file);
        assert!(config.tracked_tools.web_fetch);
    }
}
