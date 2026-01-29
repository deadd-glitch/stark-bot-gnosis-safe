//! Hybrid search combining BM25 (full-text) and vector similarity
//!
//! Uses Reciprocal Rank Fusion (RRF) to merge results from both search methods.

use crate::db::Database;
use crate::models::{Memory, MemorySearchResult, MemoryType};
use super::embeddings::{EmbeddingConfig, EmbeddingProvider, create_provider};
use std::collections::HashMap;
use std::sync::Arc;

/// Result from hybrid search with combined score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub memory: Memory,
    /// Combined RRF score (higher is better)
    pub score: f64,
    /// BM25 rank (if available)
    pub bm25_rank: Option<i32>,
    /// Vector similarity rank (if available)
    pub vector_rank: Option<i32>,
}

/// Hybrid searcher combining BM25 and vector search
pub struct HybridSearcher {
    db: Arc<Database>,
    embedding_provider: Box<dyn EmbeddingProvider>,
    config: EmbeddingConfig,
    /// RRF constant (typically 60)
    rrf_k: f64,
}

impl HybridSearcher {
    pub fn new(db: Arc<Database>, config: EmbeddingConfig) -> Self {
        let embedding_provider = create_provider(&config);
        Self {
            db,
            embedding_provider,
            config,
            rrf_k: 60.0,
        }
    }

    /// Check if vector search is enabled
    pub fn vector_search_enabled(&self) -> bool {
        self.config.is_enabled()
    }

    /// Perform hybrid search combining BM25 and vector similarity
    pub async fn search(
        &self,
        query: &str,
        memory_type: Option<MemoryType>,
        identity_id: Option<&str>,
        limit: i32,
    ) -> Result<Vec<SearchResult>, String> {
        // Always run BM25 search
        let bm25_results = self.db.search_memories(
            query,
            memory_type,
            identity_id,
            None, // category
            None, // min_importance
            limit * 2, // Get more results for merging
        ).map_err(|e| format!("BM25 search failed: {}", e))?;

        // If vector search is disabled, just return BM25 results
        if !self.vector_search_enabled() {
            return Ok(bm25_results.into_iter().take(limit as usize).map(|r| {
                SearchResult {
                    memory: self.response_to_memory(&r),
                    score: -r.rank, // BM25 returns negative scores (lower is better)
                    bm25_rank: Some(1), // Will be renumbered
                    vector_rank: None,
                }
            }).collect());
        }

        // Generate query embedding
        let query_embedding = match self.embedding_provider.embed(query).await {
            Ok(emb) => emb,
            Err(e) => {
                log::warn!("Failed to generate query embedding: {}. Falling back to BM25 only.", e);
                return Ok(bm25_results.into_iter().take(limit as usize).map(|r| {
                    SearchResult {
                        memory: self.response_to_memory(&r),
                        score: -r.rank,
                        bm25_rank: Some(1),
                        vector_rank: None,
                    }
                }).collect());
            }
        };

        // Get vector search results
        let vector_results = self.vector_search(
            &query_embedding.vector,
            memory_type,
            identity_id,
            limit * 2,
        ).await?;

        // Merge results using RRF
        let merged = self.reciprocal_rank_fusion(bm25_results, vector_results, limit);

        Ok(merged)
    }

    /// Perform vector similarity search
    async fn vector_search(
        &self,
        query_vector: &[f32],
        memory_type: Option<MemoryType>,
        identity_id: Option<&str>,
        limit: i32,
    ) -> Result<Vec<(i64, f64)>, String> {
        // For now, we do a simple linear scan of embeddings
        // In production, this should use sqlite-vec or a dedicated vector DB

        let conn = self.db.conn.lock().unwrap();

        let type_filter = memory_type.map(|t| format!("AND m.memory_type = '{}'", t.as_str())).unwrap_or_default();
        let identity_filter = identity_id.map(|id| format!("AND m.identity_id = '{}'", id)).unwrap_or_default();

        let sql = format!(
            "SELECT e.memory_id, e.embedding FROM memory_embeddings e
             JOIN memories m ON e.memory_id = m.id
             WHERE m.superseded_by IS NULL {} {}
             LIMIT 1000", // Cap for performance
            type_filter, identity_filter
        );

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("Failed to prepare vector search: {}", e))?;

        let rows = stmt.query_map([], |row| {
            let memory_id: i64 = row.get(0)?;
            let embedding_blob: Vec<u8> = row.get(1)?;
            Ok((memory_id, embedding_blob))
        }).map_err(|e| format!("Failed to execute vector search: {}", e))?;

        let mut similarities: Vec<(i64, f64)> = Vec::new();

        for row in rows.flatten() {
            let (memory_id, embedding_blob) = row;

            // Deserialize embedding from blob (f32 array stored as bytes)
            let stored_vector: Vec<f32> = embedding_blob
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();

            // Calculate cosine similarity
            let similarity = cosine_similarity(query_vector, &stored_vector);
            similarities.push((memory_id, similarity));
        }

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results
        Ok(similarities.into_iter().take(limit as usize).collect())
    }

    /// Merge BM25 and vector results using Reciprocal Rank Fusion
    fn reciprocal_rank_fusion(
        &self,
        bm25_results: Vec<MemorySearchResult>,
        vector_results: Vec<(i64, f64)>,
        limit: i32,
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<i64, (f64, Option<i32>, Option<i32>, Option<Memory>)> = HashMap::new();

        // Add BM25 scores
        for (rank, result) in bm25_results.iter().enumerate() {
            let rrf_score = 1.0 / (self.rrf_k + (rank + 1) as f64);
            let entry = scores.entry(result.memory.id).or_insert((0.0, None, None, None));
            entry.0 += rrf_score;
            entry.1 = Some((rank + 1) as i32);
            entry.3 = Some(self.response_to_memory(result));
        }

        // Add vector scores
        for (rank, (memory_id, _similarity)) in vector_results.iter().enumerate() {
            let rrf_score = 1.0 / (self.rrf_k + (rank + 1) as f64);
            let entry = scores.entry(*memory_id).or_insert((0.0, None, None, None));
            entry.0 += rrf_score;
            entry.2 = Some((rank + 1) as i32);
        }

        // Convert to SearchResult and sort by combined score
        let mut results: Vec<SearchResult> = scores.into_iter()
            .filter_map(|(id, (score, bm25_rank, vector_rank, memory))| {
                // If we don't have the memory from BM25, fetch it
                let memory = memory.or_else(|| {
                    self.db.get_memory(id).ok().flatten()
                })?;

                Some(SearchResult {
                    memory,
                    score,
                    bm25_rank,
                    vector_rank,
                })
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit as usize);

        results
    }

    /// Convert MemoryResponse back to Memory (helper)
    fn response_to_memory(&self, result: &MemorySearchResult) -> Memory {
        Memory {
            id: result.memory.id,
            memory_type: result.memory.memory_type,
            content: result.memory.content.clone(),
            category: result.memory.category.clone(),
            tags: result.memory.tags.clone(),
            importance: result.memory.importance,
            identity_id: result.memory.identity_id.clone(),
            session_id: None,
            source_channel_type: result.memory.source_channel_type.clone(),
            source_message_id: None,
            log_date: result.memory.log_date,
            created_at: result.memory.created_at,
            updated_at: result.memory.updated_at,
            expires_at: None,
            entity_type: result.memory.entity_type.clone(),
            entity_name: result.memory.entity_name.clone(),
            confidence: result.memory.confidence,
            source_type: result.memory.source_type.clone(),
            last_referenced_at: result.memory.last_referenced_at,
            superseded_by: result.memory.superseded_by,
            superseded_at: None,
            valid_from: result.memory.valid_from,
            valid_until: result.memory.valid_until,
            temporal_type: result.memory.temporal_type.clone(),
        }
    }

    /// Generate and store embedding for a memory
    pub async fn embed_memory(&self, memory_id: i64, content: &str) -> Result<(), String> {
        if !self.vector_search_enabled() {
            return Ok(());
        }

        let embedding = self.embedding_provider.embed(content).await?;

        // Store embedding in database
        let conn = self.db.conn.lock().unwrap();

        // Serialize embedding to bytes
        let embedding_bytes: Vec<u8> = embedding.vector.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        conn.execute(
            "INSERT OR REPLACE INTO memory_embeddings (memory_id, embedding, model, dimensions, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            rusqlite::params![
                memory_id,
                embedding_bytes,
                embedding.model,
                embedding.dimensions as i32,
            ],
        ).map_err(|e| format!("Failed to store embedding: {}", e))?;

        Ok(())
    }

    /// Generate embeddings for all memories that don't have one
    pub async fn backfill_embeddings(&self, batch_size: usize) -> Result<usize, String> {
        if !self.vector_search_enabled() {
            return Ok(0);
        }

        let conn = self.db.conn.lock().unwrap();

        // Find memories without embeddings
        let mut stmt = conn.prepare(
            "SELECT m.id, m.content FROM memories m
             LEFT JOIN memory_embeddings e ON m.id = e.memory_id
             WHERE e.memory_id IS NULL AND m.superseded_by IS NULL
             LIMIT ?"
        ).map_err(|e| format!("Failed to find memories: {}", e))?;

        let memories: Vec<(i64, String)> = stmt
            .query_map([batch_size as i32], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e| format!("Query failed: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        drop(stmt);
        drop(conn);

        if memories.is_empty() {
            return Ok(0);
        }

        // Generate embeddings
        let texts: Vec<&str> = memories.iter().map(|(_, c)| c.as_str()).collect();
        let embeddings = self.embedding_provider.embed_batch(&texts).await?;

        // Store embeddings
        let conn = self.db.conn.lock().unwrap();
        for ((memory_id, _), embedding) in memories.iter().zip(embeddings.iter()) {
            let embedding_bytes: Vec<u8> = embedding.vector.iter()
                .flat_map(|f| f.to_le_bytes())
                .collect();

            let _ = conn.execute(
                "INSERT OR REPLACE INTO memory_embeddings (memory_id, embedding, model, dimensions, created_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                rusqlite::params![
                    memory_id,
                    embedding_bytes,
                    embedding.model,
                    embedding.dimensions as i32,
                ],
            );
        }

        Ok(memories.len())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.0001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.0001);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &d) - (-1.0)).abs() < 0.0001);
    }
}
