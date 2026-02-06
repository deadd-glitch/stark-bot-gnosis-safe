//! Shared utilities for channel implementations.

/// Split a message into chunks respecting a platform's character limit.
/// Splits on line boundaries; lines exceeding `max_len` are hard-split.
pub fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if current.len() + line.len() + 1 > max_len {
            if !current.is_empty() {
                chunks.push(current);
                current = String::new();
            }
            if line.len() > max_len {
                let mut remaining = line;
                while remaining.len() > max_len {
                    chunks.push(remaining[..max_len].to_string());
                    remaining = &remaining[max_len..];
                }
                if !remaining.is_empty() {
                    current = remaining.to_string();
                }
            } else {
                current = line.to_string();
            }
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// Check whether a broadcast event belongs to a specific channel + chat session.
///
/// Matches on `channel_id` and optionally `chat_id` inside `event.data`:
/// - Both present in event → both must match
/// - Only `channel_id` present (legacy event) → channel must match
/// - No `channel_id` → returns `false`
pub fn event_matches_session(
    data: &serde_json::Value,
    channel_id: i64,
    chat_id: &str,
) -> bool {
    let ev_channel_id = data.get("channel_id").and_then(|v| v.as_i64());
    let ev_chat_id = data.get("chat_id").and_then(|v| v.as_str());

    match (ev_channel_id, ev_chat_id) {
        (Some(ch_id), Some(ev_chat)) => ch_id == channel_id && ev_chat == chat_id,
        (Some(ch_id), None) => ch_id == channel_id,
        _ => false,
    }
}
