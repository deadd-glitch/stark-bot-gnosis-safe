//! Memory consolidation module (Phase 4)
//!
//! Provides functionality to:
//! - Find clusters of similar memories
//! - Merge related memories into consolidated entries
//! - Deduplicate near-identical memories

use crate::ai::{AiClient, Message, MessageRole};
use crate::db::Database;
use crate::models::{Memory, MemoryType};
use super::embeddings::{EmbeddingConfig, EmbeddingProvider, create_provider};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Memory consolidator for managing memory clusters
pub struct MemoryConsolidator {
    db: Arc<Database>,
    embedding_provider: Box<dyn EmbeddingProvider>,
    config: EmbeddingConfig,
    /// Similarity threshold for clustering (0.0 - 1.0)
    similarity_threshold: f64,
    /// Minimum cluster size for consolidation
    min_cluster_size: usize,
}

impl MemoryConsolidator {
    pub fn new(db: Arc<Database>, config: EmbeddingConfig) -> Self {
        let embedding_provider = create_provider(&config);
        Self {
            db,
            embedding_provider,
            config,
            similarity_threshold: 0.85,
            min_cluster_size: 2,
        }
    }

    pub fn with_similarity_threshold(mut self, threshold: f64) -> Self {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_min_cluster_size(mut self, size: usize) -> Self {
        self.min_cluster_size = size.max(2);
        self
    }

    /// Find clusters of similar memories for a given identity
    pub async fn find_similar_clusters(
        &self,
        identity_id: &str,
        memory_type: Option<MemoryType>,
        limit: i32,
    ) -> Result<Vec<MemoryCluster>, String> {
        // Get memories with embeddings
        let memories = self.get_memories_with_embeddings(identity_id, memory_type, limit).await?;

        if memories.len() < self.min_cluster_size {
            return Ok(vec![]);
        }

        // Simple clustering using cosine similarity
        let mut clusters: Vec<MemoryCluster> = Vec::new();
        let mut assigned: HashSet<i64> = HashSet::new();

        for (i, (mem_a, emb_a)) in memories.iter().enumerate() {
            if assigned.contains(&mem_a.id) {
                continue;
            }

            let mut cluster = MemoryCluster {
                memories: vec![mem_a.clone()],
                embeddings: vec![emb_a.clone()],
                centroid: emb_a.clone(),
            };
            assigned.insert(mem_a.id);

            // Find similar memories
            for (mem_b, emb_b) in memories.iter().skip(i + 1) {
                if assigned.contains(&mem_b.id) {
                    continue;
                }

                let similarity = cosine_similarity(emb_a, emb_b);
                if similarity >= self.similarity_threshold {
                    cluster.memories.push(mem_b.clone());
                    cluster.embeddings.push(emb_b.clone());
                    assigned.insert(mem_b.id);
                }
            }

            // Only keep clusters above minimum size
            if cluster.memories.len() >= self.min_cluster_size {
                // Calculate centroid
                cluster.centroid = calculate_centroid(&cluster.embeddings);
                clusters.push(cluster);
            }
        }

        Ok(clusters)
    }

    /// Merge a cluster of memories into a single consolidated memory
    pub async fn merge_memories(
        &self,
        cluster: &MemoryCluster,
        client: &AiClient,
        identity_id: &str,
    ) -> Result<Memory, String> {
        if cluster.memories.is_empty() {
            return Err("Cannot merge empty cluster".to_string());
        }

        if cluster.memories.len() == 1 {
            return Ok(cluster.memories[0].clone());
        }

        // Build prompt for AI to merge
        let mut memory_text = String::new();
        for (i, mem) in cluster.memories.iter().enumerate() {
            memory_text.push_str(&format!(
                "Memory {}: [{}] {}\n",
                i + 1,
                mem.memory_type.as_str(),
                mem.content
            ));
        }

        let merge_prompt = format!(
            "Consolidate these related memories into a single, comprehensive memory. \
            Preserve all important information but remove redundancy. \
            Keep the same type/format as the original memories.\n\n\
            {}\n\n\
            Consolidated memory:",
            memory_text
        );

        let messages = vec![
            Message {
                role: MessageRole::System,
                content: "You consolidate related memories into single comprehensive entries. Be concise but preserve all important facts.".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: merge_prompt,
            },
        ];

        let merged_content = client.generate_text(messages).await
            .map_err(|e| format!("Failed to generate merged content: {}", e))?;

        // Use the highest importance from the cluster
        let max_importance = cluster.memories.iter()
            .map(|m| m.importance)
            .max()
            .unwrap_or(5);

        // Use the most common memory type
        let memory_type = cluster.memories[0].memory_type;

        // Create the consolidated memory
        let consolidated = self.db.create_memory_extended(
            memory_type,
            &merged_content.trim(),
            Some("consolidated"),
            None,
            max_importance,
            Some(identity_id),
            None, // session_id
            None, // source_channel_type
            None, // source_message_id
            None, // log_date
            None, // expires_at
            cluster.memories[0].entity_type.as_deref(),
            cluster.memories[0].entity_name.as_deref(),
            Some(1.0),
            Some("consolidated"),
            None, None, None,
        ).map_err(|e| format!("Failed to create consolidated memory: {}", e))?;

        // Mark original memories as superseded
        for mem in &cluster.memories {
            if let Err(e) = self.db.supersede_memory(mem.id, consolidated.id) {
                log::warn!("Failed to supersede memory {}: {}", mem.id, e);
            }
        }

        log::info!(
            "[CONSOLIDATION] Merged {} memories into memory {} for identity {}",
            cluster.memories.len(),
            consolidated.id,
            identity_id
        );

        Ok(consolidated)
    }

    /// Find and remove near-duplicate memories
    pub async fn deduplicate(
        &self,
        identity_id: &str,
        dry_run: bool,
    ) -> Result<DeduplicationResult, String> {
        let memories = self.get_memories_with_embeddings(identity_id, None, 500).await?;

        let mut duplicates: Vec<(i64, i64, f64)> = Vec::new(); // (keep_id, remove_id, similarity)
        let mut to_remove: HashSet<i64> = HashSet::new();

        for (i, (mem_a, emb_a)) in memories.iter().enumerate() {
            if to_remove.contains(&mem_a.id) {
                continue;
            }

            for (mem_b, emb_b) in memories.iter().skip(i + 1) {
                if to_remove.contains(&mem_b.id) {
                    continue;
                }

                let similarity = cosine_similarity(emb_a, emb_b);

                // Very high similarity threshold for deduplication (near-identical)
                if similarity >= 0.95 {
                    // Keep the one with higher importance, or the older one
                    let (keep, remove) = if mem_a.importance > mem_b.importance {
                        (&mem_a, &mem_b)
                    } else if mem_b.importance > mem_a.importance {
                        (&mem_b, &mem_a)
                    } else if mem_a.created_at <= mem_b.created_at {
                        (&mem_a, &mem_b)
                    } else {
                        (&mem_b, &mem_a)
                    };

                    duplicates.push((keep.id, remove.id, similarity));
                    to_remove.insert(remove.id);
                }
            }
        }

        if !dry_run {
            for (keep_id, remove_id, _) in &duplicates {
                if let Err(e) = self.db.supersede_memory(*remove_id, *keep_id) {
                    log::warn!("Failed to supersede duplicate memory {}: {}", remove_id, e);
                }
            }
        }

        Ok(DeduplicationResult {
            duplicates_found: duplicates.len(),
            duplicates_removed: if dry_run { 0 } else { duplicates.len() },
            pairs: duplicates,
        })
    }

    /// Get memories with their embeddings
    async fn get_memories_with_embeddings(
        &self,
        identity_id: &str,
        memory_type: Option<MemoryType>,
        limit: i32,
    ) -> Result<Vec<(Memory, Vec<f32>)>, String> {
        let conn = self.db.conn.lock().unwrap();

        let type_filter = memory_type
            .map(|t| format!("AND m.memory_type = '{}'", t.as_str()))
            .unwrap_or_default();

        let sql = format!(
            "SELECT m.id, m.memory_type, m.content, m.category, m.tags, m.importance, m.identity_id,
             m.session_id, m.source_channel_type, m.source_message_id, m.log_date,
             m.created_at, m.updated_at, m.expires_at,
             m.entity_type, m.entity_name, m.confidence, m.source_type, m.last_referenced_at,
             m.superseded_by, m.superseded_at, m.valid_from, m.valid_until, m.temporal_type,
             e.embedding
             FROM memories m
             JOIN memory_embeddings e ON m.id = e.memory_id
             WHERE m.identity_id = ?1 AND m.superseded_by IS NULL {}
             ORDER BY m.created_at DESC LIMIT ?2",
            type_filter
        );

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let results: Vec<(Memory, Vec<f32>)> = stmt.query_map(
            rusqlite::params![identity_id, limit],
            |row| {
                let memory = Database::row_to_memory_internal(row)?;
                let embedding_blob: Vec<u8> = row.get(24)?;
                let embedding: Vec<f32> = embedding_blob
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok((memory, embedding))
            }
        ).map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(results)
    }
}

/// A cluster of related memories
#[derive(Debug, Clone)]
pub struct MemoryCluster {
    pub memories: Vec<Memory>,
    pub embeddings: Vec<Vec<f32>>,
    pub centroid: Vec<f32>,
}

impl MemoryCluster {
    pub fn size(&self) -> usize {
        self.memories.len()
    }

    pub fn memory_ids(&self) -> Vec<i64> {
        self.memories.iter().map(|m| m.id).collect()
    }
}

/// Result of deduplication operation
#[derive(Debug)]
pub struct DeduplicationResult {
    pub duplicates_found: usize,
    pub duplicates_removed: usize,
    /// (keep_id, remove_id, similarity)
    pub pairs: Vec<(i64, i64, f64)>,
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f64 = a.iter().zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();

    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Calculate centroid of embeddings
fn calculate_centroid(embeddings: &[Vec<f32>]) -> Vec<f32> {
    if embeddings.is_empty() {
        return vec![];
    }

    let dim = embeddings[0].len();
    let mut centroid = vec![0.0f32; dim];

    for emb in embeddings {
        for (i, val) in emb.iter().enumerate() {
            centroid[i] += val;
        }
    }

    let n = embeddings.len() as f32;
    for val in &mut centroid {
        *val /= n;
    }

    centroid
}

// Helper method for Database - needs to be added
impl Database {
    /// Internal helper to parse memory from row (24 columns)
    pub fn row_to_memory_internal(row: &rusqlite::Row) -> rusqlite::Result<Memory> {
        use chrono::{DateTime, NaiveDate, Utc};
        use crate::models::MemoryType;

        let created_at_str: String = row.get(11)?;
        let updated_at_str: String = row.get(12)?;
        let expires_at_str: Option<String> = row.get(13)?;
        let log_date_str: Option<String> = row.get(10)?;
        let memory_type_str: String = row.get(1)?;
        let last_referenced_str: Option<String> = row.get(18)?;
        let superseded_at_str: Option<String> = row.get(20)?;
        let valid_from_str: Option<String> = row.get(21)?;
        let valid_until_str: Option<String> = row.get(22)?;

        Ok(Memory {
            id: row.get(0)?,
            memory_type: MemoryType::from_str(&memory_type_str).unwrap_or(MemoryType::DailyLog),
            content: row.get(2)?,
            category: row.get(3)?,
            tags: row.get(4)?,
            importance: row.get(5)?,
            identity_id: row.get(6)?,
            session_id: row.get(7)?,
            source_channel_type: row.get(8)?,
            source_message_id: row.get(9)?,
            log_date: log_date_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .unwrap()
                .with_timezone(&Utc),
            expires_at: expires_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
            }),
            entity_type: row.get(14)?,
            entity_name: row.get(15)?,
            confidence: row.get(16)?,
            source_type: row.get(17)?,
            last_referenced_at: last_referenced_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
            }),
            superseded_by: row.get(19)?,
            superseded_at: superseded_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
            }),
            valid_from: valid_from_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
            }),
            valid_until: valid_until_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
            }),
            temporal_type: row.get(23)?,
        })
    }
}
