//! Channel settings database operations

use rusqlite::Result as SqliteResult;

use crate::models::ChannelSetting;
use super::super::Database;

impl Database {
    /// Get all settings for a channel
    pub fn get_channel_settings(&self, channel_id: i64) -> SqliteResult<Vec<ChannelSetting>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT channel_id, setting_key, setting_value
             FROM channel_settings WHERE channel_id = ?1",
        )?;

        let settings = stmt
            .query_map([channel_id], |row| {
                Ok(ChannelSetting {
                    channel_id: row.get(0)?,
                    setting_key: row.get(1)?,
                    setting_value: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(settings)
    }

    /// Get a single setting value for a channel
    pub fn get_channel_setting(&self, channel_id: i64, key: &str) -> SqliteResult<Option<String>> {
        let conn = self.conn.lock().unwrap();

        let value = conn
            .query_row(
                "SELECT setting_value FROM channel_settings WHERE channel_id = ?1 AND setting_key = ?2",
                rusqlite::params![channel_id, key],
                |row| row.get(0),
            )
            .ok();

        Ok(value)
    }

    /// Set a channel setting (upsert)
    pub fn set_channel_setting(
        &self,
        channel_id: i64,
        key: &str,
        value: &str,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO channel_settings (channel_id, setting_key, setting_value, created_at, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'), datetime('now'))
             ON CONFLICT(channel_id, setting_key) DO UPDATE SET
                setting_value = excluded.setting_value,
                updated_at = datetime('now')",
            rusqlite::params![channel_id, key, value],
        )?;

        Ok(())
    }

    /// Delete a channel setting
    pub fn delete_channel_setting(&self, channel_id: i64, key: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "DELETE FROM channel_settings WHERE channel_id = ?1 AND setting_key = ?2",
            rusqlite::params![channel_id, key],
        )?;
        Ok(rows_affected > 0)
    }

    /// Delete all settings for a channel
    pub fn delete_all_channel_settings(&self, channel_id: i64) -> SqliteResult<usize> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "DELETE FROM channel_settings WHERE channel_id = ?1",
            [channel_id],
        )?;
        Ok(rows_affected)
    }

    /// Bulk update channel settings
    pub fn update_channel_settings(
        &self,
        channel_id: i64,
        settings: &[(String, String)],
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        for (key, value) in settings {
            conn.execute(
                "INSERT INTO channel_settings (channel_id, setting_key, setting_value, created_at, updated_at)
                 VALUES (?1, ?2, ?3, datetime('now'), datetime('now'))
                 ON CONFLICT(channel_id, setting_key) DO UPDATE SET
                    setting_value = excluded.setting_value,
                    updated_at = datetime('now')",
                rusqlite::params![channel_id, key, value],
            )?;
        }

        Ok(())
    }
}
