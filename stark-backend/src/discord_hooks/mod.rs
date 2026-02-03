//! Discord Hooks - Self-contained module for Discord command handling
//!
//! This module provides:
//! - Admin command detection and forwarding to the agent
//! - Limited command handling for regular users (register, status, help)
//! - Discord user profile management with public address registration
//! - Tool for resolving Discord mentions to registered public addresses
//!
//! ## Query Mode for Admins
//!
//! By default, admins must first say "@bot query" to activate query mode.
//! The next @mention from that admin will be treated as an agentic query.
//! This prevents accidental agent invocations.

pub mod commands;
pub mod config;
pub mod db;
pub mod tools;

use serenity::all::{Context, Message, UserId};
use std::collections::HashMap;
use std::sync::Mutex;

pub use config::DiscordHooksConfig;
pub use db::DiscordUserProfile;

// Track which admin users are currently listening for a query
// Key: discord_user_id, Value: true if waiting for next message to be treated as query
lazy_static::lazy_static! {
    static ref LISTENING_FOR_QUERY: Mutex<HashMap<String, bool>> = Mutex::new(HashMap::new());
}

/// Result of processing a Discord message
#[derive(Debug)]
pub struct ProcessResult {
    /// Whether the module handled the message
    pub handled: bool,
    /// Direct response to send (if handled internally)
    pub response: Option<String>,
    /// Request to forward to the agent (if admin command)
    pub forward_to_agent: Option<ForwardRequest>,
}

impl ProcessResult {
    /// Message was not handled (bot not mentioned, etc.)
    pub fn not_handled() -> Self {
        Self {
            handled: false,
            response: None,
            forward_to_agent: None,
        }
    }

    /// Message was handled with a direct response
    pub fn handled(response: String) -> Self {
        Self {
            handled: true,
            response: Some(response),
            forward_to_agent: None,
        }
    }

    /// Message should be forwarded to the agent
    pub fn forward_to_agent(request: ForwardRequest) -> Self {
        Self {
            handled: true,
            response: None,
            forward_to_agent: Some(request),
        }
    }
}

/// Request to forward a message to the agent dispatcher
#[derive(Debug, Clone)]
pub struct ForwardRequest {
    /// Command text (bot mention removed)
    pub text: String,
    /// Discord user ID
    pub user_id: String,
    /// Discord username
    pub user_name: String,
    /// Whether the user is an admin
    pub is_admin: bool,
}

/// Check if a user is in "listening for query" mode
fn is_listening_for_query(user_id: &str) -> bool {
    LISTENING_FOR_QUERY
        .lock()
        .unwrap()
        .get(user_id)
        .copied()
        .unwrap_or(false)
}

/// Set a user's "listening for query" state
fn set_listening_for_query(user_id: &str, listening: bool) {
    let mut map = LISTENING_FOR_QUERY.lock().unwrap();
    if listening {
        map.insert(user_id.to_string(), true);
        log::info!("Discord hooks: Admin {} is now listening for query", user_id);
    } else {
        map.remove(user_id);
        log::info!("Discord hooks: Admin {} query mode reset", user_id);
    }
}

/// Check if the message text contains the "query" keyword (case-insensitive)
fn contains_query_keyword(text: &str) -> bool {
    text.to_lowercase().contains("query")
}

/// Check if the bot is mentioned in a message
pub fn is_bot_mentioned(msg: &Message, bot_id: UserId) -> bool {
    msg.mentions.iter().any(|u| u.id == bot_id)
}

/// Extract command text from a message, removing bot mentions
pub fn extract_command_text(content: &str, bot_id: UserId) -> String {
    // Remove <@BOT_ID> and <@!BOT_ID> patterns
    let bot_mention = format!("<@{}>", bot_id);
    let bot_mention_nick = format!("<@!{}>", bot_id);

    content
        .replace(&bot_mention, "")
        .replace(&bot_mention_nick, "")
        .trim()
        .to_string()
}

/// Process a Discord message through the hooks system
///
/// Returns a ProcessResult indicating how to handle the message:
/// - `handled: false` - Bot not mentioned, fall through to existing behavior
/// - `handled: true` with `response` - Send the response directly
/// - `handled: true` with `forward_to_agent` - Forward to agent dispatcher
///
/// Note: The config is reloaded from the database on each message to pick up
/// changes to admin user IDs without requiring a channel restart.
pub async fn process(
    msg: &Message,
    ctx: &Context,
    db: &std::sync::Arc<crate::db::Database>,
    channel_id: i64,
) -> Result<ProcessResult, String> {
    // Reload config from database to pick up any changes
    let config = DiscordHooksConfig::from_channel_settings(db, channel_id);
    // Get bot's user ID by fetching current user info
    let current_user = ctx
        .http
        .get_current_user()
        .await
        .map_err(|e| format!("Failed to get current user: {}", e))?;
    let bot_id = current_user.id;

    // Check if bot is mentioned
    if !is_bot_mentioned(msg, bot_id) {
        // In DMs, we might want to process without mention
        // For now, require mention in all contexts
        return Ok(ProcessResult::not_handled());
    }

    // Extract command text (remove bot mention)
    let command_text = extract_command_text(&msg.content, bot_id);

    if command_text.is_empty() {
        return Ok(ProcessResult::handled(
            "Hi! I'm StarkBot. Try `@starkbot help` to see available commands.".to_string(),
        ));
    }

    // Get user info
    let user_id = msg.author.id.to_string();
    let user_name = msg.author.name.clone();

    // Get or create user profile
    if let Err(e) = db::get_or_create_profile(db, &user_id, &user_name) {
        log::error!("Discord hooks: Failed to get/create profile: {}", e);
        // Don't fail the whole request, just log it
    }

    // Check if user is admin
    let is_admin = config.is_admin(&user_id);

    log::info!(
        "Discord hooks: Processing message from {} ({}), admin={}, text='{}'",
        user_name,
        user_id,
        is_admin,
        if command_text.len() > 50 {
            format!("{}...", &command_text[..50])
        } else {
            command_text.clone()
        }
    );

    if is_admin {
        // Admin flow: implement query mode state machine
        let is_listening = is_listening_for_query(&user_id);

        if is_listening {
            // Admin was listening for a query - this message IS the query
            // Reset the listening state and forward to agent
            set_listening_for_query(&user_id, false);
            log::info!(
                "Discord hooks: Admin {} submitted query: '{}'",
                user_name,
                if command_text.len() > 50 {
                    format!("{}...", &command_text[..50])
                } else {
                    command_text.clone()
                }
            );
            Ok(ProcessResult::forward_to_agent(ForwardRequest {
                text: command_text,
                user_id,
                user_name,
                is_admin: true,
            }))
        } else if contains_query_keyword(&command_text) {
            // Admin said "query" - activate listening mode
            set_listening_for_query(&user_id, true);
            Ok(ProcessResult::handled(
                "Okay, I am ready for your query. Send your next message with @starkbot and I'll process it.".to_string(),
            ))
        } else {
            // Admin mentioned bot without "query" keyword and wasn't in listening mode
            // Explain how to activate query mode
            Ok(ProcessResult::handled(
                "Hi! I'd be happy to help with a query. Just say the magic word **\"query\"** \
                (e.g., `@starkbot query`) and I'll listen for your next command.\n\n\
                Example:\n\
                1. `@starkbot query` → I'll respond that I'm ready\n\
                2. `@starkbot check my portfolio` → I'll process this as an agentic query".to_string(),
            ))
        }
    } else {
        // Regular user: try limited commands
        match commands::parse(&command_text) {
            Some(cmd) => {
                let response = commands::execute(cmd, &user_id, db).await?;
                Ok(ProcessResult::handled(response))
            }
            None => {
                // Not a recognized limited command
                Ok(ProcessResult::handled(commands::permission_denied_message()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_command_text() {
        let bot_id = UserId::new(123456789);

        // Normal mention
        assert_eq!(
            extract_command_text("<@123456789> help", bot_id),
            "help"
        );

        // Nickname mention
        assert_eq!(
            extract_command_text("<@!123456789> register 0x123", bot_id),
            "register 0x123"
        );

        // Multiple mentions
        assert_eq!(
            extract_command_text("<@123456789> <@123456789> test", bot_id),
            "test"
        );

        // No mention
        assert_eq!(
            extract_command_text("just some text", bot_id),
            "just some text"
        );
    }

    #[test]
    fn test_contains_query_keyword() {
        // Basic cases
        assert!(contains_query_keyword("query"));
        assert!(contains_query_keyword("QUERY"));
        assert!(contains_query_keyword("Query"));

        // In context
        assert!(contains_query_keyword("hey bot, query please"));
        assert!(contains_query_keyword("I have a query for you"));
        assert!(contains_query_keyword("query: what is the price"));

        // Should not match
        assert!(!contains_query_keyword("hello"));
        assert!(!contains_query_keyword("tip @user 100"));
        assert!(!contains_query_keyword("check status"));
    }

    #[test]
    fn test_listening_for_query_state() {
        let user_id = "test_user_123";

        // Initially not listening
        assert!(!is_listening_for_query(user_id));

        // Set to listening
        set_listening_for_query(user_id, true);
        assert!(is_listening_for_query(user_id));

        // Reset
        set_listening_for_query(user_id, false);
        assert!(!is_listening_for_query(user_id));
    }

    #[test]
    fn test_multiple_users_listening_state() {
        let user1 = "admin_1";
        let user2 = "admin_2";

        // Set user1 to listening
        set_listening_for_query(user1, true);

        // user2 should not be listening
        assert!(is_listening_for_query(user1));
        assert!(!is_listening_for_query(user2));

        // Set user2 to listening
        set_listening_for_query(user2, true);

        // Both should be listening
        assert!(is_listening_for_query(user1));
        assert!(is_listening_for_query(user2));

        // Reset user1
        set_listening_for_query(user1, false);

        // Only user2 should be listening
        assert!(!is_listening_for_query(user1));
        assert!(is_listening_for_query(user2));

        // Cleanup
        set_listening_for_query(user2, false);
    }
}
