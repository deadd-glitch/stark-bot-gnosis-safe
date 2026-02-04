//! Mind map database operations (mind_nodes, mind_node_connections)

use chrono::{DateTime, Utc};
use rusqlite::Result as SqliteResult;
use serde::{Deserialize, Serialize};

use super::super::Database;

/// A node in the mind map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindNode {
    pub id: i64,
    pub body: String,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub is_trunk: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A connection between two mind nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindNodeConnection {
    pub id: i64,
    pub parent_id: i64,
    pub child_id: i64,
    pub created_at: DateTime<Utc>,
}

/// Request to create a new mind node
#[derive(Debug, Deserialize)]
pub struct CreateMindNodeRequest {
    pub body: Option<String>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub parent_id: Option<i64>,
}

/// Request to update a mind node
#[derive(Debug, Deserialize)]
pub struct UpdateMindNodeRequest {
    pub body: Option<String>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
}

/// Full graph response with nodes and connections
#[derive(Debug, Serialize)]
pub struct MindGraphResponse {
    pub nodes: Vec<MindNode>,
    pub connections: Vec<MindNodeConnection>,
}

impl Database {
    /// Get or create the trunk (root) node
    pub fn get_or_create_trunk_node(&self) -> SqliteResult<MindNode> {
        let conn = self.conn.lock().unwrap();

        // Check if trunk exists
        let existing: Option<MindNode> = conn
            .query_row(
                "SELECT id, body, position_x, position_y, is_trunk, created_at, updated_at
                 FROM mind_nodes WHERE is_trunk = 1 LIMIT 1",
                [],
                |row| Self::row_to_mind_node(row),
            )
            .ok();

        if let Some(trunk) = existing {
            return Ok(trunk);
        }

        // Create trunk node
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO mind_nodes (body, position_x, position_y, is_trunk, created_at, updated_at)
             VALUES ('', 0.0, 0.0, 1, ?1, ?1)",
            [&now],
        )?;

        let id = conn.last_insert_rowid();
        let created_at = DateTime::parse_from_rfc3339(&now)
            .unwrap()
            .with_timezone(&Utc);

        Ok(MindNode {
            id,
            body: String::new(),
            position_x: Some(0.0),
            position_y: Some(0.0),
            is_trunk: true,
            created_at,
            updated_at: created_at,
        })
    }

    /// Create a new mind node
    pub fn create_mind_node(&self, request: &CreateMindNodeRequest) -> SqliteResult<MindNode> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let body = request.body.as_deref().unwrap_or("");

        conn.execute(
            "INSERT INTO mind_nodes (body, position_x, position_y, is_trunk, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, ?4, ?4)",
            rusqlite::params![body, request.position_x, request.position_y, &now],
        )?;

        let id = conn.last_insert_rowid();
        let created_at = DateTime::parse_from_rfc3339(&now)
            .unwrap()
            .with_timezone(&Utc);

        // If parent_id is specified, create connection
        if let Some(parent_id) = request.parent_id {
            conn.execute(
                "INSERT INTO mind_node_connections (parent_id, child_id, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![parent_id, id, &now],
            )?;
        }

        Ok(MindNode {
            id,
            body: body.to_string(),
            position_x: request.position_x,
            position_y: request.position_y,
            is_trunk: false,
            created_at,
            updated_at: created_at,
        })
    }

    /// Get a mind node by ID
    pub fn get_mind_node(&self, id: i64) -> SqliteResult<Option<MindNode>> {
        let conn = self.conn.lock().unwrap();
        let node = conn
            .query_row(
                "SELECT id, body, position_x, position_y, is_trunk, created_at, updated_at
                 FROM mind_nodes WHERE id = ?1",
                [id],
                |row| Self::row_to_mind_node(row),
            )
            .ok();
        Ok(node)
    }

    /// List all mind nodes
    pub fn list_mind_nodes(&self) -> SqliteResult<Vec<MindNode>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, body, position_x, position_y, is_trunk, created_at, updated_at
             FROM mind_nodes ORDER BY created_at ASC",
        )?;

        let nodes = stmt
            .query_map([], |row| Self::row_to_mind_node(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(nodes)
    }

    /// Update a mind node
    pub fn update_mind_node(&self, id: i64, request: &UpdateMindNodeRequest) -> SqliteResult<Option<MindNode>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // Build dynamic update
        let mut updates = vec!["updated_at = ?1".to_string()];
        let mut param_idx = 2;

        if request.body.is_some() {
            updates.push(format!("body = ?{}", param_idx));
            param_idx += 1;
        }
        if request.position_x.is_some() {
            updates.push(format!("position_x = ?{}", param_idx));
            param_idx += 1;
        }
        if request.position_y.is_some() {
            updates.push(format!("position_y = ?{}", param_idx));
            param_idx += 1;
        }

        let sql = format!(
            "UPDATE mind_nodes SET {} WHERE id = ?{}",
            updates.join(", "),
            param_idx
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];
        if let Some(ref body) = request.body {
            params.push(Box::new(body.clone()));
        }
        if let Some(x) = request.position_x {
            params.push(Box::new(x));
        }
        if let Some(y) = request.position_y {
            params.push(Box::new(y));
        }
        params.push(Box::new(id));

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_ref.as_slice())?;

        drop(conn);
        self.get_mind_node(id)
    }

    /// Delete a mind node (also deletes connections via CASCADE)
    pub fn delete_mind_node(&self, id: i64) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();

        // Don't allow deleting trunk
        let is_trunk: bool = conn
            .query_row("SELECT is_trunk FROM mind_nodes WHERE id = ?1", [id], |row| row.get(0))
            .unwrap_or(false);

        if is_trunk {
            return Ok(false);
        }

        let rows_affected = conn.execute("DELETE FROM mind_nodes WHERE id = ?1", [id])?;
        Ok(rows_affected > 0)
    }

    /// List all connections
    pub fn list_mind_node_connections(&self) -> SqliteResult<Vec<MindNodeConnection>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, parent_id, child_id, created_at FROM mind_node_connections ORDER BY created_at ASC",
        )?;

        let connections = stmt
            .query_map([], |row| Self::row_to_mind_connection(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(connections)
    }

    /// Create a connection between two nodes
    pub fn create_mind_node_connection(&self, parent_id: i64, child_id: i64) -> SqliteResult<MindNodeConnection> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO mind_node_connections (parent_id, child_id, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![parent_id, child_id, &now],
        )?;

        let id = conn.last_insert_rowid();
        let created_at = DateTime::parse_from_rfc3339(&now)
            .unwrap()
            .with_timezone(&Utc);

        Ok(MindNodeConnection {
            id,
            parent_id,
            child_id,
            created_at,
        })
    }

    /// Delete a connection
    pub fn delete_mind_node_connection(&self, parent_id: i64, child_id: i64) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "DELETE FROM mind_node_connections WHERE parent_id = ?1 AND child_id = ?2",
            [parent_id, child_id],
        )?;
        Ok(rows_affected > 0)
    }

    /// Get the full mind map graph (nodes + connections)
    pub fn get_mind_graph(&self) -> SqliteResult<MindGraphResponse> {
        // Ensure trunk exists
        let _ = self.get_or_create_trunk_node();

        let nodes = self.list_mind_nodes()?;
        let connections = self.list_mind_node_connections()?;

        Ok(MindGraphResponse { nodes, connections })
    }

    /// Get random mind nodes (for future heartbeat integration)
    pub fn get_random_mind_nodes(&self, count: i32) -> SqliteResult<Vec<MindNode>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, body, position_x, position_y, is_trunk, created_at, updated_at
             FROM mind_nodes WHERE body != '' ORDER BY RANDOM() LIMIT ?1",
        )?;

        let nodes = stmt
            .query_map([count], |row| Self::row_to_mind_node(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(nodes)
    }

    fn row_to_mind_node(row: &rusqlite::Row) -> rusqlite::Result<MindNode> {
        let created_at_str: String = row.get(5)?;
        let updated_at_str: String = row.get(6)?;
        let is_trunk_int: i32 = row.get(4)?;

        Ok(MindNode {
            id: row.get(0)?,
            body: row.get(1)?,
            position_x: row.get(2)?,
            position_y: row.get(3)?,
            is_trunk: is_trunk_int != 0,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .unwrap()
                .with_timezone(&Utc),
        })
    }

    fn row_to_mind_connection(row: &rusqlite::Row) -> rusqlite::Result<MindNodeConnection> {
        let created_at_str: String = row.get(3)?;

        Ok(MindNodeConnection {
            id: row.get(0)?,
            parent_id: row.get(1)?,
            child_id: row.get(2)?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .unwrap()
                .with_timezone(&Utc),
        })
    }
}
