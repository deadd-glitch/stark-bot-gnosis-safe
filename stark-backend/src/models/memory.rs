use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Type of memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    DailyLog,
    LongTerm,
    /// Session summary - saved when a session is reset
    SessionSummary,
    /// Compaction summary - condensed older conversation history
    Compaction,
    /// User preferences (e.g., "prefers TypeScript over JavaScript")
    Preference,
    /// Facts about the user (e.g., "works at Acme Corp")
    Fact,
    /// Named entities (people, projects, tools)
    Entity,
    /// Tasks, commitments, and todos
    Task,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::DailyLog => "daily_log",
            MemoryType::LongTerm => "long_term",
            MemoryType::SessionSummary => "session_summary",
            MemoryType::Compaction => "compaction",
            MemoryType::Preference => "preference",
            MemoryType::Fact => "fact",
            MemoryType::Entity => "entity",
            MemoryType::Task => "task",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "daily_log" => Some(MemoryType::DailyLog),
            "long_term" => Some(MemoryType::LongTerm),
            "session_summary" => Some(MemoryType::SessionSummary),
            "compaction" => Some(MemoryType::Compaction),
            "preference" => Some(MemoryType::Preference),
            "fact" => Some(MemoryType::Fact),
            "entity" => Some(MemoryType::Entity),
            "task" => Some(MemoryType::Task),
            _ => None,
        }
    }

    /// Returns all memory types that represent user-specific memories (for context building)
    pub fn user_memory_types() -> &'static [MemoryType] {
        &[
            MemoryType::LongTerm,
            MemoryType::Preference,
            MemoryType::Fact,
            MemoryType::Entity,
            MemoryType::Task,
        ]
    }
}

/// Memory - daily logs, long-term memories, preferences, facts, entities, tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: i64,
    pub memory_type: MemoryType,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub importance: i32,
    pub identity_id: Option<String>,
    pub session_id: Option<i64>,
    pub source_channel_type: Option<String>,
    pub source_message_id: Option<String>,
    pub log_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    // Phase 2: Enhanced memory fields
    /// Entity type for Entity memories (e.g., "person", "project", "tool")
    pub entity_type: Option<String>,
    /// Normalized entity name
    pub entity_name: Option<String>,
    /// Confidence score 0.0-1.0 (for inferred memories)
    pub confidence: Option<f32>,
    /// Source type: "explicit" (user stated) or "inferred" (AI derived)
    pub source_type: Option<String>,
    /// Last time this memory was referenced in context
    pub last_referenced_at: Option<DateTime<Utc>>,
    // Phase 4: Memory consolidation fields
    /// ID of memory that supersedes this one (after consolidation)
    pub superseded_by: Option<i64>,
    /// When this memory was superseded
    pub superseded_at: Option<DateTime<Utc>>,
    // Phase 7: Temporal reasoning fields
    /// When the memory becomes relevant
    pub valid_from: Option<DateTime<Utc>>,
    /// When the memory expires/becomes irrelevant
    pub valid_until: Option<DateTime<Utc>>,
    /// Temporal type: "permanent", "temporary", "scheduled"
    pub temporal_type: Option<String>,
}

/// Request to create a memory
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMemoryRequest {
    pub memory_type: MemoryType,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<String>,
    #[serde(default = "default_importance")]
    pub importance: i32,
    pub identity_id: Option<String>,
    pub session_id: Option<i64>,
    pub source_channel_type: Option<String>,
    pub source_message_id: Option<String>,
    pub log_date: Option<NaiveDate>,
    pub expires_at: Option<DateTime<Utc>>,
    // Phase 2: Enhanced memory fields
    pub entity_type: Option<String>,
    pub entity_name: Option<String>,
    pub confidence: Option<f32>,
    pub source_type: Option<String>,
    // Phase 7: Temporal reasoning
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub temporal_type: Option<String>,
}

fn default_importance() -> i32 {
    5
}

fn default_confidence() -> f32 {
    1.0
}

/// Request to search memories
#[derive(Debug, Clone, Deserialize)]
pub struct SearchMemoriesRequest {
    pub query: String,
    pub memory_type: Option<MemoryType>,
    pub identity_id: Option<String>,
    pub category: Option<String>,
    pub min_importance: Option<i32>,
    #[serde(default = "default_limit")]
    pub limit: i32,
}

fn default_limit() -> i32 {
    20
}

/// Memory response for API
#[derive(Debug, Clone, Serialize)]
pub struct MemoryResponse {
    pub id: i64,
    pub memory_type: MemoryType,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub importance: i32,
    pub identity_id: Option<String>,
    pub source_channel_type: Option<String>,
    pub log_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Phase 2: Enhanced fields
    pub entity_type: Option<String>,
    pub entity_name: Option<String>,
    pub confidence: Option<f32>,
    pub source_type: Option<String>,
    pub last_referenced_at: Option<DateTime<Utc>>,
    // Phase 4: Consolidation
    pub superseded_by: Option<i64>,
    // Phase 7: Temporal
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub temporal_type: Option<String>,
}

impl From<Memory> for MemoryResponse {
    fn from(memory: Memory) -> Self {
        MemoryResponse {
            id: memory.id,
            memory_type: memory.memory_type,
            content: memory.content,
            category: memory.category,
            tags: memory.tags,
            importance: memory.importance,
            identity_id: memory.identity_id,
            source_channel_type: memory.source_channel_type,
            log_date: memory.log_date,
            created_at: memory.created_at,
            updated_at: memory.updated_at,
            entity_type: memory.entity_type,
            entity_name: memory.entity_name,
            confidence: memory.confidence,
            source_type: memory.source_type,
            last_referenced_at: memory.last_referenced_at,
            superseded_by: memory.superseded_by,
            valid_from: memory.valid_from,
            valid_until: memory.valid_until,
            temporal_type: memory.temporal_type,
        }
    }
}

/// Request to update a memory
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMemoryRequest {
    pub content: Option<String>,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub importance: Option<i32>,
    pub entity_type: Option<String>,
    pub entity_name: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub temporal_type: Option<String>,
}

/// Request to merge multiple memories
#[derive(Debug, Clone, Deserialize)]
pub struct MergeMemoriesRequest {
    /// IDs of memories to merge
    pub memory_ids: Vec<i64>,
    /// The merged content (AI-generated)
    pub merged_content: String,
    /// Optional: keep the highest importance from the merged memories
    pub use_max_importance: Option<bool>,
}

/// Memory statistics response
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total_count: i64,
    pub by_type: std::collections::HashMap<String, i64>,
    pub by_identity: std::collections::HashMap<String, i64>,
    pub avg_importance: f64,
    pub oldest_memory_at: Option<DateTime<Utc>>,
    pub newest_memory_at: Option<DateTime<Utc>>,
    pub superseded_count: i64,
    pub temporal_active_count: i64,
}

/// Memory search result with relevance score
#[derive(Debug, Clone, Serialize)]
pub struct MemorySearchResult {
    pub memory: MemoryResponse,
    pub rank: f64,
}
