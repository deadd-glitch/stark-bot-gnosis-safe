//! Memory database operations (daily logs, long-term memories, preferences, facts, entities, tasks)

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Result as SqliteResult;

use crate::models::{Memory, MemorySearchResult, MemoryStats, MemoryType, UpdateMemoryRequest};
use super::super::Database;
use std::collections::HashMap;

impl Database {
    /// Create a memory (daily_log, long_term, session_summary, compaction, preference, fact, entity, task)
    #[allow(clippy::too_many_arguments)]
    pub fn create_memory(
        &self,
        memory_type: MemoryType,
        content: &str,
        category: Option<&str>,
        tags: Option<&str>,
        importance: i32,
        identity_id: Option<&str>,
        session_id: Option<i64>,
        source_channel_type: Option<&str>,
        source_message_id: Option<&str>,
        log_date: Option<NaiveDate>,
        expires_at: Option<DateTime<Utc>>,
    ) -> SqliteResult<Memory> {
        self.create_memory_extended(
            memory_type,
            content,
            category,
            tags,
            importance,
            identity_id,
            session_id,
            source_channel_type,
            source_message_id,
            log_date,
            expires_at,
            None, None, None, None, None, None, None,
        )
    }

    /// Create a memory with all extended fields (Phase 2, 7)
    #[allow(clippy::too_many_arguments)]
    pub fn create_memory_extended(
        &self,
        memory_type: MemoryType,
        content: &str,
        category: Option<&str>,
        tags: Option<&str>,
        importance: i32,
        identity_id: Option<&str>,
        session_id: Option<i64>,
        source_channel_type: Option<&str>,
        source_message_id: Option<&str>,
        log_date: Option<NaiveDate>,
        expires_at: Option<DateTime<Utc>>,
        entity_type: Option<&str>,
        entity_name: Option<&str>,
        confidence: Option<f32>,
        source_type: Option<&str>,
        valid_from: Option<DateTime<Utc>>,
        valid_until: Option<DateTime<Utc>>,
        temporal_type: Option<&str>,
    ) -> SqliteResult<Memory> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let log_date_str = log_date.map(|d| d.to_string());
        let expires_at_str = expires_at.map(|dt| dt.to_rfc3339());
        let valid_from_str = valid_from.map(|dt| dt.to_rfc3339());
        let valid_until_str = valid_until.map(|dt| dt.to_rfc3339());
        let conf = confidence.unwrap_or(1.0);
        let src_type = source_type.unwrap_or("inferred");

        conn.execute(
            "INSERT INTO memories (memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, valid_from, valid_until, temporal_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            rusqlite::params![
                memory_type.as_str(),
                content,
                category,
                tags,
                importance,
                identity_id,
                session_id,
                source_channel_type,
                source_message_id,
                log_date_str,
                &now_str,
                expires_at_str,
                entity_type,
                entity_name,
                conf,
                src_type,
                valid_from_str,
                valid_until_str,
                temporal_type,
            ],
        )?;

        let id = conn.last_insert_rowid();

        Ok(Memory {
            id,
            memory_type,
            content: content.to_string(),
            category: category.map(|s| s.to_string()),
            tags: tags.map(|s| s.to_string()),
            importance,
            identity_id: identity_id.map(|s| s.to_string()),
            session_id,
            source_channel_type: source_channel_type.map(|s| s.to_string()),
            source_message_id: source_message_id.map(|s| s.to_string()),
            log_date,
            created_at: now,
            updated_at: now,
            expires_at,
            entity_type: entity_type.map(|s| s.to_string()),
            entity_name: entity_name.map(|s| s.to_string()),
            confidence: Some(conf),
            source_type: Some(src_type.to_string()),
            last_referenced_at: None,
            superseded_by: None,
            superseded_at: None,
            valid_from,
            valid_until,
            temporal_type: temporal_type.map(|s| s.to_string()),
        })
    }

    /// Search memories using FTS5
    pub fn search_memories(
        &self,
        query: &str,
        memory_type: Option<MemoryType>,
        identity_id: Option<&str>,
        category: Option<&str>,
        min_importance: Option<i32>,
        limit: i32,
    ) -> SqliteResult<Vec<MemorySearchResult>> {
        let conn = self.conn.lock().unwrap();

        // Build the query with filters - includes all new columns
        let mut sql = String::from(
            "SELECT m.id, m.memory_type, m.content, m.category, m.tags, m.importance, m.identity_id,
             m.session_id, m.source_channel_type, m.source_message_id, m.log_date,
             m.created_at, m.updated_at, m.expires_at,
             m.entity_type, m.entity_name, m.confidence, m.source_type, m.last_referenced_at,
             m.superseded_by, m.superseded_at, m.valid_from, m.valid_until, m.temporal_type,
             bm25(memories_fts) as rank
             FROM memories m
             JOIN memories_fts ON m.id = memories_fts.rowid
             WHERE memories_fts MATCH ?1 AND m.superseded_by IS NULL",
        );

        let mut conditions: Vec<String> = Vec::new();
        if memory_type.is_some() {
            conditions.push("m.memory_type = ?2".to_string());
        }
        if identity_id.is_some() {
            conditions.push(format!("m.identity_id = ?{}", if memory_type.is_some() { 3 } else { 2 }));
        }
        if category.is_some() {
            let idx = 2 + (memory_type.is_some() as usize) + (identity_id.is_some() as usize);
            conditions.push(format!("m.category = ?{}", idx));
        }
        if min_importance.is_some() {
            let idx = 2 + (memory_type.is_some() as usize) + (identity_id.is_some() as usize) + (category.is_some() as usize);
            conditions.push(format!("m.importance >= ?{}", idx));
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY rank LIMIT ?");
        let limit_idx = 2 + (memory_type.is_some() as usize) + (identity_id.is_some() as usize)
            + (category.is_some() as usize) + (min_importance.is_some() as usize);
        sql = sql.replace("LIMIT ?", &format!("LIMIT ?{}", limit_idx));

        let mut stmt = conn.prepare(&sql)?;

        // Build params dynamically
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(query.to_string())];
        if let Some(mt) = memory_type {
            params.push(Box::new(mt.as_str().to_string()));
        }
        if let Some(iid) = identity_id {
            params.push(Box::new(iid.to_string()));
        }
        if let Some(cat) = category {
            params.push(Box::new(cat.to_string()));
        }
        if let Some(mi) = min_importance {
            params.push(Box::new(mi));
        }
        params.push(Box::new(limit));

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let results = stmt
            .query_map(params_ref.as_slice(), |row| {
                let memory = Self::row_to_memory(row)?;
                let rank: f64 = row.get(24)?; // rank is now at index 24
                Ok(MemorySearchResult {
                    memory: memory.into(),
                    rank,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Get today's daily logs
    pub fn get_todays_daily_logs(&self, identity_id: Option<&str>) -> SqliteResult<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();
        let today = Utc::now().date_naive().to_string();

        let sql = if identity_id.is_some() {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE memory_type = 'daily_log' AND log_date = ?1 AND identity_id = ?2
             AND superseded_by IS NULL ORDER BY created_at ASC"
        } else {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE memory_type = 'daily_log' AND log_date = ?1
             AND superseded_by IS NULL ORDER BY created_at ASC"
        };

        let mut stmt = conn.prepare(sql)?;

        let memories: Vec<Memory> = if let Some(iid) = identity_id {
            stmt.query_map(rusqlite::params![&today, iid], |row| Self::row_to_memory(row))?
                .filter_map(|r| r.ok())
                .collect()
        } else {
            stmt.query_map([&today], |row| Self::row_to_memory(row))?
                .filter_map(|r| r.ok())
                .collect()
        };

        Ok(memories)
    }

    /// Get long-term memories for an identity (includes preference, fact, entity, task types)
    pub fn get_long_term_memories(&self, identity_id: Option<&str>, min_importance: Option<i32>, limit: i32) -> SqliteResult<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();
        let min_imp = min_importance.unwrap_or(0);
        let now = Utc::now().to_rfc3339();

        // Include all user memory types: long_term, preference, fact, entity, task
        let sql = if identity_id.is_some() {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE memory_type IN ('long_term', 'preference', 'fact', 'entity', 'task')
             AND identity_id = ?1 AND importance >= ?2
             AND superseded_by IS NULL
             AND (valid_from IS NULL OR valid_from <= ?3)
             AND (valid_until IS NULL OR valid_until >= ?3)
             ORDER BY importance DESC, created_at DESC LIMIT ?4"
        } else {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE memory_type IN ('long_term', 'preference', 'fact', 'entity', 'task')
             AND importance >= ?1
             AND superseded_by IS NULL
             AND (valid_from IS NULL OR valid_from <= ?2)
             AND (valid_until IS NULL OR valid_until >= ?2)
             ORDER BY importance DESC, created_at DESC LIMIT ?3"
        };

        let mut stmt = conn.prepare(sql)?;

        let memories: Vec<Memory> = if let Some(iid) = identity_id {
            stmt.query_map(rusqlite::params![iid, min_imp, &now, limit], |row| Self::row_to_memory(row))?
                .filter_map(|r| r.ok())
                .collect()
        } else {
            stmt.query_map(rusqlite::params![min_imp, &now, limit], |row| Self::row_to_memory(row))?
                .filter_map(|r| r.ok())
                .collect()
        };

        Ok(memories)
    }

    /// Get session summaries (past conversation summaries)
    pub fn get_session_summaries(&self, identity_id: Option<&str>, limit: i32) -> SqliteResult<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();

        let sql = if identity_id.is_some() {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE memory_type = 'session_summary' AND identity_id = ?1
             ORDER BY created_at DESC LIMIT ?2"
        } else {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE memory_type = 'session_summary'
             ORDER BY created_at DESC LIMIT ?1"
        };

        let mut stmt = conn.prepare(sql)?;

        let memories: Vec<Memory> = if let Some(iid) = identity_id {
            stmt.query_map(rusqlite::params![iid, limit], |row| Self::row_to_memory(row))?
                .filter_map(|r| r.ok())
                .collect()
        } else {
            stmt.query_map([limit], |row| Self::row_to_memory(row))?
                .filter_map(|r| r.ok())
                .collect()
        };

        Ok(memories)
    }

    /// List all memories (with pagination support)
    pub fn list_memories(&self) -> SqliteResult<Vec<Memory>> {
        self.list_memories_paginated(100, 0)
    }

    /// List memories with pagination
    pub fn list_memories_paginated(&self, limit: i32, offset: i32) -> SqliteResult<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )?;

        let memories = stmt
            .query_map(rusqlite::params![limit, offset], |row| Self::row_to_memory(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    /// List memories with filters (Phase 5: UI)
    pub fn list_memories_filtered(
        &self,
        memory_type: Option<MemoryType>,
        identity_id: Option<&str>,
        min_importance: Option<i32>,
        include_superseded: bool,
        limit: i32,
        offset: i32,
    ) -> SqliteResult<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();

        let mut conditions = Vec::new();
        if memory_type.is_some() { conditions.push("memory_type = ?1".to_string()); }
        if identity_id.is_some() {
            let idx = if memory_type.is_some() { 2 } else { 1 };
            conditions.push(format!("identity_id = ?{}", idx));
        }
        if min_importance.is_some() {
            let idx = 1 + memory_type.is_some() as usize + identity_id.is_some() as usize;
            conditions.push(format!("importance >= ?{}", idx));
        }
        if !include_superseded {
            conditions.push("superseded_by IS NULL".to_string());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit_idx = 1 + memory_type.is_some() as usize + identity_id.is_some() as usize + min_importance.is_some() as usize;
        let offset_idx = limit_idx + 1;

        let sql = format!(
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
            where_clause, limit_idx, offset_idx
        );

        let mut stmt = conn.prepare(&sql)?;

        // Build params
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(mt) = memory_type { params.push(Box::new(mt.as_str().to_string())); }
        if let Some(iid) = identity_id { params.push(Box::new(iid.to_string())); }
        if let Some(mi) = min_importance { params.push(Box::new(mi)); }
        params.push(Box::new(limit));
        params.push(Box::new(offset));

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let memories = stmt
            .query_map(params_ref.as_slice(), |row| Self::row_to_memory(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(memories)
    }

    /// Delete a memory
    pub fn delete_memory(&self, id: i64) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute("DELETE FROM memories WHERE id = ?1", [id])?;
        Ok(rows_affected > 0)
    }

    /// Cleanup expired memories
    pub fn cleanup_expired_memories(&self) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let rows_affected = conn.execute(
            "DELETE FROM memories WHERE expires_at IS NOT NULL AND expires_at < ?1",
            [&now],
        )?;
        Ok(rows_affected as i64)
    }

    /// Update a memory's fields
    pub fn update_memory(&self, id: i64, update: &UpdateMemoryRequest) -> SqliteResult<Option<Memory>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // Build dynamic update query
        let mut updates = vec!["updated_at = ?1".to_string()];
        let mut param_idx = 2;

        if update.content.is_some() { updates.push(format!("content = ?{}", param_idx)); param_idx += 1; }
        if update.category.is_some() { updates.push(format!("category = ?{}", param_idx)); param_idx += 1; }
        if update.tags.is_some() { updates.push(format!("tags = ?{}", param_idx)); param_idx += 1; }
        if update.importance.is_some() { updates.push(format!("importance = ?{}", param_idx)); param_idx += 1; }
        if update.entity_type.is_some() { updates.push(format!("entity_type = ?{}", param_idx)); param_idx += 1; }
        if update.entity_name.is_some() { updates.push(format!("entity_name = ?{}", param_idx)); param_idx += 1; }
        if update.valid_from.is_some() { updates.push(format!("valid_from = ?{}", param_idx)); param_idx += 1; }
        if update.valid_until.is_some() { updates.push(format!("valid_until = ?{}", param_idx)); param_idx += 1; }
        if update.temporal_type.is_some() { updates.push(format!("temporal_type = ?{}", param_idx)); param_idx += 1; }

        let sql = format!("UPDATE memories SET {} WHERE id = ?{}", updates.join(", "), param_idx);

        // Build params
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];
        if let Some(ref v) = update.content { params.push(Box::new(v.clone())); }
        if let Some(ref v) = update.category { params.push(Box::new(v.clone())); }
        if let Some(ref v) = update.tags { params.push(Box::new(v.clone())); }
        if let Some(v) = update.importance { params.push(Box::new(v)); }
        if let Some(ref v) = update.entity_type { params.push(Box::new(v.clone())); }
        if let Some(ref v) = update.entity_name { params.push(Box::new(v.clone())); }
        if let Some(v) = update.valid_from { params.push(Box::new(v.to_rfc3339())); }
        if let Some(v) = update.valid_until { params.push(Box::new(v.to_rfc3339())); }
        if let Some(ref v) = update.temporal_type { params.push(Box::new(v.clone())); }
        params.push(Box::new(id));

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_ref.as_slice())?;

        drop(conn);
        self.get_memory(id)
    }

    /// Get a single memory by ID
    pub fn get_memory(&self, id: i64) -> SqliteResult<Option<Memory>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE id = ?1",
        )?;

        let memory = stmt.query_row([id], |row| Self::row_to_memory(row)).ok();
        Ok(memory)
    }

    /// Mark a memory as superseded by another (Phase 4: consolidation)
    pub fn supersede_memory(&self, memory_id: i64, superseded_by: i64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE memories SET superseded_by = ?1, superseded_at = ?2, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![superseded_by, &now, memory_id],
        )?;
        Ok(())
    }

    /// Update last_referenced_at for a memory
    pub fn touch_memory(&self, id: i64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE memories SET last_referenced_at = ?1 WHERE id = ?2",
            rusqlite::params![&now, id],
        )?;
        Ok(())
    }

    /// Get memories by entity (Phase 2)
    pub fn get_memories_by_entity(
        &self,
        entity_type: &str,
        entity_name: Option<&str>,
        identity_id: Option<&str>,
        limit: i32,
    ) -> SqliteResult<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();

        let sql = if entity_name.is_some() && identity_id.is_some() {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE entity_type = ?1 AND entity_name = ?2 AND identity_id = ?3
             AND superseded_by IS NULL ORDER BY importance DESC, created_at DESC LIMIT ?4"
        } else if entity_name.is_some() {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE entity_type = ?1 AND entity_name = ?2
             AND superseded_by IS NULL ORDER BY importance DESC, created_at DESC LIMIT ?3"
        } else if identity_id.is_some() {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE entity_type = ?1 AND identity_id = ?2
             AND superseded_by IS NULL ORDER BY importance DESC, created_at DESC LIMIT ?3"
        } else {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories WHERE entity_type = ?1
             AND superseded_by IS NULL ORDER BY importance DESC, created_at DESC LIMIT ?2"
        };

        let mut stmt = conn.prepare(sql)?;
        let memories: Vec<Memory> = if let Some(name) = entity_name {
            if let Some(iid) = identity_id {
                stmt.query_map(rusqlite::params![entity_type, name, iid, limit], Self::row_to_memory)?
                    .filter_map(|r| r.ok()).collect()
            } else {
                stmt.query_map(rusqlite::params![entity_type, name, limit], Self::row_to_memory)?
                    .filter_map(|r| r.ok()).collect()
            }
        } else if let Some(iid) = identity_id {
            stmt.query_map(rusqlite::params![entity_type, iid, limit], Self::row_to_memory)?
                .filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map(rusqlite::params![entity_type, limit], Self::row_to_memory)?
                .filter_map(|r| r.ok()).collect()
        };

        Ok(memories)
    }

    /// Get temporally valid memories (Phase 7)
    pub fn get_valid_memories(
        &self,
        identity_id: Option<&str>,
        memory_types: Option<&[MemoryType]>,
        limit: i32,
    ) -> SqliteResult<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let type_filter = memory_types.map(|types| {
            let type_strs: Vec<_> = types.iter().map(|t| format!("'{}'", t.as_str())).collect();
            format!("AND memory_type IN ({})", type_strs.join(", "))
        }).unwrap_or_default();

        let sql = format!(
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories
             WHERE superseded_by IS NULL
             AND (valid_from IS NULL OR valid_from <= ?1)
             AND (valid_until IS NULL OR valid_until >= ?1)
             {} {}
             ORDER BY importance DESC, created_at DESC LIMIT ?2",
            if identity_id.is_some() { "AND identity_id = ?3" } else { "" },
            type_filter
        );

        let mut stmt = conn.prepare(&sql)?;
        let memories: Vec<Memory> = if let Some(iid) = identity_id {
            stmt.query_map(rusqlite::params![&now, limit, iid], Self::row_to_memory)?
                .filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map(rusqlite::params![&now, limit], Self::row_to_memory)?
                .filter_map(|r| r.ok()).collect()
        };

        Ok(memories)
    }

    /// Get cross-channel memories for an identity (Phase 6)
    pub fn get_cross_channel_memories(
        &self,
        identity_id: &str,
        exclude_channel_type: Option<&str>,
        limit: i32,
    ) -> SqliteResult<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let sql = if exclude_channel_type.is_some() {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories
             WHERE identity_id = ?1
             AND source_channel_type IS NOT NULL
             AND source_channel_type != ?2
             AND superseded_by IS NULL
             AND (valid_from IS NULL OR valid_from <= ?3)
             AND (valid_until IS NULL OR valid_until >= ?3)
             AND memory_type IN ('long_term', 'preference', 'fact', 'entity', 'task')
             ORDER BY importance DESC, created_at DESC LIMIT ?4"
        } else {
            "SELECT id, memory_type, content, category, tags, importance, identity_id, session_id,
             source_channel_type, source_message_id, log_date, created_at, updated_at, expires_at,
             entity_type, entity_name, confidence, source_type, last_referenced_at,
             superseded_by, superseded_at, valid_from, valid_until, temporal_type
             FROM memories
             WHERE identity_id = ?1
             AND superseded_by IS NULL
             AND (valid_from IS NULL OR valid_from <= ?2)
             AND (valid_until IS NULL OR valid_until >= ?2)
             AND memory_type IN ('long_term', 'preference', 'fact', 'entity', 'task')
             ORDER BY importance DESC, created_at DESC LIMIT ?3"
        };

        let mut stmt = conn.prepare(sql)?;
        let memories: Vec<Memory> = if let Some(exc_channel) = exclude_channel_type {
            stmt.query_map(rusqlite::params![identity_id, exc_channel, &now, limit], Self::row_to_memory)?
                .filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map(rusqlite::params![identity_id, &now, limit], Self::row_to_memory)?
                .filter_map(|r| r.ok()).collect()
        };

        Ok(memories)
    }

    /// Get memory statistics (Phase 5: UI)
    pub fn get_memory_stats(&self) -> SqliteResult<MemoryStats> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let total_count: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;

        // Count by type
        let mut by_type = HashMap::new();
        let mut stmt = conn.prepare("SELECT memory_type, COUNT(*) FROM memories GROUP BY memory_type")?;
        let rows = stmt.query_map([], |row| {
            let type_str: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((type_str, count))
        })?;
        for row in rows.flatten() {
            by_type.insert(row.0, row.1);
        }

        // Count by identity
        let mut by_identity = HashMap::new();
        let mut stmt = conn.prepare("SELECT COALESCE(identity_id, 'anonymous'), COUNT(*) FROM memories GROUP BY identity_id")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((id, count))
        })?;
        for row in rows.flatten() {
            by_identity.insert(row.0, row.1);
        }

        let avg_importance: f64 = conn.query_row(
            "SELECT COALESCE(AVG(importance), 0) FROM memories",
            [],
            |row| row.get(0),
        )?;

        let oldest: Option<String> = conn.query_row(
            "SELECT MIN(created_at) FROM memories",
            [],
            |row| row.get(0),
        ).ok().flatten();

        let newest: Option<String> = conn.query_row(
            "SELECT MAX(created_at) FROM memories",
            [],
            |row| row.get(0),
        ).ok().flatten();

        let superseded_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE superseded_by IS NOT NULL",
            [],
            |row| row.get(0),
        )?;

        let temporal_active_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE (valid_from IS NULL OR valid_from <= ?1) AND (valid_until IS NULL OR valid_until >= ?1)",
            [&now],
            |row| row.get(0),
        )?;

        Ok(MemoryStats {
            total_count,
            by_type,
            by_identity,
            avg_importance,
            oldest_memory_at: oldest.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
            newest_memory_at: newest.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
            superseded_count,
            temporal_active_count,
        })
    }

    /// Export memories as markdown (Phase 5: UI)
    pub fn export_memories_markdown(&self, identity_id: Option<&str>) -> SqliteResult<String> {
        let memories = if let Some(iid) = identity_id {
            self.get_long_term_memories(Some(iid), Some(0), 1000)?
        } else {
            self.list_memories()?
        };

        let mut md = String::from("# Exported Memories\n\n");
        for memory in memories {
            md.push_str(&format!("## {} (ID: {})\n", memory.memory_type.as_str(), memory.id));
            md.push_str(&format!("**Importance:** {} | **Created:** {}\n\n", memory.importance, memory.created_at));
            md.push_str(&format!("{}\n\n", memory.content));
            if let Some(tags) = &memory.tags {
                md.push_str(&format!("*Tags: {}*\n\n", tags));
            }
            md.push_str("---\n\n");
        }

        Ok(md)
    }

    fn row_to_memory(row: &rusqlite::Row) -> rusqlite::Result<Memory> {
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
