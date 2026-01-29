//! x402 RPC tool for making paid EVM RPC calls via DeFi Relay

use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::x402::X402Client;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// JSON-RPC request structure
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: u64,
}

/// JSON-RPC response structure
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

/// x402 RPC tool for paid EVM RPC calls
pub struct X402RpcTool {
    definition: ToolDefinition,
    client: Arc<RwLock<Option<X402Client>>>,
}

impl X402RpcTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "method".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The JSON-RPC method to call (e.g., 'eth_call', 'eth_getBalance', 'eth_blockNumber')".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "params".to_string(),
            PropertySchema {
                schema_type: "array".to_string(),
                description: "The parameters for the RPC call as a JSON array".to_string(),
                default: Some(json!([])),
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "network".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The network to use: 'base' or 'mainnet'".to_string(),
                default: Some(json!("base")),
                items: None,
                enum_values: Some(vec!["base".to_string(), "mainnet".to_string()]),
            },
        );

        properties.insert(
            "endpoint_type".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Endpoint type: 'light' for standard methods (cheaper), 'heavy' for eth_getLogs, debug_*, trace_* methods".to_string(),
                default: Some(json!("light")),
                items: None,
                enum_values: Some(vec!["light".to_string(), "heavy".to_string()]),
            },
        );

        X402RpcTool {
            definition: ToolDefinition {
                name: "x402_rpc".to_string(),
                description: "Make paid EVM RPC calls via x402 protocol. Costs USDC per request (light: ~0.0001 USDC, heavy: ~0.001 USDC). Use for on-chain queries like balances, contract calls, etc.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["method".to_string()],
                },
                group: ToolGroup::Web,
            },
            client: Arc::new(RwLock::new(None)),
        }
    }

    /// Get or create the x402 client
    async fn get_client(&self) -> Result<X402Client, String> {
        // Check if we have a cached client
        {
            let client_guard = self.client.read().await;
            if let Some(ref client) = *client_guard {
                // We can't clone X402Client, so we need to recreate it each time
                // or store the private key. For now, let's just get the private key again.
            }
        }

        // Get private key from config
        let private_key = crate::config::burner_wallet_private_key()
            .ok_or("BURNER_WALLET_BOT_PRIVATE_KEY environment variable not set")?;

        X402Client::new(&private_key)
    }
}

impl Default for X402RpcTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct X402RpcParams {
    method: String,
    #[serde(default)]
    params: Value,
    #[serde(default = "default_network")]
    network: String,
    #[serde(default = "default_endpoint_type")]
    endpoint_type: String,
}

fn default_network() -> String {
    "base".to_string()
}

fn default_endpoint_type() -> String {
    "light".to_string()
}

#[async_trait]
impl Tool for X402RpcTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> ToolResult {
        let params: X402RpcParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate network
        if params.network != "base" && params.network != "mainnet" {
            return ToolResult::error("Network must be 'base' or 'mainnet'");
        }

        // Validate endpoint type
        if params.endpoint_type != "light" && params.endpoint_type != "heavy" {
            return ToolResult::error("Endpoint type must be 'light' or 'heavy'");
        }

        // Build the RPC URL
        let url = format!(
            "https://rpc.defirelay.com/rpc/{}/{}",
            params.endpoint_type, params.network
        );

        // Ensure params is an array
        let rpc_params = match &params.params {
            Value::Array(_) => params.params.clone(),
            Value::Null => json!([]),
            other => json!([other]),
        };

        // Build JSON-RPC request
        let rpc_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: params.method.clone(),
            params: rpc_params,
            id: 1,
        };

        // Get the x402 client
        let client = match self.get_client().await {
            Ok(c) => c,
            Err(e) => return ToolResult::error(e),
        };

        log::info!("[x402_rpc] Calling {} on {} via {}", params.method, params.network, params.endpoint_type);

        // Make the request with x402 payment handling
        let response = match client.post_with_payment(&url, &rpc_request).await {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("RPC request failed: {}", e)),
        };

        // Check HTTP status
        let status = response.response.status();
        if !status.is_success() {
            let body = response.response.text().await.unwrap_or_default();
            return ToolResult::error(format!("HTTP error {}: {}", status, body));
        }

        // Parse response
        let body = match response.response.text().await {
            Ok(b) => b,
            Err(e) => return ToolResult::error(format!("Failed to read response: {}", e)),
        };

        let rpc_response: JsonRpcResponse = match serde_json::from_str(&body) {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("Invalid JSON-RPC response: {} - Body: {}", e, body)),
        };

        // Check for RPC error
        if let Some(error) = rpc_response.error {
            return ToolResult::error(format!("RPC error {}: {}", error.code, error.message));
        }

        // Build metadata
        let mut metadata = json!({
            "method": params.method,
            "network": params.network,
            "endpoint_type": params.endpoint_type,
            "wallet": client.wallet_address(),
        });

        // Add payment info if a payment was made
        if let Some(payment) = response.payment {
            metadata["payment"] = json!({
                "amount": payment.amount_formatted,
                "asset": payment.asset,
                "pay_to": payment.pay_to,
            });
        }

        // Return the result
        match rpc_response.result {
            Some(result) => ToolResult::success(serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()))
                .with_metadata(metadata),
            None => ToolResult::success("null").with_metadata(metadata),
        }
    }
}
