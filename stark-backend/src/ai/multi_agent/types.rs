//! Agent types

use serde::{Deserialize, Serialize};

use crate::tools::types::ToolGroup;

/// The specialized mode/persona of the agent
/// Controls which tools and skills are available (acts as a "toolbox")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentSubtype {
    /// No subtype selected yet - agent MUST choose one before using other tools
    #[default]
    None,
    /// Finance/DeFi specialist - crypto swaps, transfers, web3 operations
    Finance,
    /// Code engineer - software development, git, code editing
    CodeEngineer,
    /// Secretary - social media, marketing, messaging, scheduling
    Secretary,
}

impl AgentSubtype {
    /// Get all selectable subtypes (excludes None)
    pub fn all() -> Vec<AgentSubtype> {
        vec![
            AgentSubtype::Finance,
            AgentSubtype::CodeEngineer,
            AgentSubtype::Secretary,
        ]
    }

    /// Check if a subtype has been selected
    pub fn is_selected(&self) -> bool {
        !matches!(self, AgentSubtype::None)
    }

    /// Get the tool groups allowed for this subtype
    /// Note: When None, only System tools are available (to allow set_agent_subtype)
    pub fn allowed_tool_groups(&self) -> Vec<ToolGroup> {
        match self {
            AgentSubtype::None => {
                // Only system tools when no subtype selected
                // This forces the agent to call set_agent_subtype first
                vec![ToolGroup::System]
            }
            _ => {
                // Core groups available to all selected subtypes
                let mut groups = vec![
                    ToolGroup::System,     // set_agent_subtype, subagent
                    ToolGroup::Web,        // web_fetch
                    ToolGroup::Filesystem, // read_file, list_files
                ];

                // Add subtype-specific groups
                match self {
                    AgentSubtype::Finance => {
                        groups.push(ToolGroup::Finance); // web3_tx, token_lookup, x402_*, etc.
                    }
                    AgentSubtype::CodeEngineer => {
                        groups.push(ToolGroup::Development); // edit_file, grep, glob, git, etc.
                        groups.push(ToolGroup::Exec);        // exec command
                    }
                    AgentSubtype::Secretary => {
                        groups.push(ToolGroup::Messaging); // agent_send
                        groups.push(ToolGroup::Social);    // moltx, scheduling tools
                    }
                    AgentSubtype::None => unreachable!(), // Handled above
                }

                groups
            }
        }
    }

    /// Get the skill tags allowed for this subtype
    /// Note: "general" and "all" tags are available to ALL subtypes
    /// When None, no skills are available (must select subtype first)
    pub fn allowed_skill_tags(&self) -> Vec<&'static str> {
        match self {
            AgentSubtype::None => {
                // No skills available until subtype is selected
                vec![]
            }
            _ => {
                // Universal tags available to all selected subtypes
                let mut tags = vec!["general", "all"];

                // Add subtype-specific tags
                match self {
                    AgentSubtype::Finance => tags.extend(["crypto", "defi", "transfer", "swap", "finance", "wallet", "token"]),
                    AgentSubtype::CodeEngineer => tags.extend(["development", "git", "testing", "debugging", "review", "code", "github"]),
                    AgentSubtype::Secretary => tags.extend(["social", "marketing", "messaging", "moltx", "scheduling", "communication", "social-media"]),
                    AgentSubtype::None => unreachable!(),
                }

                tags
            }
        }
    }

    /// Human-readable label for UI display
    pub fn label(&self) -> &'static str {
        match self {
            AgentSubtype::None => "Selecting...",
            AgentSubtype::Finance => "Finance",
            AgentSubtype::CodeEngineer => "CodeEngineer",
            AgentSubtype::Secretary => "Secretary",
        }
    }

    /// Get description of what this subtype does
    pub fn description(&self) -> &'static str {
        match self {
            AgentSubtype::None => "No toolbox selected - must choose one first",
            AgentSubtype::Finance => "Crypto swaps, transfers, DeFi operations, token lookups",
            AgentSubtype::CodeEngineer => "Code editing, git operations, testing, debugging",
            AgentSubtype::Secretary => "Social media, messaging, scheduling, marketing",
        }
    }

    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentSubtype::None => "none",
            AgentSubtype::Finance => "finance",
            AgentSubtype::CodeEngineer => "code_engineer",
            AgentSubtype::Secretary => "secretary",
        }
    }

    /// Parse from string (does not parse "none" - use None variant directly)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "finance" | "defi" | "crypto" | "swap" | "transfer" => Some(AgentSubtype::Finance),
            "code_engineer" | "codeengineer" | "code" | "dev" | "developer" | "git" => {
                Some(AgentSubtype::CodeEngineer)
            }
            "secretary" | "social" | "marketing" | "messaging" | "moltx" => {
                Some(AgentSubtype::Secretary)
            }
            _ => None,
        }
    }

    /// Get emoji for this subtype
    pub fn emoji(&self) -> &'static str {
        match self {
            AgentSubtype::None => "❓",
            AgentSubtype::Finance => "💰",
            AgentSubtype::CodeEngineer => "🛠️",
            AgentSubtype::Secretary => "📱",
        }
    }
}

impl std::fmt::Display for AgentSubtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The current mode of the agent (simplified - single mode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// Active assistant mode - handles all tasks
    Assistant,
}

impl Default for AgentMode {
    fn default() -> Self {
        AgentMode::Assistant
    }
}

impl std::fmt::Display for AgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentMode::Assistant => write!(f, "assistant"),
        }
    }
}

impl AgentMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "assistant" | "explore" | "plan" | "perform" | "execute" => Some(AgentMode::Assistant),
            _ => None,
        }
    }

    /// Check if skills are available in this mode
    pub fn allows_skills(&self) -> bool {
        true // Always allow skills
    }

    /// Check if action tools (swap, transfer, etc.) are available in this mode
    pub fn allows_action_tools(&self) -> bool {
        true // Always allow action tools
    }

    /// Human-readable label for UI display
    pub fn label(&self) -> &'static str {
        match self {
            AgentMode::Assistant => "Assistant",
        }
    }
}

/// Context accumulated during the agent session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    /// Original user request
    pub original_request: String,

    /// Notes gathered during the session
    pub exploration_notes: Vec<String>,

    /// Current mode (always Assistant)
    pub mode: AgentMode,

    /// Current agent subtype/specialization
    #[serde(default)]
    pub subtype: AgentSubtype,

    /// Number of iterations in current session
    pub mode_iterations: u32,

    /// Total iterations
    pub total_iterations: u32,

    /// Scratchpad for agent notes
    pub scratchpad: String,

    /// Currently active skill context
    #[serde(default)]
    pub active_skill: Option<ActiveSkill>,

    /// Total actual tool calls made (excludes orchestrator tools)
    #[serde(default)]
    pub actual_tool_calls: u32,

    /// Number of times the agent tried to respond without calling tools
    #[serde(default)]
    pub no_tool_warnings: u32,

    /// Context saved when waiting for user response (e.g., from ask_user tool).
    /// Contains a summary of what tool calls were made before asking the user,
    /// so the AI can continue where it left off when the user responds.
    #[serde(default)]
    pub waiting_for_user_context: Option<String>,
}

/// Active skill context that persists across turns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSkill {
    /// Name of the skill
    pub name: String,
    /// Skill instructions/body
    pub instructions: String,
    /// When the skill was activated
    pub activated_at: String,
    /// Number of actual tool calls made since this skill was activated
    #[serde(default)]
    pub tool_calls_made: u32,
    /// Tools required by this skill - these are force-included in the toolset
    /// regardless of tool profile/config restrictions
    #[serde(default)]
    pub requires_tools: Vec<String>,
}

/// Mode transition (kept for API compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeTransition {
    pub from: AgentMode,
    pub to: AgentMode,
    pub reason: String,
}

// =====================================================
// Sub-Agent System Types
// =====================================================

/// Status of a sub-agent execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentStatus {
    /// Waiting to be started
    Pending,
    /// Currently executing
    Running,
    /// Successfully completed
    Completed,
    /// Failed with an error
    Failed,
    /// Timed out during execution
    TimedOut,
    /// Cancelled by user or system
    Cancelled,
}

impl Default for SubAgentStatus {
    fn default() -> Self {
        SubAgentStatus::Pending
    }
}

impl std::fmt::Display for SubAgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubAgentStatus::Pending => write!(f, "pending"),
            SubAgentStatus::Running => write!(f, "running"),
            SubAgentStatus::Completed => write!(f, "completed"),
            SubAgentStatus::Failed => write!(f, "failed"),
            SubAgentStatus::TimedOut => write!(f, "timed_out"),
            SubAgentStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl SubAgentStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(SubAgentStatus::Pending),
            "running" => Some(SubAgentStatus::Running),
            "completed" => Some(SubAgentStatus::Completed),
            "failed" => Some(SubAgentStatus::Failed),
            "timed_out" | "timedout" => Some(SubAgentStatus::TimedOut),
            "cancelled" | "canceled" => Some(SubAgentStatus::Cancelled),
            _ => None,
        }
    }

    /// Check if this status represents a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, SubAgentStatus::Completed | SubAgentStatus::Failed | SubAgentStatus::TimedOut | SubAgentStatus::Cancelled)
    }
}

/// Context for a sub-agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentContext {
    /// Unique identifier for this sub-agent
    pub id: String,
    /// Parent session ID that spawned this sub-agent
    pub parent_session_id: i64,
    /// Channel ID where the sub-agent was spawned
    pub parent_channel_id: i64,
    /// Session ID for this sub-agent's conversation (created when execution starts)
    pub session_id: Option<i64>,
    /// Human-readable label for this sub-agent
    pub label: String,
    /// The task/prompt for the sub-agent to work on
    pub task: String,
    /// Current execution status
    pub status: SubAgentStatus,
    /// Optional model override (uses parent's model if None)
    pub model_override: Option<String>,
    /// Timeout in seconds for this sub-agent
    pub timeout_secs: u64,
    /// Result of the execution (set on completion)
    pub result: Option<String>,
    /// Error message (set on failure)
    pub error: Option<String>,
    /// When the sub-agent was created
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When the sub-agent completed (success, failure, or timeout)
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Additional context passed from the parent
    pub context: Option<String>,
    /// Thinking level for Claude models
    pub thinking_level: Option<String>,
}

impl SubAgentContext {
    /// Create a new sub-agent context
    pub fn new(
        id: String,
        parent_session_id: i64,
        parent_channel_id: i64,
        label: String,
        task: String,
        timeout_secs: u64,
    ) -> Self {
        Self {
            id,
            parent_session_id,
            parent_channel_id,
            session_id: None,
            label,
            task,
            status: SubAgentStatus::Pending,
            model_override: None,
            timeout_secs,
            result: None,
            error: None,
            started_at: chrono::Utc::now(),
            completed_at: None,
            context: None,
            thinking_level: None,
        }
    }

    /// Set the model override
    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model_override = model;
        self
    }

    /// Set additional context
    pub fn with_context(mut self, context: Option<String>) -> Self {
        self.context = context;
        self
    }

    /// Set thinking level
    pub fn with_thinking(mut self, level: Option<String>) -> Self {
        self.thinking_level = level;
        self
    }

    /// Mark the sub-agent as running
    pub fn mark_running(&mut self, session_id: i64) {
        self.status = SubAgentStatus::Running;
        self.session_id = Some(session_id);
    }

    /// Mark the sub-agent as completed
    pub fn mark_completed(&mut self, result: String) {
        self.status = SubAgentStatus::Completed;
        self.result = Some(result);
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark the sub-agent as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = SubAgentStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark the sub-agent as timed out
    pub fn mark_timed_out(&mut self) {
        self.status = SubAgentStatus::TimedOut;
        self.error = Some(format!("Execution timed out after {} seconds", self.timeout_secs));
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark the sub-agent as cancelled
    pub fn mark_cancelled(&mut self) {
        self.status = SubAgentStatus::Cancelled;
        self.error = Some("Cancelled by user or system".to_string());
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Get the duration of the execution
    pub fn duration(&self) -> chrono::Duration {
        match self.completed_at {
            Some(completed) => completed - self.started_at,
            None => chrono::Utc::now() - self.started_at,
        }
    }
}

/// Configuration for the sub-agent system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentConfig {
    /// Maximum concurrent sub-agents per channel
    pub max_concurrent_per_channel: usize,
    /// Maximum total concurrent sub-agents system-wide
    pub max_total_concurrent: usize,
    /// Default timeout in seconds for sub-agents
    pub default_timeout_secs: u64,
    /// Maximum timeout allowed (cannot exceed this)
    pub max_timeout_secs: u64,
}

impl Default for SubAgentConfig {
    fn default() -> Self {
        Self {
            max_concurrent_per_channel: 3,
            max_total_concurrent: 10,
            default_timeout_secs: 300,
            max_timeout_secs: 3600,
        }
    }
}
