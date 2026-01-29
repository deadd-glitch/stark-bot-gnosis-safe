pub mod builtin;
pub mod registry;
pub mod types;

pub use registry::{Tool, ToolRegistry};
pub use types::{
    PropertySchema, ToolConfig, ToolContext, ToolDefinition, ToolExecution, ToolGroup,
    ToolInputSchema, ToolProfile, ToolResult,
};

use std::sync::Arc;

/// Register all built-in tools to a registry
fn register_all_tools(registry: &mut ToolRegistry) {
    // Web tools
    registry.register(Arc::new(builtin::WebFetchTool::new()));
    registry.register(Arc::new(builtin::X402RpcTool::new()));
    registry.register(Arc::new(builtin::X402FetchTool::new()));
    registry.register(Arc::new(builtin::LocalBurnerWalletTool::new()));
    registry.register(Arc::new(builtin::Web3TxTool::new()));
    registry.register(Arc::new(builtin::Web3FunctionCallTool::new()));

    // Filesystem tools
    registry.register(Arc::new(builtin::ReadFileTool::new()));
    registry.register(Arc::new(builtin::WriteFileTool::new()));
    registry.register(Arc::new(builtin::ListFilesTool::new()));
    registry.register(Arc::new(builtin::ApplyPatchTool::new()));

    // Exec tool
    registry.register(Arc::new(builtin::ExecTool::new()));

    // Messaging tools
    registry.register(Arc::new(builtin::AgentSendTool::new()));

    // System tools (subagents)
    registry.register(Arc::new(builtin::SubagentTool::new()));
    registry.register(Arc::new(builtin::SubagentStatusTool::new()));
}

/// Create a new ToolRegistry with all built-in tools registered
pub fn create_default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    register_all_tools(&mut registry);
    registry
}

/// Create a registry with specific configuration
pub fn create_registry_with_config(config: ToolConfig) -> ToolRegistry {
    let mut registry = ToolRegistry::with_config(config);
    register_all_tools(&mut registry);
    registry
}
