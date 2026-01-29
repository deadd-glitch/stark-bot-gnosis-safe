//! Web3 Function Call tool - call any contract function using ABI
//!
//! This tool loads ABIs from the /abis folder and encodes function calls,
//! so the LLM doesn't have to deal with hex-encoded calldata.

use crate::gateway::events::EventBroadcaster;
use crate::gateway::protocol::GatewayEvent;
use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::x402::X402EvmRpc;
use async_trait::async_trait;
use ethers::abi::{Abi, Function, Token, ParamType};
use ethers::prelude::*;
use ethers::types::transaction::eip1559::Eip1559TransactionRequest;
use ethers::types::transaction::eip2718::TypedTransaction;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Web3 function call tool
pub struct Web3FunctionCallTool {
    definition: ToolDefinition,
    abis_dir: PathBuf,
}

impl Web3FunctionCallTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "abi".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Name of the ABI file (without .json). Available: 'erc20', '0x_settler'".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "contract".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Contract address to call".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "function".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Function name to call (e.g., 'approve', 'transfer', 'balanceOf')".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "params".to_string(),
            PropertySchema {
                schema_type: "array".to_string(),
                description: "Function parameters as an array. Use strings for addresses and numbers, booleans for bool. Order must match the function signature.".to_string(),
                default: Some(json!([])),
                items: Some(Box::new(PropertySchema {
                    schema_type: "string".to_string(),
                    description: "Parameter value".to_string(),
                    default: None,
                    items: None,
                    enum_values: None,
                })),
                enum_values: None,
            },
        );

        properties.insert(
            "value".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "ETH value to send in wei (as decimal string). Default '0'.".to_string(),
                default: Some(json!("0")),
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "network".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Network: 'base' or 'mainnet'".to_string(),
                default: Some(json!("base")),
                items: None,
                enum_values: Some(vec!["base".to_string(), "mainnet".to_string()]),
            },
        );

        properties.insert(
            "call_only".to_string(),
            PropertySchema {
                schema_type: "boolean".to_string(),
                description: "If true, perform a read-only call (no transaction). Use for view/pure functions like balanceOf.".to_string(),
                default: Some(json!(false)),
                items: None,
                enum_values: None,
            },
        );

        // Determine abis directory relative to working directory
        let abis_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("abis");

        Web3FunctionCallTool {
            definition: ToolDefinition {
                name: "web3_function_call".to_string(),
                description: "Call a smart contract function using a known ABI. Handles parameter encoding automatically. Use for ERC20 approve/transfer, or any contract with an ABI in the /abis folder. For read-only calls (balanceOf, etc), set call_only=true.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["abi".to_string(), "contract".to_string(), "function".to_string()],
                },
                group: ToolGroup::Web,
            },
            abis_dir,
        }
    }

    /// Load ABI from file
    fn load_abi(&self, name: &str) -> Result<AbiFile, String> {
        let path = self.abis_dir.join(format!("{}.json", name));

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to load ABI '{}': {}. Available ABIs are in the /abis folder.", name, e))?;

        let abi_file: AbiFile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse ABI '{}': {}", name, e))?;

        Ok(abi_file)
    }

    /// Parse ethers Abi from our ABI file format
    fn parse_abi(&self, abi_file: &AbiFile) -> Result<Abi, String> {
        let abi_json = serde_json::to_string(&abi_file.abi)
            .map_err(|e| format!("Failed to serialize ABI: {}", e))?;

        serde_json::from_str(&abi_json)
            .map_err(|e| format!("Failed to parse ABI: {}", e))
    }

    /// Find function in ABI
    fn find_function<'a>(&self, abi: &'a Abi, name: &str) -> Result<&'a Function, String> {
        abi.function(name)
            .map_err(|_| format!("Function '{}' not found in ABI", name))
    }

    /// Convert JSON value to ethers Token based on param type
    fn value_to_token(&self, value: &Value, param_type: &ParamType) -> Result<Token, String> {
        match param_type {
            ParamType::Address => {
                let s = value.as_str()
                    .ok_or_else(|| format!("Expected string for address, got {:?}", value))?;
                let addr: Address = s.parse()
                    .map_err(|_| format!("Invalid address: {}", s))?;
                Ok(Token::Address(addr))
            }
            ParamType::Uint(bits) => {
                let s = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => return Err(format!("Expected string or number for uint{}, got {:?}", bits, value)),
                };
                let n: U256 = s.parse()
                    .map_err(|_| format!("Invalid uint{}: {}", bits, s))?;
                Ok(Token::Uint(n))
            }
            ParamType::Int(bits) => {
                let s = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => return Err(format!("Expected string or number for int{}, got {:?}", bits, value)),
                };
                // Parse as signed - ethers handles the conversion
                let n: I256 = s.parse()
                    .map_err(|_| format!("Invalid int{}: {}", bits, s))?;
                Ok(Token::Int(n.into_raw()))
            }
            ParamType::Bool => {
                let b = value.as_bool()
                    .ok_or_else(|| format!("Expected boolean, got {:?}", value))?;
                Ok(Token::Bool(b))
            }
            ParamType::String => {
                let s = value.as_str()
                    .ok_or_else(|| format!("Expected string, got {:?}", value))?;
                Ok(Token::String(s.to_string()))
            }
            ParamType::Bytes => {
                let s = value.as_str()
                    .ok_or_else(|| format!("Expected hex string for bytes, got {:?}", value))?;
                let hex_str = s.strip_prefix("0x").unwrap_or(s);
                let bytes = hex::decode(hex_str)
                    .map_err(|e| format!("Invalid hex for bytes: {}", e))?;
                Ok(Token::Bytes(bytes))
            }
            ParamType::FixedBytes(size) => {
                let s = value.as_str()
                    .ok_or_else(|| format!("Expected hex string for bytes{}, got {:?}", size, value))?;
                let hex_str = s.strip_prefix("0x").unwrap_or(s);
                let bytes = hex::decode(hex_str)
                    .map_err(|e| format!("Invalid hex for bytes{}: {}", size, e))?;
                if bytes.len() != *size {
                    return Err(format!("Expected {} bytes, got {}", size, bytes.len()));
                }
                Ok(Token::FixedBytes(bytes))
            }
            ParamType::Array(inner) => {
                let arr = value.as_array()
                    .ok_or_else(|| format!("Expected array, got {:?}", value))?;
                let tokens: Result<Vec<Token>, String> = arr.iter()
                    .map(|v| self.value_to_token(v, inner))
                    .collect();
                Ok(Token::Array(tokens?))
            }
            ParamType::Tuple(types) => {
                let arr = value.as_array()
                    .ok_or_else(|| format!("Expected array for tuple, got {:?}", value))?;
                if arr.len() != types.len() {
                    return Err(format!("Tuple expects {} elements, got {}", types.len(), arr.len()));
                }
                let tokens: Result<Vec<Token>, String> = arr.iter()
                    .zip(types.iter())
                    .map(|(v, t)| self.value_to_token(v, t))
                    .collect();
                Ok(Token::Tuple(tokens?))
            }
            ParamType::FixedArray(inner, size) => {
                let arr = value.as_array()
                    .ok_or_else(|| format!("Expected array, got {:?}", value))?;
                if arr.len() != *size {
                    return Err(format!("Fixed array expects {} elements, got {}", size, arr.len()));
                }
                let tokens: Result<Vec<Token>, String> = arr.iter()
                    .map(|v| self.value_to_token(v, inner))
                    .collect();
                Ok(Token::FixedArray(tokens?))
            }
        }
    }

    /// Encode function call
    fn encode_call(&self, function: &Function, params: &[Value]) -> Result<Vec<u8>, String> {
        if params.len() != function.inputs.len() {
            return Err(format!(
                "Function '{}' expects {} parameters, got {}. Expected: {:?}",
                function.name,
                function.inputs.len(),
                params.len(),
                function.inputs.iter().map(|i| format!("{}: {}", i.name, i.kind)).collect::<Vec<_>>()
            ));
        }

        let tokens: Result<Vec<Token>, String> = params.iter()
            .zip(function.inputs.iter())
            .map(|(value, input)| self.value_to_token(value, &input.kind))
            .collect();

        let tokens = tokens?;

        function.encode_input(&tokens)
            .map_err(|e| format!("Failed to encode function call: {}", e))
    }

    /// Get wallet from environment
    fn get_wallet(chain_id: u64) -> Result<LocalWallet, String> {
        let private_key = crate::config::burner_wallet_private_key()
            .ok_or("BURNER_WALLET_BOT_PRIVATE_KEY not set")?;

        private_key
            .parse::<LocalWallet>()
            .map(|w| w.with_chain_id(chain_id))
            .map_err(|e| format!("Invalid private key: {}", e))
    }

    /// Get private key from environment
    fn get_private_key() -> Result<String, String> {
        crate::config::burner_wallet_private_key()
            .ok_or_else(|| "BURNER_WALLET_BOT_PRIVATE_KEY not set".to_string())
    }

    /// Execute a read-only call
    async fn call_function(
        network: &str,
        to: Address,
        calldata: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        let private_key = Self::get_private_key()?;
        let rpc = X402EvmRpc::new(&private_key, network)?;

        rpc.call(to, &calldata).await
    }

    /// Send a transaction
    async fn send_transaction(
        network: &str,
        to: Address,
        calldata: Vec<u8>,
        value: U256,
        broadcaster: Option<&Arc<EventBroadcaster>>,
        channel_id: Option<i64>,
    ) -> Result<(String, String, String), String> {
        let private_key = Self::get_private_key()?;
        let rpc = X402EvmRpc::new(&private_key, network)?;
        let chain_id = rpc.chain_id();

        let wallet = Self::get_wallet(chain_id)?;
        let from_address = wallet.address();
        let from_str = format!("{:?}", from_address);

        // Get nonce
        let nonce = rpc.get_transaction_count(from_address).await?;

        // Estimate gas
        let gas = rpc.estimate_gas(from_address, to, &calldata, value).await?;
        let gas = gas * 120 / 100; // 20% buffer

        // Get gas prices
        let (max_fee, priority_fee) = rpc.estimate_eip1559_fees().await?;

        log::info!(
            "[web3_function_call] Sending tx: to={:?}, value={}, data_len={} bytes, gas={}, nonce={} on {}",
            to, value, calldata.len(), gas, nonce, network
        );

        // Build EIP-1559 transaction
        let tx = Eip1559TransactionRequest::new()
            .from(from_address)
            .to(to)
            .value(value)
            .data(calldata)
            .nonce(nonce)
            .gas(gas)
            .max_fee_per_gas(max_fee)
            .max_priority_fee_per_gas(priority_fee)
            .chain_id(chain_id);

        // Sign the transaction
        let typed_tx: TypedTransaction = tx.into();
        let signature = wallet
            .sign_transaction(&typed_tx)
            .await
            .map_err(|e| format!("Failed to sign transaction: {}", e))?;

        // Serialize the signed transaction
        let signed_tx = typed_tx.rlp_signed(&signature);

        // Broadcast via x402 RPC
        let tx_hash = rpc.send_raw_transaction(&signed_tx).await?;
        let tx_hash_str = format!("{:?}", tx_hash);

        log::info!("[web3_function_call] Transaction sent: {}", tx_hash_str);

        // Get explorer URL
        let explorer = if network == "mainnet" {
            "https://etherscan.io/tx"
        } else {
            "https://basescan.org/tx"
        };
        let explorer_url = format!("{}/{}", explorer, tx_hash_str);

        // Emit tx.pending event
        if let (Some(broadcaster), Some(ch_id)) = (broadcaster, channel_id) {
            broadcaster.broadcast(GatewayEvent::tx_pending(
                ch_id,
                &tx_hash_str,
                network,
                &explorer_url,
            ));
        }

        // Wait for receipt
        let receipt = rpc.wait_for_receipt(tx_hash, Duration::from_secs(120)).await?;

        let status = if receipt.status == Some(U64::from(1)) {
            "confirmed".to_string()
        } else {
            "reverted".to_string()
        };

        // Emit tx.confirmed event
        if let (Some(broadcaster), Some(ch_id)) = (broadcaster, channel_id) {
            broadcaster.broadcast(GatewayEvent::tx_confirmed(
                ch_id,
                &tx_hash_str,
                network,
                &status,
            ));
        }

        Ok((from_str, tx_hash_str, status))
    }

    /// Decode return value from a call
    fn decode_return(&self, function: &Function, data: &[u8]) -> Result<Value, String> {
        let tokens = function.decode_output(data)
            .map_err(|e| format!("Failed to decode return value: {}", e))?;

        // Convert tokens to JSON
        let values: Vec<Value> = tokens.iter().map(|t| self.token_to_value(t)).collect();

        if values.len() == 1 {
            Ok(values.into_iter().next().unwrap())
        } else {
            Ok(Value::Array(values))
        }
    }

    /// Convert ethers Token to JSON value
    fn token_to_value(&self, token: &Token) -> Value {
        match token {
            Token::Address(a) => json!(format!("{:?}", a)),
            Token::Uint(n) => json!(n.to_string()),
            Token::Int(n) => json!(I256::from_raw(*n).to_string()),
            Token::Bool(b) => json!(b),
            Token::String(s) => json!(s),
            Token::Bytes(b) => json!(format!("0x{}", hex::encode(b))),
            Token::FixedBytes(b) => json!(format!("0x{}", hex::encode(b))),
            Token::Array(arr) | Token::FixedArray(arr) => {
                json!(arr.iter().map(|t| self.token_to_value(t)).collect::<Vec<_>>())
            }
            Token::Tuple(tuple) => {
                json!(tuple.iter().map(|t| self.token_to_value(t)).collect::<Vec<_>>())
            }
        }
    }
}

impl Default for Web3FunctionCallTool {
    fn default() -> Self {
        Self::new()
    }
}

/// ABI file structure
#[derive(Debug, Deserialize)]
struct AbiFile {
    name: String,
    #[serde(default)]
    description: String,
    abi: Vec<Value>,
    #[serde(default)]
    address: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct Web3FunctionCallParams {
    abi: String,
    contract: String,
    function: String,
    #[serde(default)]
    params: Vec<Value>,
    #[serde(default = "default_value")]
    value: String,
    #[serde(default = "default_network")]
    network: String,
    #[serde(default)]
    call_only: bool,
}

fn default_value() -> String {
    "0".to_string()
}

fn default_network() -> String {
    "base".to_string()
}

#[async_trait]
impl Tool for Web3FunctionCallTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: Web3FunctionCallParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate network
        if params.network != "base" && params.network != "mainnet" {
            return ToolResult::error("Network must be 'base' or 'mainnet'");
        }

        // Load ABI
        let abi_file = match self.load_abi(&params.abi) {
            Ok(a) => a,
            Err(e) => return ToolResult::error(e),
        };

        // Parse ABI
        let abi = match self.parse_abi(&abi_file) {
            Ok(a) => a,
            Err(e) => return ToolResult::error(e),
        };

        // Find function
        let function = match self.find_function(&abi, &params.function) {
            Ok(f) => f,
            Err(e) => return ToolResult::error(e),
        };

        // Encode call
        let calldata = match self.encode_call(function, &params.params) {
            Ok(d) => d,
            Err(e) => return ToolResult::error(e),
        };

        // Parse contract address
        let contract: Address = match params.contract.parse() {
            Ok(a) => a,
            Err(_) => return ToolResult::error(format!("Invalid contract address: {}", params.contract)),
        };

        log::info!(
            "[web3_function_call] {}::{}({:?}) on {} (call_only={})",
            params.abi, params.function, params.params, params.network, params.call_only
        );

        if params.call_only {
            // Read-only call
            match Self::call_function(&params.network, contract, calldata).await {
                Ok(result) => {
                    let decoded = self.decode_return(function, &result)
                        .unwrap_or_else(|_| json!(format!("0x{}", hex::encode(&result))));

                    ToolResult::success(serde_json::to_string_pretty(&decoded).unwrap_or_default())
                        .with_metadata(json!({
                            "abi": params.abi,
                            "contract": params.contract,
                            "function": params.function,
                            "result": decoded,
                        }))
                }
                Err(e) => ToolResult::error(e),
            }
        } else {
            // Transaction
            let value: U256 = match params.value.parse() {
                Ok(v) => v,
                Err(_) => return ToolResult::error(format!("Invalid value: {}", params.value)),
            };

            match Self::send_transaction(
                &params.network,
                contract,
                calldata,
                value,
                context.broadcaster.as_ref(),
                context.channel_id,
            ).await {
                Ok((from, tx_hash, status)) => {
                    let explorer = if params.network == "mainnet" {
                        "https://etherscan.io/tx"
                    } else {
                        "https://basescan.org/tx"
                    };

                    ToolResult::success(format!(
                        "Transaction {}\nFunction: {}::{}()\nFrom: {}\nTo: {}\nHash: {}\nExplorer: {}/{}",
                        status, params.abi, params.function, from, params.contract, tx_hash, explorer, tx_hash
                    )).with_metadata(json!({
                        "abi": params.abi,
                        "contract": params.contract,
                        "function": params.function,
                        "from": from,
                        "tx_hash": tx_hash,
                        "status": status,
                        "network": params.network,
                        "explorer_url": format!("{}/{}", explorer, tx_hash)
                    }))
                }
                Err(e) => ToolResult::error(e),
            }
        }
    }
}
