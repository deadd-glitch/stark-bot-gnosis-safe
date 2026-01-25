pub mod dispatcher;
pub mod slack;
pub mod telegram;
pub mod types;

pub use dispatcher::MessageDispatcher;
pub use types::ChannelHandle;

use crate::db::Database;
use crate::gateway::events::EventBroadcaster;
use crate::gateway::protocol::GatewayEvent;
use crate::models::Channel;
use crate::tools::ToolRegistry;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Manages all running channel listeners
pub struct ChannelManager {
    db: Arc<Database>,
    broadcaster: Arc<EventBroadcaster>,
    running_channels: Arc<DashMap<i64, ChannelHandle>>,
    tool_registry: Option<Arc<ToolRegistry>>,
}

impl ChannelManager {
    pub fn new(db: Arc<Database>, broadcaster: Arc<EventBroadcaster>) -> Self {
        Self {
            db,
            broadcaster,
            running_channels: Arc::new(DashMap::new()),
            tool_registry: None,
        }
    }

    pub fn new_with_tools(
        db: Arc<Database>,
        broadcaster: Arc<EventBroadcaster>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            db,
            broadcaster,
            running_channels: Arc::new(DashMap::new()),
            tool_registry: Some(tool_registry),
        }
    }

    /// Check if a channel is currently running
    pub fn is_running(&self, channel_id: i64) -> bool {
        self.running_channels.contains_key(&channel_id)
    }

    /// Get list of running channel IDs
    pub fn running_channel_ids(&self) -> Vec<i64> {
        self.running_channels.iter().map(|e| *e.key()).collect()
    }

    /// Start a channel listener
    pub async fn start_channel(&self, channel: Channel) -> Result<(), String> {
        let channel_id = channel.id;
        let channel_type = channel.channel_type.clone();
        let channel_name = channel.name.clone();

        // Check if already running
        if self.is_running(channel_id) {
            return Err(format!("Channel {} is already running", channel_id));
        }

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Create dispatcher with or without tools
        let dispatcher = if let Some(ref tool_registry) = self.tool_registry {
            Arc::new(MessageDispatcher::new(
                self.db.clone(),
                self.broadcaster.clone(),
                tool_registry.clone(),
            ))
        } else {
            Arc::new(MessageDispatcher::new_without_tools(
                self.db.clone(),
                self.broadcaster.clone(),
            ))
        };

        // Store handle
        let handle = ChannelHandle::new(
            channel_id,
            channel_type.clone(),
            channel_name.clone(),
            shutdown_tx,
        );
        self.running_channels.insert(channel_id, handle);

        // Start the appropriate listener
        let broadcaster = self.broadcaster.clone();
        let running_channels = self.running_channels.clone();

        match channel_type.as_str() {
            "telegram" => {
                tokio::spawn(async move {
                    let result = telegram::start_telegram_listener(
                        channel,
                        dispatcher,
                        broadcaster.clone(),
                        shutdown_rx,
                    )
                    .await;

                    if let Err(e) = result {
                        log::error!("Telegram listener error: {}", e);
                        broadcaster.broadcast(GatewayEvent::channel_error(channel_id, &e));
                    }

                    // Remove from running channels
                    running_channels.remove(&channel_id);
                });
            }
            "slack" => {
                tokio::spawn(async move {
                    let result = slack::start_slack_listener(
                        channel,
                        dispatcher,
                        broadcaster.clone(),
                        shutdown_rx,
                    )
                    .await;

                    if let Err(e) = result {
                        log::error!("Slack listener error: {}", e);
                        broadcaster.broadcast(GatewayEvent::channel_error(channel_id, &e));
                    }

                    // Remove from running channels
                    running_channels.remove(&channel_id);
                });
            }
            other => {
                // Remove the handle we just added
                self.running_channels.remove(&channel_id);
                return Err(format!("Unknown channel type: {}", other));
            }
        }

        log::info!(
            "Started {} channel listener: {} (id={})",
            channel_type,
            channel_name,
            channel_id
        );

        Ok(())
    }

    /// Stop a channel listener
    pub async fn stop_channel(&self, channel_id: i64) -> Result<(), String> {
        match self.running_channels.remove(&channel_id) {
            Some((_, handle)) => {
                log::info!(
                    "Stopping {} channel: {} (id={})",
                    handle.channel_type,
                    handle.name,
                    channel_id
                );

                // Send shutdown signal
                let _ = handle.shutdown_tx.send(());

                Ok(())
            }
            None => Err(format!("Channel {} is not running", channel_id)),
        }
    }

    /// Stop all running channels
    pub async fn stop_all(&self) {
        let ids: Vec<i64> = self.running_channels.iter().map(|e| *e.key()).collect();
        for id in ids {
            let _ = self.stop_channel(id).await;
        }
    }
}
