//! Context management for session conversations
//!
//! This module provides:
//! - Token estimation for messages
//! - Context compaction (summarizing old messages when context grows too large)
//! - Pre-compaction memory flush (AI extracts memories before summarization)
//! - Session memory hooks (saving session summaries on reset)

use crate::ai::{AiClient, Message, MessageRole};
use crate::config::MemoryConfig;
use crate::db::Database;
use crate::models::{MemoryType, SessionMessage};
use crate::models::session_message::MessageRole as DbMessageRole;
use chrono::Utc;
use regex::Regex;
use std::sync::Arc;

/// Default context window size (Claude 3.5 Sonnet)
pub const DEFAULT_MAX_CONTEXT_TOKENS: i32 = 100_000;

/// Reserve tokens for system prompt and output
pub const DEFAULT_RESERVE_TOKENS: i32 = 20_000;

/// Minimum messages to keep after compaction
pub const MIN_KEEP_RECENT_MESSAGES: i32 = 5;

/// Default number of messages to keep after compaction
pub const DEFAULT_KEEP_RECENT_MESSAGES: i32 = 10;

/// Estimate token count for a string
/// Uses a simple heuristic: ~4 characters per token for English text
/// This is a rough approximation - actual tokenization varies by model
pub fn estimate_tokens(text: &str) -> i32 {
    // Average ~4 chars per token, but account for whitespace and punctuation
    let chars = text.chars().count();
    ((chars as f64) / 3.5).ceil() as i32
}

/// Estimate total tokens for a list of messages
pub fn estimate_messages_tokens(messages: &[SessionMessage]) -> i32 {
    messages.iter()
        .map(|m| {
            // Add overhead for role prefix (~4 tokens)
            estimate_tokens(&m.content) + 4
        })
        .sum()
}

/// Context manager for handling session context and compaction
pub struct ContextManager {
    db: Arc<Database>,
    /// Maximum context window size in tokens
    max_context_tokens: i32,
    /// Tokens to reserve for system prompt and output
    reserve_tokens: i32,
    /// Number of recent messages to keep after compaction
    keep_recent_messages: i32,
    /// Memory configuration
    memory_config: MemoryConfig,
}

impl ContextManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            max_context_tokens: DEFAULT_MAX_CONTEXT_TOKENS,
            reserve_tokens: DEFAULT_RESERVE_TOKENS,
            keep_recent_messages: DEFAULT_KEEP_RECENT_MESSAGES,
            memory_config: MemoryConfig::from_env(),
        }
    }

    pub fn with_max_context(mut self, tokens: i32) -> Self {
        self.max_context_tokens = tokens;
        self
    }

    pub fn with_reserve_tokens(mut self, tokens: i32) -> Self {
        self.reserve_tokens = tokens;
        self
    }

    pub fn with_keep_recent(mut self, count: i32) -> Self {
        self.keep_recent_messages = count.max(MIN_KEEP_RECENT_MESSAGES);
        self
    }

    pub fn with_memory_config(mut self, config: MemoryConfig) -> Self {
        self.memory_config = config;
        self
    }

    /// Check if compaction is needed for a session
    pub fn needs_compaction(&self, session_id: i64) -> bool {
        if let Ok(session) = self.db.get_chat_session(session_id) {
            if let Some(session) = session {
                let threshold = session.max_context_tokens - self.reserve_tokens;
                return session.context_tokens > threshold;
            }
        }
        false
    }

    /// Get available context budget (after reserving tokens)
    pub fn get_context_budget(&self, session_id: i64) -> i32 {
        if let Ok(Some(session)) = self.db.get_chat_session(session_id) {
            return session.max_context_tokens - self.reserve_tokens - session.context_tokens;
        }
        self.max_context_tokens - self.reserve_tokens
    }

    /// Build conversation context for AI, including compaction summary if present
    pub fn build_context(&self, session_id: i64, limit: i32) -> Vec<SessionMessage> {
        // Get recent messages
        let messages = self.db.get_recent_session_messages(session_id, limit)
            .unwrap_or_default();

        messages
    }

    /// Get compaction summary for a session (if any)
    pub fn get_compaction_summary(&self, session_id: i64) -> Option<String> {
        self.db.get_session_compaction_summary(session_id).ok().flatten()
    }

    /// Phase 1: Flush memories before compaction
    /// Gives the AI a "silent turn" to extract important memories from the conversation
    /// that would otherwise be lost during summarization.
    pub async fn flush_memories_before_compaction(
        &self,
        session_id: i64,
        client: &AiClient,
        identity_id: Option<&str>,
        messages_to_compact: &[SessionMessage],
    ) -> Result<Vec<i64>, String> {
        if messages_to_compact.is_empty() {
            return Ok(vec![]);
        }

        log::info!("[PRE_FLUSH] Starting memory flush for session {} ({} messages)",
            session_id, messages_to_compact.len());

        // Build conversation text
        let conversation_text = messages_to_compact.iter()
            .map(|m| {
                let role = match m.role {
                    DbMessageRole::User => "User",
                    DbMessageRole::Assistant => "Assistant",
                    DbMessageRole::System => "System",
                };
                format!("{}: {}", role, m.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        // Prompt the AI to extract memories
        let flush_prompt = format!(
            "Before this conversation history is summarized and archived, extract any important information that should be remembered. \
            Use these markers to save memories:\n\n\
            - [PREFERENCE: description] - User preferences (e.g., coding style, communication preferences)\n\
            - [FACT: description] - Facts about the user (e.g., name, job, location)\n\
            - [TASK: description] - Commitments, todos, or tasks mentioned\n\
            - [REMEMBER: description] - Any other important information worth remembering\n\
            - [REMEMBER_IMPORTANT: description] - Critical information that must be preserved\n\n\
            Only extract genuinely important information. Don't save trivial details.\n\
            If nothing important needs to be saved, respond with just: NO_MEMORIES_NEEDED\n\n\
            Conversation to analyze:\n{}\n\n\
            Extract memories:",
            conversation_text
        );

        let flush_messages = vec![
            Message {
                role: MessageRole::System,
                content: "You are a memory extraction assistant. Analyze conversations and extract important information that should be preserved as long-term memories.".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: flush_prompt,
            },
        ];

        let response = client.generate_text(flush_messages).await
            .map_err(|e| format!("Failed to generate memory flush: {}", e))?;

        if response.contains("NO_MEMORIES_NEEDED") {
            log::info!("[PRE_FLUSH] No memories to extract for session {}", session_id);
            return Ok(vec![]);
        }

        // Parse and create memories from the response
        let created_ids = self.parse_and_create_flush_memories(
            &response,
            identity_id,
            session_id,
        )?;

        log::info!("[PRE_FLUSH] Extracted {} memories for session {}", created_ids.len(), session_id);

        // Update last_flush_at timestamp
        if let Err(e) = self.db.update_session_last_flush(session_id) {
            log::warn!("[PRE_FLUSH] Failed to update last_flush_at: {}", e);
        }

        Ok(created_ids)
    }

    /// Parse memory markers from flush response and create memories
    fn parse_and_create_flush_memories(
        &self,
        response: &str,
        identity_id: Option<&str>,
        session_id: i64,
    ) -> Result<Vec<i64>, String> {
        let mut created_ids = Vec::new();
        let today = Utc::now().date_naive();

        // Memory marker patterns for pre-flush
        let patterns = [
            (Regex::new(r"\[PREFERENCE:\s*(.+?)\]").unwrap(), MemoryType::Preference, 7, "explicit"),
            (Regex::new(r"\[FACT:\s*(.+?)\]").unwrap(), MemoryType::Fact, 7, "explicit"),
            (Regex::new(r"\[TASK:\s*(.+?)\]").unwrap(), MemoryType::Task, 8, "explicit"),
            (Regex::new(r"\[REMEMBER:\s*(.+?)\]").unwrap(), MemoryType::LongTerm, 7, "explicit"),
            (Regex::new(r"\[REMEMBER_IMPORTANT:\s*(.+?)\]").unwrap(), MemoryType::LongTerm, 9, "explicit"),
        ];

        for (pattern, memory_type, importance, source_type) in &patterns {
            for cap in pattern.captures_iter(response) {
                if let Some(content) = cap.get(1) {
                    let content_str = content.as_str().trim();
                    if !content_str.is_empty() {
                        match self.db.create_memory_extended(
                            *memory_type,
                            content_str,
                            Some("pre_compaction_flush"),
                            None,
                            *importance,
                            identity_id,
                            Some(session_id),
                            None,
                            None,
                            if *memory_type == MemoryType::Task { Some(today) } else { None },
                            None,
                            None, // entity_type
                            None, // entity_name
                            Some(1.0), // confidence
                            Some(source_type), // source_type
                            None, // valid_from
                            None, // valid_until
                            None, // temporal_type
                        ) {
                            Ok(memory) => {
                                log::info!("[PRE_FLUSH] Created {} memory: {}", memory_type.as_str(), content_str);
                                created_ids.push(memory.id);
                            }
                            Err(e) => {
                                log::error!("[PRE_FLUSH] Failed to create memory: {}", e);
                            }
                        }
                    }
                }
            }
        }

        Ok(created_ids)
    }

    /// Perform context compaction for a session
    /// Returns the number of messages compacted
    pub async fn compact_session(
        &self,
        session_id: i64,
        client: &AiClient,
        identity_id: Option<&str>,
    ) -> Result<i32, String> {
        // Get messages to compact (all except recent ones)
        let messages_to_compact = self.db.get_messages_for_compaction(session_id, self.keep_recent_messages)
            .map_err(|e| format!("Failed to get messages for compaction: {}", e))?;

        if messages_to_compact.is_empty() {
            log::info!("[COMPACTION] No messages to compact for session {}", session_id);
            return Ok(0);
        }

        let message_count = messages_to_compact.len() as i32;
        log::info!("[COMPACTION] Compacting {} messages for session {}", message_count, session_id);

        // Phase 1: Pre-compaction memory flush
        if self.memory_config.enable_pre_compaction_flush {
            match self.flush_memories_before_compaction(
                session_id,
                client,
                identity_id,
                &messages_to_compact,
            ).await {
                Ok(ids) => {
                    if !ids.is_empty() {
                        log::info!("[COMPACTION] Pre-flush saved {} memories", ids.len());
                    }
                }
                Err(e) => {
                    log::warn!("[COMPACTION] Pre-flush failed (continuing with compaction): {}", e);
                }
            }
        }

        // Build the conversation text for summarization
        let conversation_text = messages_to_compact.iter()
            .map(|m| {
                let role = match m.role {
                    DbMessageRole::User => "User",
                    DbMessageRole::Assistant => "Assistant",
                    DbMessageRole::System => "System",
                };
                format!("{}: {}", role, m.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        // Generate summary using AI
        let summary_prompt = format!(
            "Summarize the following conversation history concisely. \
            Focus on: key topics discussed, important decisions made, user preferences learned, \
            and any tasks or commitments. Keep it factual and under 500 words.\n\n\
            Conversation:\n{}\n\nSummary:",
            conversation_text
        );

        let summary_messages = vec![
            Message {
                role: MessageRole::System,
                content: "You are a helpful assistant that summarizes conversations accurately and concisely.".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: summary_prompt,
            },
        ];

        let summary = client.generate_text(summary_messages).await
            .map_err(|e| format!("Failed to generate compaction summary: {}", e))?;

        log::info!("[COMPACTION] Generated summary ({} chars) for session {}", summary.len(), session_id);

        // Store the compaction summary as a memory
        let compaction_memory = self.db.create_memory(
            MemoryType::Compaction,
            &summary,
            Some("compaction"),
            None,
            10, // High importance
            identity_id,
            Some(session_id),
            None,
            None,
            None,
            None,
        ).map_err(|e| format!("Failed to store compaction memory: {}", e))?;

        // Update session with compaction reference
        self.db.set_session_compaction(session_id, compaction_memory.id)
            .map_err(|e| format!("Failed to update session compaction: {}", e))?;

        // Delete the compacted messages
        let deleted = self.db.delete_compacted_messages(session_id, self.keep_recent_messages)
            .map_err(|e| format!("Failed to delete compacted messages: {}", e))?;

        log::info!("[COMPACTION] Deleted {} old messages for session {}", deleted, session_id);

        // Recalculate and update context tokens
        let remaining = self.db.get_session_messages(session_id).unwrap_or_default();
        let new_token_count = estimate_messages_tokens(&remaining) + estimate_tokens(&summary);
        self.db.update_session_context_tokens(session_id, new_token_count)
            .map_err(|e| format!("Failed to update context tokens: {}", e))?;

        Ok(message_count)
    }

    /// Update context tokens after adding a message
    pub fn update_context_tokens(&self, session_id: i64, message_tokens: i32) {
        if let Ok(Some(session)) = self.db.get_chat_session(session_id) {
            let new_total = session.context_tokens + message_tokens;
            let _ = self.db.update_session_context_tokens(session_id, new_total);
        }
    }
}

/// Save session summary before reset (session memory hook)
pub async fn save_session_memory(
    db: &Arc<Database>,
    client: &AiClient,
    session_id: i64,
    identity_id: Option<&str>,
    message_limit: i32,
) -> Result<i64, String> {
    // Get recent messages from the session
    let messages = db.get_recent_session_messages(session_id, message_limit)
        .map_err(|e| format!("Failed to get session messages: {}", e))?;

    if messages.is_empty() {
        return Err("No messages to summarize".to_string());
    }

    log::info!("[SESSION_MEMORY] Saving session memory for {} messages", messages.len());

    // Build conversation text
    let conversation_text = messages.iter()
        .map(|m| {
            let role = match m.role {
                DbMessageRole::User => "User",
                DbMessageRole::Assistant => "Assistant",
                DbMessageRole::System => "System",
            };
            format!("{}: {}", role, m.content)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Generate summary and title using AI
    let summary_prompt = format!(
        "Analyze this conversation and provide:\n\
        1. A short descriptive title (5-10 words, suitable for a filename)\n\
        2. A brief summary of the key points discussed\n\n\
        Format your response as:\n\
        TITLE: <title here>\n\
        SUMMARY: <summary here>\n\n\
        Conversation:\n{}",
        conversation_text
    );

    let ai_messages = vec![
        Message {
            role: MessageRole::System,
            content: "You summarize conversations concisely. Respond only with the requested TITLE and SUMMARY format.".to_string(),
        },
        Message {
            role: MessageRole::User,
            content: summary_prompt,
        },
    ];

    let response = client.generate_text(ai_messages).await
        .map_err(|e| format!("Failed to generate session summary: {}", e))?;

    // Parse title and summary from response
    let (title, summary) = parse_title_summary(&response);

    // Create the session summary memory
    let today = Utc::now().date_naive();
    let content = format!("## {}\n\n{}", title, summary);

    let memory = db.create_memory(
        MemoryType::SessionSummary,
        &content,
        Some("session_summary"),
        Some(&title),
        8, // High importance for session summaries
        identity_id,
        Some(session_id),
        None,
        None,
        Some(today),
        None,
    ).map_err(|e| format!("Failed to create session memory: {}", e))?;

    log::info!("[SESSION_MEMORY] Created session summary: {} (id={})", title, memory.id);

    Ok(memory.id)
}

/// Parse title and summary from AI response
fn parse_title_summary(response: &str) -> (String, String) {
    let mut title = String::new();
    let mut summary = String::new();

    for line in response.lines() {
        let line = line.trim();
        if line.to_uppercase().starts_with("TITLE:") {
            title = line[6..].trim().to_string();
        } else if line.to_uppercase().starts_with("SUMMARY:") {
            summary = line[8..].trim().to_string();
        } else if !title.is_empty() && !line.to_uppercase().starts_with("SUMMARY:") && summary.is_empty() {
            // Multi-line handling - append to title if before summary
        } else if !summary.is_empty() {
            // Append to summary if we're past the SUMMARY: prefix
            if !summary.is_empty() {
                summary.push(' ');
            }
            summary.push_str(line);
        }
    }

    // Fallbacks
    if title.is_empty() {
        title = format!("Session {}", Utc::now().format("%Y-%m-%d %H:%M"));
    }
    if summary.is_empty() {
        summary = response.chars().take(500).collect();
    }

    (title, summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        // Roughly 4 chars per token
        assert!(estimate_tokens("hello") >= 1);
        assert!(estimate_tokens("hello world") >= 2);

        // Longer text
        let long_text = "This is a longer piece of text that should estimate to roughly 10-15 tokens based on our heuristic.";
        let tokens = estimate_tokens(long_text);
        assert!(tokens >= 10 && tokens <= 50);
    }

    #[test]
    fn test_parse_title_summary() {
        let response = "TITLE: Discussion about Rust programming\nSUMMARY: User asked about ownership and borrowing in Rust.";
        let (title, summary) = parse_title_summary(response);
        assert_eq!(title, "Discussion about Rust programming");
        assert!(summary.contains("ownership"));
    }
}
