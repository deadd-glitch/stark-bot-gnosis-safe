use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Bot settings stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotSettings {
    pub id: i64,
    pub bot_name: String,
    pub bot_email: String,
    pub web3_tx_requires_confirmation: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for BotSettings {
    fn default() -> Self {
        Self {
            id: 0,
            bot_name: "StarkBot".to_string(),
            bot_email: "starkbot@users.noreply.github.com".to_string(),
            web3_tx_requires_confirmation: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Request type for updating bot settings
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBotSettingsRequest {
    pub bot_name: Option<String>,
    pub bot_email: Option<String>,
    pub web3_tx_requires_confirmation: Option<bool>,
}
