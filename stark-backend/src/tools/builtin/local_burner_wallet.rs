//! Local burner wallet tool for on-chain interactions
//!
//! Provides access to the local burner wallet configured via BURNER_WALLET_BOT_PRIVATE_KEY.
//! Supports getting address, checking balances, and signing messages.
//! All RPC calls go through defirelay.com with x402 payments.

use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::x402::{erc20, X402EvmRpc};
use async_trait::async_trait;
use ethers::prelude::*;
use ethers::utils::format_units;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Local burner wallet tool
pub struct LocalBurnerWalletTool {
    definition: ToolDefinition,
}

impl LocalBurnerWalletTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "action".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Action: 'address' (get wallet address), 'balance' (check ETH balance), 'token_balance' (check ERC20 balance), 'sign' (sign a message)".to_string(),
                default: Some(json!("address")),
                items: None,
                enum_values: Some(vec![
                    "address".to_string(),
                    "balance".to_string(),
                    "token_balance".to_string(),
                    "sign".to_string(),
                ]),
            },
        );

        properties.insert(
            "network".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Network for balance checks: 'base' or 'mainnet'".to_string(),
                default: Some(json!("base")),
                items: None,
                enum_values: Some(vec!["base".to_string(), "mainnet".to_string()]),
            },
        );

        properties.insert(
            "token".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Token contract address for 'token_balance' action".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "message".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Message to sign for 'sign' action".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        LocalBurnerWalletTool {
            definition: ToolDefinition {
                name: "local_burner_wallet".to_string(),
                description: "Access the local burner wallet. Get address, check balances, sign messages. Requires BURNER_WALLET_BOT_PRIVATE_KEY env var.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["action".to_string()],
                },
                group: ToolGroup::Web,
            },
        }
    }

    /// Get the wallet from environment
    fn get_wallet() -> Result<LocalWallet, String> {
        let private_key = crate::config::burner_wallet_private_key()
            .ok_or("BURNER_WALLET_BOT_PRIVATE_KEY not set")?;

        private_key
            .parse::<LocalWallet>()
            .map_err(|e| format!("Invalid private key: {}", e))
    }

    /// Get the private key from environment
    fn get_private_key() -> Result<String, String> {
        crate::config::burner_wallet_private_key()
            .ok_or_else(|| "BURNER_WALLET_BOT_PRIVATE_KEY not set".to_string())
    }

    /// Get wallet address
    fn get_address() -> Result<String, String> {
        let wallet = Self::get_wallet()?;
        Ok(format!("{:?}", wallet.address()))
    }

    /// Check ETH balance via x402 RPC
    async fn get_balance(network: &str) -> Result<(String, String), String> {
        let wallet = Self::get_wallet()?;
        let address = wallet.address();
        let private_key = Self::get_private_key()?;

        let rpc = X402EvmRpc::new(&private_key, network)?;

        let balance = rpc.get_balance(address).await?;

        let formatted = format_units(balance, "ether")
            .map_err(|e| format!("Failed to format balance: {}", e))?;

        Ok((format!("{:?}", address), formatted))
    }

    /// Check ERC20 token balance via x402 RPC
    async fn get_token_balance(network: &str, token_address: &str) -> Result<(String, String, String), String> {
        let wallet = Self::get_wallet()?;
        let address = wallet.address();
        let private_key = Self::get_private_key()?;

        let token: Address = token_address
            .parse()
            .map_err(|_| "Invalid token address")?;

        let rpc = X402EvmRpc::new(&private_key, network)?;

        // Get balance
        let balance_data = erc20::encode_balance_of(address);
        let balance_result = rpc.eth_call(token, &balance_data).await?;
        let balance = erc20::decode_balance(&balance_result)
            .map_err(|e| format!("Failed to decode balance: {}", e))?;

        // Get decimals (default to 18 if call fails)
        let decimals = match rpc.eth_call(token, &erc20::encode_decimals()).await {
            Ok(data) => erc20::decode_decimals(&data).unwrap_or(18),
            Err(_) => 18,
        };

        // Get symbol (default to "TOKEN" if call fails)
        let symbol = match rpc.eth_call(token, &erc20::encode_symbol()).await {
            Ok(data) => erc20::decode_symbol(&data).unwrap_or_else(|_| "TOKEN".to_string()),
            Err(_) => "TOKEN".to_string(),
        };

        let formatted = format_units(balance, decimals as u32)
            .map_err(|e| format!("Failed to format balance: {}", e))?;

        Ok((format!("{:?}", address), formatted, symbol))
    }

    /// Sign a message
    async fn sign_message(message: &str) -> Result<(String, String), String> {
        let wallet = Self::get_wallet()?;
        let address = format!("{:?}", wallet.address());

        let signature = wallet
            .sign_message(message)
            .await
            .map_err(|e| format!("Failed to sign message: {}", e))?;

        Ok((address, format!("0x{}", hex::encode(signature.to_vec()))))
    }
}

impl Default for LocalBurnerWalletTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct WalletParams {
    action: String,
    #[serde(default = "default_network")]
    network: String,
    token: Option<String>,
    message: Option<String>,
}

fn default_network() -> String {
    "base".to_string()
}

#[async_trait]
impl Tool for LocalBurnerWalletTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> ToolResult {
        let params: WalletParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        match params.action.as_str() {
            "address" => {
                match Self::get_address() {
                    Ok(address) => ToolResult::success(format!("Wallet address: {}", address))
                        .with_metadata(json!({"address": address})),
                    Err(e) => ToolResult::error(e),
                }
            }

            "balance" => {
                match Self::get_balance(&params.network).await {
                    Ok((address, balance)) => {
                        let symbol = if params.network == "mainnet" { "ETH" } else { "ETH" };
                        ToolResult::success(format!(
                            "Wallet: {}\nBalance: {} {} ({})",
                            address, balance, symbol, params.network
                        )).with_metadata(json!({
                            "address": address,
                            "balance": balance,
                            "network": params.network
                        }))
                    }
                    Err(e) => ToolResult::error(e),
                }
            }

            "token_balance" => {
                let token = match params.token {
                    Some(t) => t,
                    None => return ToolResult::error("'token' address is required for token_balance action"),
                };

                match Self::get_token_balance(&params.network, &token).await {
                    Ok((address, balance, symbol)) => {
                        ToolResult::success(format!(
                            "Wallet: {}\nToken: {} ({})\nBalance: {} ({})",
                            address, token, symbol, balance, params.network
                        )).with_metadata(json!({
                            "address": address,
                            "token": token,
                            "symbol": symbol,
                            "balance": balance,
                            "network": params.network
                        }))
                    }
                    Err(e) => ToolResult::error(e),
                }
            }

            "sign" => {
                let message = match params.message {
                    Some(m) => m,
                    None => return ToolResult::error("'message' is required for sign action"),
                };

                match Self::sign_message(&message).await {
                    Ok((address, signature)) => {
                        ToolResult::success(format!(
                            "Signed by: {}\nMessage: {}\nSignature: {}",
                            address, message, signature
                        )).with_metadata(json!({
                            "address": address,
                            "message": message,
                            "signature": signature
                        }))
                    }
                    Err(e) => ToolResult::error(e),
                }
            }

            _ => ToolResult::error(format!("Unknown action: {}", params.action)),
        }
    }
}
