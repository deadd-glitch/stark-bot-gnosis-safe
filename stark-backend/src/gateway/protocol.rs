use crate::models::{ExecutionTask, TaskMetrics};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Event types for gateway broadcasts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    // Channel events
    ChannelStarted,
    ChannelStopped,
    ChannelError,
    ChannelMessage,
    // Agent events
    AgentResponse,
    AgentToolCall,  // Real-time tool call notification for chat display
    // Tool events
    ToolExecution,
    ToolResult,
    // Skill events
    SkillInvoked,
    // Execution progress events
    ExecutionStarted,
    ExecutionThinking,
    ExecutionTaskStarted,
    ExecutionTaskUpdated,
    ExecutionTaskCompleted,
    ExecutionCompleted,
    // Payment events
    X402Payment,
    // Confirmation events
    ConfirmationRequired,
    ConfirmationApproved,
    ConfirmationRejected,
    ConfirmationExpired,
    // Transaction events
    TxPending,
    TxConfirmed,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChannelStarted => "channel.started",
            Self::ChannelStopped => "channel.stopped",
            Self::ChannelError => "channel.error",
            Self::ChannelMessage => "channel.message",
            Self::AgentResponse => "agent.response",
            Self::AgentToolCall => "agent.tool_call",
            Self::ToolExecution => "tool.execution",
            Self::ToolResult => "tool.result",
            Self::SkillInvoked => "skill.invoked",
            Self::ExecutionStarted => "execution.started",
            Self::ExecutionThinking => "execution.thinking",
            Self::ExecutionTaskStarted => "execution.task_started",
            Self::ExecutionTaskUpdated => "execution.task_updated",
            Self::ExecutionTaskCompleted => "execution.task_completed",
            Self::ExecutionCompleted => "execution.completed",
            Self::X402Payment => "x402.payment",
            Self::ConfirmationRequired => "confirmation.required",
            Self::ConfirmationApproved => "confirmation.approved",
            Self::ConfirmationRejected => "confirmation.rejected",
            Self::ConfirmationExpired => "confirmation.expired",
            Self::TxPending => "tx.pending",
            Self::TxConfirmed => "tx.confirmed",
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<EventType> for String {
    fn from(event_type: EventType) -> Self {
        event_type.as_str().to_string()
    }
}

/// JSON-RPC request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC response to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub fn success(id: String, result: Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: String, error: RpcError) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid request")
    }

    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(-32602, message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(-32603, message)
    }
}

/// Server-push event to all connected clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEvent {
    #[serde(rename = "type")]
    pub type_: String,
    pub event: String,
    pub data: Value,
}

impl GatewayEvent {
    pub fn new(event: impl Into<String>, data: Value) -> Self {
        Self {
            type_: "event".to_string(),
            event: event.into(),
            data,
        }
    }

    pub fn channel_started(channel_id: i64, channel_type: &str, name: &str) -> Self {
        Self::new(
            EventType::ChannelStarted,
            serde_json::json!({
                "channel_id": channel_id,
                "channel_type": channel_type,
                "name": name
            }),
        )
    }

    pub fn channel_stopped(channel_id: i64, channel_type: &str, name: &str) -> Self {
        Self::new(
            EventType::ChannelStopped,
            serde_json::json!({
                "channel_id": channel_id,
                "channel_type": channel_type,
                "name": name
            }),
        )
    }

    pub fn channel_error(channel_id: i64, error: &str) -> Self {
        Self::new(
            EventType::ChannelError,
            serde_json::json!({
                "channel_id": channel_id,
                "error": error
            }),
        )
    }

    pub fn channel_message(
        channel_id: i64,
        channel_type: &str,
        from: &str,
        text: &str,
    ) -> Self {
        Self::new(
            EventType::ChannelMessage,
            serde_json::json!({
                "channel_id": channel_id,
                "channel_type": channel_type,
                "from": from,
                "text": text
            }),
        )
    }

    pub fn agent_response(channel_id: i64, to: &str, text: &str) -> Self {
        Self::new(
            EventType::AgentResponse,
            serde_json::json!({
                "channel_id": channel_id,
                "to": to,
                "text": text
            }),
        )
    }

    /// Emit a tool call notification for real-time display in chat
    pub fn agent_tool_call(channel_id: i64, tool_name: &str, parameters: &Value) -> Self {
        Self::new(
            EventType::AgentToolCall,
            serde_json::json!({
                "channel_id": channel_id,
                "tool_name": tool_name,
                "parameters": parameters
            }),
        )
    }

    pub fn tool_execution(channel_id: i64, tool_name: &str, parameters: &Value) -> Self {
        Self::new(
            EventType::ToolExecution,
            serde_json::json!({
                "channel_id": channel_id,
                "tool_name": tool_name,
                "parameters": parameters
            }),
        )
    }

    pub fn tool_result(channel_id: i64, tool_name: &str, success: bool, duration_ms: i64, content: &str) -> Self {
        Self::new(
            EventType::ToolResult,
            serde_json::json!({
                "channel_id": channel_id,
                "tool_name": tool_name,
                "success": success,
                "duration_ms": duration_ms,
                "content": content
            }),
        )
    }

    pub fn skill_invoked(channel_id: i64, skill_name: &str) -> Self {
        Self::new(
            EventType::SkillInvoked,
            serde_json::json!({
                "channel_id": channel_id,
                "skill_name": skill_name
            }),
        )
    }

    // =====================================================
    // Execution Progress Events
    // =====================================================

    /// Execution started (plan mode or direct execution)
    pub fn execution_started(
        channel_id: i64,
        execution_id: &str,
        mode: &str,
        description: &str,
        active_form: &str,
    ) -> Self {
        Self::new(
            EventType::ExecutionStarted,
            serde_json::json!({
                "channel_id": channel_id,
                "execution_id": execution_id,
                "mode": mode,  // "plan" or "execute"
                "description": description,
                "active_form": active_form
            }),
        )
    }

    /// AI is thinking/reasoning
    pub fn execution_thinking(channel_id: i64, execution_id: &str, text: &str) -> Self {
        Self::new(
            EventType::ExecutionThinking,
            serde_json::json!({
                "channel_id": channel_id,
                "execution_id": execution_id,
                "text": text
            }),
        )
    }

    /// Task started (tool, sub-agent, etc.)
    pub fn task_started(task: &ExecutionTask, execution_id: &str) -> Self {
        Self::new(
            EventType::ExecutionTaskStarted,
            serde_json::json!({
                "id": task.id,
                "execution_id": execution_id,
                "parent_id": task.parent_id,
                "parent_task_id": task.parent_id,  // Alias for frontend compatibility
                "channel_id": task.channel_id,
                "type": task.task_type.to_string(),
                "name": task.description,  // Frontend expects 'name' field
                "description": task.description,
                "active_form": task.active_form,
                "status": task.status.to_string()
            }),
        )
    }

    /// Task metrics updated
    pub fn task_updated(task_id: &str, channel_id: i64, metrics: &TaskMetrics) -> Self {
        Self::new(
            EventType::ExecutionTaskUpdated,
            serde_json::json!({
                "task_id": task_id,
                "channel_id": channel_id,
                "metrics": {
                    "tool_uses": metrics.tool_uses,
                    "tokens_used": metrics.tokens_used,
                    "lines_read": metrics.lines_read,
                    "duration_ms": metrics.duration_ms
                }
            }),
        )
    }

    /// Task completed
    pub fn task_completed(task_id: &str, channel_id: i64, status: &str, metrics: &TaskMetrics) -> Self {
        Self::new(
            EventType::ExecutionTaskCompleted,
            serde_json::json!({
                "task_id": task_id,
                "channel_id": channel_id,
                "status": status,
                "metrics": {
                    "tool_uses": metrics.tool_uses,
                    "tokens_used": metrics.tokens_used,
                    "lines_read": metrics.lines_read,
                    "duration_ms": metrics.duration_ms
                }
            }),
        )
    }

    /// Execution completed
    pub fn execution_completed(channel_id: i64, execution_id: &str, total_metrics: &TaskMetrics) -> Self {
        Self::new(
            EventType::ExecutionCompleted,
            serde_json::json!({
                "channel_id": channel_id,
                "execution_id": execution_id,
                "metrics": {
                    "tool_uses": total_metrics.tool_uses,
                    "tokens_used": total_metrics.tokens_used,
                    "duration_ms": total_metrics.duration_ms
                }
            }),
        )
    }

    // =====================================================
    // Confirmation Events
    // =====================================================

    /// Confirmation required for a tool execution
    pub fn confirmation_required(
        channel_id: i64,
        confirmation_id: &str,
        tool_name: &str,
        description: &str,
        parameters: &Value,
    ) -> Self {
        Self::new(
            EventType::ConfirmationRequired,
            serde_json::json!({
                "channel_id": channel_id,
                "confirmation_id": confirmation_id,
                "tool_name": tool_name,
                "description": description,
                "parameters": parameters,
                "instructions": "Type /confirm to execute or /cancel to abort",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )
    }

    /// Confirmation approved and tool executing
    pub fn confirmation_approved(channel_id: i64, confirmation_id: &str, tool_name: &str) -> Self {
        Self::new(
            EventType::ConfirmationApproved,
            serde_json::json!({
                "channel_id": channel_id,
                "confirmation_id": confirmation_id,
                "tool_name": tool_name,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )
    }

    /// Confirmation rejected by user
    pub fn confirmation_rejected(channel_id: i64, confirmation_id: &str, tool_name: &str) -> Self {
        Self::new(
            EventType::ConfirmationRejected,
            serde_json::json!({
                "channel_id": channel_id,
                "confirmation_id": confirmation_id,
                "tool_name": tool_name,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )
    }

    /// Confirmation expired
    pub fn confirmation_expired(channel_id: i64, confirmation_id: &str, tool_name: &str) -> Self {
        Self::new(
            EventType::ConfirmationExpired,
            serde_json::json!({
                "channel_id": channel_id,
                "confirmation_id": confirmation_id,
                "tool_name": tool_name,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )
    }

    /// Custom event with arbitrary event name and data
    pub fn custom(event: &str, data: Value) -> Self {
        Self::new(event, data)
    }

    /// Transaction pending - broadcast when tx is sent but not yet mined
    pub fn tx_pending(
        channel_id: i64,
        tx_hash: &str,
        network: &str,
        explorer_url: &str,
    ) -> Self {
        Self::new(
            EventType::TxPending,
            serde_json::json!({
                "channel_id": channel_id,
                "tx_hash": tx_hash,
                "network": network,
                "explorer_url": explorer_url,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )
    }

    /// Transaction confirmed - broadcast when tx is mined
    pub fn tx_confirmed(
        channel_id: i64,
        tx_hash: &str,
        network: &str,
        status: &str,
    ) -> Self {
        Self::new(
            EventType::TxConfirmed,
            serde_json::json!({
                "channel_id": channel_id,
                "tx_hash": tx_hash,
                "network": network,
                "status": status,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )
    }

    /// x402 payment made
    pub fn x402_payment(
        channel_id: i64,
        amount: &str,
        amount_formatted: &str,
        asset: &str,
        pay_to: &str,
        resource: Option<&str>,
    ) -> Self {
        Self::new(
            EventType::X402Payment,
            serde_json::json!({
                "channel_id": channel_id,
                "amount": amount,
                "amount_formatted": amount_formatted,
                "asset": asset,
                "pay_to": pay_to,
                "resource": resource,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )
    }
}

/// Params for channel operations
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelIdParams {
    pub id: i64,
}
