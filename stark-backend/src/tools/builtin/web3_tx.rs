//! Generic Web3 transaction signing and broadcasting tool
//!
//! Signs and broadcasts raw EVM transactions using the burner wallet.
//! This is a generic tool - specific tx data is crafted by skills or the agent.
//! All RPC calls go through defirelay.com with x402 payments.

use crate::gateway::events::EventBroadcaster;
use crate::gateway::protocol::GatewayEvent;
use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::x402::X402EvmRpc;
use async_trait::async_trait;
use ethers::prelude::*;
use ethers::types::transaction::eip1559::Eip1559TransactionRequest;
use ethers::types::transaction::eip2718::TypedTransaction;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Web3 transaction tool
pub struct Web3TxTool {
    definition: ToolDefinition,
}

impl Web3TxTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "to".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The recipient address (contract or EOA)".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "data".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Hex-encoded calldata (e.g., '0x...'). Use '0x' for simple ETH transfers.".to_string(),
                default: Some(json!("0x")),
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "value".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Value to send in wei (as decimal string). Default '0'.".to_string(),
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
            "gas_limit".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Gas limit (optional, will estimate if not provided)".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "max_fee_per_gas".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Max fee per gas in wei (optional, will use current gas price if not provided)".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "max_priority_fee_per_gas".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Max priority fee per gas in wei (optional)".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        Web3TxTool {
            definition: ToolDefinition {
                name: "web3_tx".to_string(),
                description: "Sign and broadcast a raw EVM transaction using the burner wallet. Use this to execute swaps, transfers, contract calls, or any on-chain action. Requires BURNER_WALLET_BOT_PRIVATE_KEY.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["to".to_string()],
                },
                group: ToolGroup::Web,
            },
        }
    }

    /// Get the wallet from environment
    fn get_wallet(chain_id: u64) -> Result<LocalWallet, String> {
        let private_key = crate::config::burner_wallet_private_key()
            .ok_or("BURNER_WALLET_BOT_PRIVATE_KEY not set")?;

        private_key
            .parse::<LocalWallet>()
            .map(|w| w.with_chain_id(chain_id))
            .map_err(|e| format!("Invalid private key: {}", e))
    }

    /// Get the private key from environment
    fn get_private_key() -> Result<String, String> {
        crate::config::burner_wallet_private_key()
            .ok_or_else(|| "BURNER_WALLET_BOT_PRIVATE_KEY not set".to_string())
    }

    /// Send a transaction via x402 RPC
    async fn send_transaction(
        network: &str,
        to: &str,
        data: &str,
        value: &str,
        gas_limit: Option<&str>,
        max_fee_per_gas: Option<&str>,
        max_priority_fee_per_gas: Option<&str>,
        broadcaster: Option<&Arc<EventBroadcaster>>,
        channel_id: Option<i64>,
    ) -> Result<(String, String, String), String> {
        let private_key = Self::get_private_key()?;
        let rpc = X402EvmRpc::new(&private_key, network)?;
        let chain_id = rpc.chain_id();

        let wallet = Self::get_wallet(chain_id)?;
        let from_address = wallet.address();
        let from_str = format!("{:?}", from_address);

        // Parse recipient address
        let to_address: Address = to.parse()
            .map_err(|_| format!("Invalid 'to' address: {}", to))?;

        // Parse value
        let tx_value: U256 = value.parse()
            .map_err(|_| format!("Invalid value: {}", value))?;

        // Decode calldata (auto-pad odd-length hex strings)
        let calldata = {
            let hex_str = if data.starts_with("0x") {
                &data[2..]
            } else {
                data
            };
            // Pad with leading zero if odd length (LLMs often forget to zero-pad)
            let padded = if !hex_str.is_empty() && hex_str.len() % 2 != 0 {
                format!("0{}", hex_str)
            } else {
                hex_str.to_string()
            };
            hex::decode(&padded)
                .map_err(|e| format!("Invalid hex data: {}", e))?
        };

        // Get nonce
        let nonce = rpc.get_transaction_count(from_address).await?;

        // Determine gas limit
        let gas = if let Some(gl) = gas_limit {
            gl.parse::<U256>()
                .map_err(|_| format!("Invalid gas_limit: {}", gl))?
        } else {
            // Estimate gas
            let estimate = rpc.estimate_gas(from_address, to_address, &calldata, tx_value).await?;
            // Add 20% buffer
            estimate * 120 / 100
        };

        // Determine gas prices
        let (max_fee, priority_fee) = if let Some(mfpg) = max_fee_per_gas {
            let max_fee = mfpg.parse::<U256>()
                .map_err(|_| format!("Invalid max_fee_per_gas: {}", mfpg))?;

            let priority_fee = if let Some(mpfpg) = max_priority_fee_per_gas {
                mpfpg.parse::<U256>()
                    .map_err(|_| format!("Invalid max_priority_fee_per_gas: {}", mpfpg))?
            } else {
                // Default priority fee to a reasonable value
                U256::from(1_000_000_000u64) // 1 gwei
            };

            (max_fee, priority_fee)
        } else {
            // Estimate fees from network
            rpc.estimate_eip1559_fees().await?
        };

        log::info!(
            "[web3_tx] Sending tx: to={}, value={}, data_len={} bytes, gas={}, nonce={} on {}",
            to, value, calldata.len(), gas, nonce, network
        );

        // Build EIP-1559 transaction
        let tx = Eip1559TransactionRequest::new()
            .from(from_address)
            .to(to_address)
            .value(tx_value)
            .data(calldata)
            .nonce(nonce)
            .gas(gas)
            .max_fee_per_gas(max_fee)
            .max_priority_fee_per_gas(priority_fee)
            .chain_id(chain_id);

        // Sign the transaction locally
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

        log::info!("[web3_tx] Transaction sent: {}", tx_hash_str);

        // Get explorer URL for the tx
        let explorer = if network == "mainnet" {
            "https://etherscan.io/tx"
        } else {
            "https://basescan.org/tx"
        };
        let explorer_url = format!("{}/{}", explorer, tx_hash_str);

        // Emit tx.pending event immediately so frontend can show the hash
        if let (Some(broadcaster), Some(ch_id)) = (broadcaster, channel_id) {
            broadcaster.broadcast(GatewayEvent::tx_pending(
                ch_id,
                &tx_hash_str,
                network,
                &explorer_url,
            ));
            log::info!("[web3_tx] Emitted tx.pending event for {}", tx_hash_str);
        }

        // Wait for receipt (with timeout)
        let receipt = rpc.wait_for_receipt(tx_hash, Duration::from_secs(120)).await?;

        let status = if receipt.status == Some(U64::from(1)) {
            "confirmed".to_string()
        } else {
            "reverted".to_string()
        };

        // Emit tx.confirmed event when the transaction is mined
        if let (Some(broadcaster), Some(ch_id)) = (broadcaster, channel_id) {
            broadcaster.broadcast(GatewayEvent::tx_confirmed(
                ch_id,
                &tx_hash_str,
                network,
                &status,
            ));
            log::info!("[web3_tx] Emitted tx.confirmed event for {} (status={})", tx_hash_str, status);
        }

        Ok((from_str, tx_hash_str, status))
    }
}

impl Default for Web3TxTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct Web3TxParams {
    to: String,
    #[serde(default = "default_data")]
    data: String,
    #[serde(default = "default_value")]
    value: String,
    #[serde(default = "default_network")]
    network: String,
    gas_limit: Option<String>,
    max_fee_per_gas: Option<String>,
    max_priority_fee_per_gas: Option<String>,
}

fn default_data() -> String {
    "0x".to_string()
}

fn default_value() -> String {
    "0".to_string()
}

fn default_network() -> String {
    "base".to_string()
}

#[async_trait]
impl Tool for Web3TxTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: Web3TxParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate network
        if params.network != "base" && params.network != "mainnet" {
            return ToolResult::error("Network must be 'base' or 'mainnet'");
        }

        match Self::send_transaction(
            &params.network,
            &params.to,
            &params.data,
            &params.value,
            params.gas_limit.as_deref(),
            params.max_fee_per_gas.as_deref(),
            params.max_priority_fee_per_gas.as_deref(),
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
                    "Transaction {}\nFrom: {}\nHash: {}\nExplorer: {}/{}",
                    status, from, tx_hash, explorer, tx_hash
                )).with_metadata(json!({
                    "from": from,
                    "to": params.to,
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
