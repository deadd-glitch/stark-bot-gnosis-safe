pub mod events;
pub mod methods;
pub mod protocol;
pub mod server;

pub use events::EventBroadcaster;
pub use server::GatewayServer;

use crate::channels::ChannelManager;
use crate::db::Database;
use crate::tools::ToolRegistry;
use std::net::SocketAddr;
use std::sync::Arc;

/// Main Gateway struct that owns all channel connections and exposes WebSocket RPC
pub struct Gateway {
    db: Arc<Database>,
    channel_manager: Arc<ChannelManager>,
    broadcaster: Arc<EventBroadcaster>,
}

impl Gateway {
    pub fn new(db: Arc<Database>) -> Self {
        let broadcaster = Arc::new(EventBroadcaster::new());
        let channel_manager = Arc::new(ChannelManager::new(db.clone(), broadcaster.clone()));

        Self {
            db,
            channel_manager,
            broadcaster,
        }
    }

    /// Create a new Gateway with tool registry support
    pub fn new_with_tools(db: Arc<Database>, tool_registry: Arc<ToolRegistry>) -> Self {
        let broadcaster = Arc::new(EventBroadcaster::new());
        let channel_manager = Arc::new(ChannelManager::new_with_tools(
            db.clone(),
            broadcaster.clone(),
            tool_registry,
        ));

        Self {
            db,
            channel_manager,
            broadcaster,
        }
    }

    /// Start the Gateway WebSocket server
    pub async fn start(&self, port: u16) {
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        let server = GatewayServer::new(
            self.db.clone(),
            self.channel_manager.clone(),
            self.broadcaster.clone(),
        );

        if let Err(e) = server.run(addr).await {
            log::error!("Gateway server error: {}", e);
        }
    }

    /// Start all channels that are marked as enabled in the database
    pub async fn start_enabled_channels(&self) {
        match self.db.list_enabled_channels() {
            Ok(channels) => {
                for channel in channels {
                    let id = channel.id;
                    let name = channel.name.clone();
                    let channel_type = channel.channel_type.clone();

                    match self.channel_manager.start_channel(channel).await {
                        Ok(()) => {
                            log::info!("Started {} channel: {}", channel_type, name);
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to start {} channel {}: {}",
                                channel_type,
                                name,
                                e
                            );
                            // Disable the channel in DB since it failed to start
                            let _ = self.db.set_channel_enabled(id, false);
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to load enabled channels: {}", e);
            }
        }
    }

    /// Get the event broadcaster for emitting events
    pub fn broadcaster(&self) -> Arc<EventBroadcaster> {
        self.broadcaster.clone()
    }

    /// Get the channel manager
    pub fn channel_manager(&self) -> Arc<ChannelManager> {
        self.channel_manager.clone()
    }
}
