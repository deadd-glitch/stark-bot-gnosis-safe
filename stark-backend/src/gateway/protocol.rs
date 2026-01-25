use crate::models::{ExecutionTask, TaskMetrics};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
            "channel.started",
            serde_json::json!({
                "channel_id": channel_id,
                "channel_type": channel_type,
                "name": name
            }),
        )
    }

    pub fn channel_stopped(channel_id: i64, channel_type: &str, name: &str) -> Self {
        Self::new(
            "channel.stopped",
            serde_json::json!({
                "channel_id": channel_id,
                "channel_type": channel_type,
                "name": name
            }),
        )
    }

    pub fn channel_error(channel_id: i64, error: &str) -> Self {
        Self::new(
            "channel.error",
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
            "channel.message",
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
            "agent.response",
            serde_json::json!({
                "channel_id": channel_id,
                "to": to,
                "text": text
            }),
        )
    }

    pub fn tool_execution(channel_id: i64, tool_name: &str, parameters: &Value) -> Self {
        Self::new(
            "tool.execution",
            serde_json::json!({
                "channel_id": channel_id,
                "tool_name": tool_name,
                "parameters": parameters
            }),
        )
    }

    pub fn tool_result(channel_id: i64, tool_name: &str, success: bool, duration_ms: i64) -> Self {
        Self::new(
            "tool.result",
            serde_json::json!({
                "channel_id": channel_id,
                "tool_name": tool_name,
                "success": success,
                "duration_ms": duration_ms
            }),
        )
    }

    pub fn skill_invoked(channel_id: i64, skill_name: &str) -> Self {
        Self::new(
            "skill.invoked",
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
    pub fn execution_started(channel_id: i64, execution_id: &str, mode: &str) -> Self {
        Self::new(
            "execution.started",
            serde_json::json!({
                "channel_id": channel_id,
                "execution_id": execution_id,
                "mode": mode  // "plan" or "execute"
            }),
        )
    }

    /// AI is thinking/reasoning
    pub fn execution_thinking(channel_id: i64, execution_id: &str, text: &str) -> Self {
        Self::new(
            "execution.thinking",
            serde_json::json!({
                "channel_id": channel_id,
                "execution_id": execution_id,
                "text": text
            }),
        )
    }

    /// Task started (tool, sub-agent, etc.)
    pub fn task_started(task: &ExecutionTask) -> Self {
        Self::new(
            "execution.task_started",
            serde_json::json!({
                "id": task.id,
                "parent_id": task.parent_id,
                "channel_id": task.channel_id,
                "type": task.task_type.to_string(),
                "description": task.description,
                "active_form": task.active_form,
                "status": task.status.to_string()
            }),
        )
    }

    /// Task metrics updated
    pub fn task_updated(task_id: &str, channel_id: i64, metrics: &TaskMetrics) -> Self {
        Self::new(
            "execution.task_updated",
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
            "execution.task_completed",
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
            "execution.completed",
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
}

/// Params for channel operations
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelIdParams {
    pub id: i64,
}
