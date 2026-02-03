//! Channel-specific settings that can be configured per channel instance.
//!
//! Each channel type can have different available settings. The schema
//! defines what settings are available, and values are stored per-channel.

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter, EnumString};

use super::channel::ChannelType;

/// Controls how verbose tool call/result output is in channel messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, EnumString, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ToolOutputVerbosity {
    /// Show tool name and full parameters/content
    #[default]
    Full,
    /// Show only tool name, no parameters or content details
    Minimal,
    /// Don't show tool calls/results at all
    None,
}

impl ToolOutputVerbosity {
    /// Parse from string, defaulting to Full if invalid
    pub fn from_str_or_default(s: &str) -> Self {
        s.parse().unwrap_or_default()
    }
}

/// Available setting keys for channels.
/// Each variant maps to a specific channel type's configurable option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString, AsRefStr, EnumIter)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ChannelSettingKey {
    /// Discord: Comma-separated list of Discord user IDs with admin access
    DiscordAdminUserIds,
    /// Discord: How verbose tool call output should be (full, minimal, none)
    DiscordToolCallVerbosity,
    /// Discord: How verbose tool result output should be (full, minimal, none)
    DiscordToolResultVerbosity,
}

impl ChannelSettingKey {
    /// Get the display label for this setting
    pub fn label(&self) -> &'static str {
        match self {
            Self::DiscordAdminUserIds => "Admin User IDs",
            Self::DiscordToolCallVerbosity => "Tool Call Verbosity",
            Self::DiscordToolResultVerbosity => "Tool Result Verbosity",
        }
    }

    /// Get the description for this setting
    pub fn description(&self) -> &'static str {
        match self {
            Self::DiscordAdminUserIds => {
                "Comma-separated Discord user IDs that have full agent access. \
                 Get your ID by enabling Developer Mode in Discord, then right-click your username."
            }
            Self::DiscordToolCallVerbosity => {
                "Controls how much detail to show when tools are called. \
                 'full' shows tool name and parameters, 'minimal' shows only tool name, 'none' hides tool calls."
            }
            Self::DiscordToolResultVerbosity => {
                "Controls how much detail to show for tool results. \
                 'full' shows tool name and result content, 'minimal' shows only tool name and status, 'none' hides tool results."
            }
        }
    }

    /// Get the input type for the UI
    pub fn input_type(&self) -> SettingInputType {
        match self {
            Self::DiscordAdminUserIds => SettingInputType::Text,
            Self::DiscordToolCallVerbosity => SettingInputType::Select,
            Self::DiscordToolResultVerbosity => SettingInputType::Select,
        }
    }

    /// Get the placeholder text for the input
    pub fn placeholder(&self) -> &'static str {
        match self {
            Self::DiscordAdminUserIds => "123456789012345678, 987654321098765432",
            Self::DiscordToolCallVerbosity => "minimal",
            Self::DiscordToolResultVerbosity => "minimal",
        }
    }

    /// Get the available options for select inputs
    pub fn options(&self) -> Option<Vec<(&'static str, &'static str)>> {
        match self {
            Self::DiscordToolCallVerbosity | Self::DiscordToolResultVerbosity => Some(vec![
                ("full", "Full - Show all details"),
                ("minimal", "Minimal - Tool name only"),
                ("none", "None - Hide completely"),
            ]),
            _ => None,
        }
    }

    /// Get the default value for this setting
    pub fn default_value(&self) -> &'static str {
        match self {
            Self::DiscordAdminUserIds => "",
            Self::DiscordToolCallVerbosity => "minimal",
            Self::DiscordToolResultVerbosity => "minimal",
        }
    }
}

/// Input type for rendering the setting in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingInputType {
    /// Single-line text input
    Text,
    /// Multi-line text area
    TextArea,
    /// Boolean toggle
    Toggle,
    /// Numeric input
    Number,
    /// Dropdown select
    Select,
}

/// Option for select input type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// Definition of a channel setting for the schema API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSettingDefinition {
    pub key: String,
    pub label: String,
    pub description: String,
    pub input_type: SettingInputType,
    pub placeholder: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SelectOption>>,
    pub default_value: String,
}

impl From<ChannelSettingKey> for ChannelSettingDefinition {
    fn from(key: ChannelSettingKey) -> Self {
        Self {
            key: key.as_ref().to_string(),
            label: key.label().to_string(),
            description: key.description().to_string(),
            input_type: key.input_type(),
            placeholder: key.placeholder().to_string(),
            options: key.options().map(|opts| {
                opts.into_iter()
                    .map(|(value, label)| SelectOption {
                        value: value.to_string(),
                        label: label.to_string(),
                    })
                    .collect()
            }),
            default_value: key.default_value().to_string(),
        }
    }
}

/// A stored channel setting value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSetting {
    pub channel_id: i64,
    pub setting_key: String,
    pub setting_value: String,
}

/// Response for channel settings API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSettingsResponse {
    pub success: bool,
    pub settings: Vec<ChannelSetting>,
}

/// Response for channel settings schema API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSettingsSchemaResponse {
    pub success: bool,
    pub channel_type: String,
    pub settings: Vec<ChannelSettingDefinition>,
}

/// Request to update channel settings
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChannelSettingsRequest {
    pub settings: Vec<SettingUpdate>,
}

/// A single setting update
#[derive(Debug, Clone, Deserialize)]
pub struct SettingUpdate {
    pub key: String,
    pub value: String,
}

/// Get the available settings for a channel type
pub fn get_settings_for_channel_type(channel_type: ChannelType) -> Vec<ChannelSettingDefinition> {
    match channel_type {
        ChannelType::Discord => vec![
            ChannelSettingKey::DiscordAdminUserIds.into(),
            ChannelSettingKey::DiscordToolCallVerbosity.into(),
            ChannelSettingKey::DiscordToolResultVerbosity.into(),
        ],
        ChannelType::Telegram => vec![
            // No custom settings yet
        ],
        ChannelType::Slack => vec![
            // No custom settings yet
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setting_key_serialization() {
        let key = ChannelSettingKey::DiscordAdminUserIds;
        assert_eq!(key.as_ref(), "discord_admin_user_ids");
    }

    #[test]
    fn test_discord_settings() {
        let settings = get_settings_for_channel_type(ChannelType::Discord);
        assert_eq!(settings.len(), 3);
        assert_eq!(settings[0].key, "discord_admin_user_ids");
        assert_eq!(settings[1].key, "discord_tool_call_verbosity");
        assert_eq!(settings[2].key, "discord_tool_result_verbosity");
    }

    #[test]
    fn test_telegram_settings() {
        let settings = get_settings_for_channel_type(ChannelType::Telegram);
        assert!(settings.is_empty());
    }

    #[test]
    fn test_tool_verbosity_parsing() {
        assert_eq!(ToolOutputVerbosity::from_str_or_default("full"), ToolOutputVerbosity::Full);
        assert_eq!(ToolOutputVerbosity::from_str_or_default("minimal"), ToolOutputVerbosity::Minimal);
        assert_eq!(ToolOutputVerbosity::from_str_or_default("none"), ToolOutputVerbosity::None);
        assert_eq!(ToolOutputVerbosity::from_str_or_default("invalid"), ToolOutputVerbosity::Full);
    }
}
