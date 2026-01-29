//! Memory module for enhanced memory management
//!
//! This module provides:
//! - Vector embeddings for semantic search (Phase 3)
//! - Hybrid search combining BM25 + vector similarity
//! - Memory consolidation for deduplication (Phase 4)

pub mod embeddings;
pub mod search;
pub mod consolidation;

pub use embeddings::{EmbeddingProvider, EmbeddingConfig};
pub use search::{HybridSearcher, SearchResult};
pub use consolidation::MemoryConsolidator;
