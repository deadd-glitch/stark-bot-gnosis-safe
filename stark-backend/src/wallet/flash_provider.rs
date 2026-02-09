//! Flash Mode Wallet Provider
//!
//! Proxies signing requests to the Flash control plane, which uses Privy
//! for secure key management. The private key never leaves Privy's infrastructure.
//!
//! Required environment variables:
//! - FLASH_KEYSTORE_URL: URL of the Flash control plane (e.g., https://flash.starkbot.io)
//! - FLASH_TENANT_ID: Tenant identifier
//! - FLASH_INSTANCE_TOKEN: Authentication token for this instance

use async_trait::async_trait;
use ethers::types::{H256, Signature, U256, transaction::eip2718::TypedTransaction};
use ethers::utils::rlp;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::WalletProvider;

/// Environment variables for Flash mode
pub mod env_vars {
    pub const FLASH_KEYSTORE_URL: &str = "FLASH_KEYSTORE_URL";
    pub const FLASH_TENANT_ID: &str = "FLASH_TENANT_ID";
    pub const FLASH_INSTANCE_TOKEN: &str = "FLASH_INSTANCE_TOKEN";
}

/// Response from the Flash keystore wallet endpoint
#[derive(Debug, Deserialize)]
struct KeystoreWalletResponse {
    wallet_id: String,
    admin_address: String,
}

/// Request body for sign-message endpoint
#[derive(Debug, Serialize)]
struct SignMessageRequest {
    message: String,
}

/// Response from sign-message endpoint
#[derive(Debug, Deserialize)]
struct SignMessageResponse {
    signature: String,
}

/// Request body for sign-transaction endpoint
#[derive(Debug, Serialize)]
struct SignTransactionRequest {
    chain_id: u64,
    to: String,
    value: String,
    data: Option<String>,
    gas_limit: Option<String>,
    max_fee_per_gas: Option<String>,
    max_priority_fee_per_gas: Option<String>,
    nonce: Option<u64>,
}

/// Response from sign-transaction endpoint
#[derive(Debug, Deserialize)]
struct SignTransactionResponse {
    signed_transaction: String,
}

/// Request body for sign-typed-data endpoint
#[derive(Debug, Serialize)]
struct SignTypedDataRequest {
    typed_data: serde_json::Value,
}

/// Response from sign-typed-data endpoint
#[derive(Debug, Deserialize)]
struct SignTypedDataResponse {
    signature: String,
}

/// Response from refresh-token endpoint
#[derive(Debug, Deserialize)]
struct RefreshTokenResponse {
    token: String,
}

/// Wallet provider that proxies signing to Flash control plane
pub struct FlashWalletProvider {
    keystore_url: String,
    tenant_id: String,
    instance_token: Arc<RwLock<String>>,
    /// Wallet address - fetched from control plane on init
    address: String,
    /// Privy wallet ID - used for signing requests
    wallet_id: String,
    http_client: reqwest::Client,
}

impl FlashWalletProvider {
    /// Create a new Flash wallet provider from environment variables
    ///
    /// On initialization, fetches wallet info from control plane to:
    /// 1. Validate credentials
    /// 2. Get the wallet address and ID
    pub async fn new() -> Result<Self, String> {
        let keystore_url = std::env::var(env_vars::FLASH_KEYSTORE_URL)
            .map_err(|_| format!("{} not set", env_vars::FLASH_KEYSTORE_URL))?;

        let tenant_id = std::env::var(env_vars::FLASH_TENANT_ID)
            .map_err(|_| format!("{} not set", env_vars::FLASH_TENANT_ID))?;

        let instance_token = std::env::var(env_vars::FLASH_INSTANCE_TOKEN)
            .map_err(|_| format!("{} not set", env_vars::FLASH_INSTANCE_TOKEN))?;

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let instance_token = Arc::new(RwLock::new(instance_token));

        // Fetch wallet info from control plane
        log::info!("Fetching wallet info from Flash control plane...");
        let url = format!("{}/api/keystore/wallet", keystore_url);

        let token = instance_token.read().await.clone();
        let response = http_client
            .get(&url)
            .header("X-Tenant-ID", &tenant_id)
            .header("X-Instance-Token", &token)
            .send()
            .await
            .map_err(|e| format!("Flash keystore request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Flash keystore error ({}): {}", status, body));
        }

        let data: KeystoreWalletResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse keystore response: {}", e))?;

        log::info!(
            "Flash wallet initialized: {} (wallet_id: {})",
            data.admin_address,
            data.wallet_id
        );

        Ok(Self {
            keystore_url,
            tenant_id,
            instance_token,
            address: data.admin_address,
            wallet_id: data.wallet_id,
            http_client,
        })
    }

    /// Parse an Ethereum signature from hex string
    fn parse_signature(sig_hex: &str) -> Result<Signature, String> {
        let sig_hex = sig_hex.strip_prefix("0x").unwrap_or(sig_hex);

        if sig_hex.len() != 130 {
            return Err(format!(
                "Invalid signature length: expected 130 hex chars, got {}",
                sig_hex.len()
            ));
        }

        let sig_bytes = hex::decode(sig_hex)
            .map_err(|e| format!("Invalid signature hex: {}", e))?;

        if sig_bytes.len() != 65 {
            return Err(format!(
                "Invalid signature bytes: expected 65, got {}",
                sig_bytes.len()
            ));
        }

        let r = U256::from_big_endian(&sig_bytes[0..32]);
        let s = U256::from_big_endian(&sig_bytes[32..64]);
        let v_byte = sig_bytes[64];

        // Normalize v value: Privy may return 0/1, but ecrecover needs 27/28
        // EIP-712 typed data signatures use v = 27 or 28
        let v = if v_byte < 27 {
            v_byte as u64 + 27
        } else {
            v_byte as u64
        };

        log::debug!("Parsed signature: v={} (raw byte was {})", v, v_byte);

        Ok(Signature { r, s, v })
    }

    /// Extract (v, r, s) from a Privy-returned RLP-encoded signed transaction.
    ///
    /// For EIP-1559 (type 2), the format is:
    ///   0x02 ++ rlp([chain_id, nonce, max_priority_fee, max_fee, gas, to, value, data, access_list, y_parity, r, s])
    /// The last 3 items in the RLP list are the signature: y_parity, r, s.
    fn extract_signature_from_signed_tx(signed_tx_hex: &str) -> Result<Signature, String> {
        let hex_str = signed_tx_hex.strip_prefix("0x").unwrap_or(signed_tx_hex);
        let tx_bytes = hex::decode(hex_str)
            .map_err(|e| format!("Invalid signed tx hex: {}", e))?;

        if tx_bytes.is_empty() {
            return Err("Empty signed transaction".to_string());
        }

        // EIP-2718 typed transactions start with a type byte (0x01 or 0x02)
        // followed by the RLP payload
        let tx_type = tx_bytes[0];
        if tx_type != 0x02 && tx_type != 0x01 {
            return Err(format!(
                "Unsupported transaction type: 0x{:02x} (expected 0x01 or 0x02)",
                tx_type
            ));
        }

        // Decode the RLP list after the type byte
        let rlp_data = rlp::Rlp::new(&tx_bytes[1..]);
        let item_count = rlp_data.item_count()
            .map_err(|e| format!("Failed to decode RLP: {}", e))?;

        // EIP-1559 (type 2): 12 items [chain_id, nonce, max_priority, max_fee, gas, to, value, data, access_list, y_parity, r, s]
        // EIP-2930 (type 1): 11 items [chain_id, nonce, gas_price, gas, to, value, data, access_list, y_parity, r, s]
        if item_count < 11 {
            return Err(format!(
                "Unexpected RLP item count: {} (expected >= 11)",
                item_count
            ));
        }

        // Signature is always the last 3 items: y_parity, r, s
        let y_parity: u64 = rlp_data.val_at(item_count - 3)
            .map_err(|e| format!("Failed to decode y_parity: {}", e))?;
        let r: U256 = rlp_data.val_at(item_count - 2)
            .map_err(|e| format!("Failed to decode r: {}", e))?;
        let s: U256 = rlp_data.val_at(item_count - 1)
            .map_err(|e| format!("Failed to decode s: {}", e))?;

        // For EIP-1559/2930 typed transactions, v = y_parity (0 or 1)
        // ethers expects v as 0 or 1 for typed transactions (NOT 27/28)
        let v = y_parity;

        log::debug!(
            "Extracted signature from signed tx (type 0x{:02x}): v={}, r={:#x}, s={:#x}",
            tx_type, v, r, s
        );

        Ok(Signature { r, s, v })
    }

    /// Refresh the instance token by calling the control plane's refresh endpoint.
    /// The control plane accepts expired tokens (up to 30 days old) and returns a fresh one.
    async fn refresh_instance_token(&self) -> Result<(), String> {
        let url = format!("{}/api/keystore/refresh-token", self.keystore_url);
        let old_token = self.instance_token.read().await.clone();

        log::info!("Refreshing expired instance token...");

        let response = self.http_client
            .post(&url)
            .header("X-Tenant-ID", &self.tenant_id)
            .header("X-Instance-Token", &old_token)
            .send()
            .await
            .map_err(|e| format!("Token refresh request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Token refresh failed ({}): {}", status, body));
        }

        let data: RefreshTokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse refresh response: {}", e))?;

        let mut token = self.instance_token.write().await;
        *token = data.token;

        log::info!("Instance token refreshed successfully");
        Ok(())
    }

    /// Send a POST request with instance token auth headers.
    /// On 401, attempts to refresh the token and retry once.
    async fn post_with_retry<T: Serialize + ?Sized>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<reqwest::Response, String> {
        let token = self.instance_token.read().await.clone();

        let response = self.http_client
            .post(url)
            .header("X-Tenant-ID", &self.tenant_id)
            .header("X-Instance-Token", &token)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            log::warn!("Got 401 from control plane, attempting token refresh...");

            self.refresh_instance_token().await?;

            let new_token = self.instance_token.read().await.clone();
            let retry_response = self.http_client
                .post(url)
                .header("X-Tenant-ID", &self.tenant_id)
                .header("X-Instance-Token", &new_token)
                .json(body)
                .send()
                .await
                .map_err(|e| format!("Retry request failed: {}", e))?;

            return Ok(retry_response);
        }

        Ok(response)
    }
}

#[async_trait]
impl WalletProvider for FlashWalletProvider {
    async fn sign_message(&self, message: &[u8]) -> Result<Signature, String> {
        log::debug!("Signing message via Flash control plane");

        let url = format!("{}/api/keystore/sign-message", self.keystore_url);
        let message_str = String::from_utf8_lossy(message).to_string();
        let request = SignMessageRequest { message: message_str };

        let response = self.post_with_retry(&url, &request).await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Sign message failed ({}): {}", status, body));
        }

        let data: SignMessageResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse sign response: {}", e))?;

        Self::parse_signature(&data.signature)
    }

    async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature, String> {
        log::debug!("Signing transaction via Flash control plane");

        let url = format!("{}/api/keystore/sign-transaction", self.keystore_url);

        // Extract transaction fields
        let chain_id = tx.chain_id()
            .ok_or("Transaction missing chain_id")?
            .as_u64();

        let to = tx.to()
            .ok_or("Transaction missing 'to' address")?
            .as_address()
            .ok_or("Invalid 'to' address")?;

        let value = tx.value().cloned().unwrap_or_default();
        let data = tx.data().map(|d| format!("0x{}", hex::encode(d)));

        // Extract EIP-1559 gas fields if available
        let (max_fee, priority_fee) = match tx {
            TypedTransaction::Eip1559(eip1559) => (
                eip1559.max_fee_per_gas.map(|g| g.to_string()),
                eip1559.max_priority_fee_per_gas.map(|g| g.to_string()),
            ),
            _ => (None, None),
        };

        let request = SignTransactionRequest {
            chain_id,
            to: format!("{:?}", to),
            value: value.to_string(),
            data,
            gas_limit: tx.gas().map(|g| g.to_string()),
            max_fee_per_gas: max_fee,
            max_priority_fee_per_gas: priority_fee,
            nonce: tx.nonce().map(|n| n.as_u64()),
        };

        let response = self.post_with_retry(&url, &request).await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Sign transaction failed ({}): {}", status, body));
        }

        let data: SignTransactionResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse sign response: {}", e))?;

        // Privy returns the full RLP-encoded signed transaction.
        // Extract (v, r, s) signature components from it.
        Self::extract_signature_from_signed_tx(&data.signed_transaction)
    }

    async fn sign_hash(&self, hash: H256) -> Result<Signature, String> {
        // In Flash mode, we can't sign raw hashes directly via Privy
        // Instead, we wrap the hash in a minimal typed data structure
        // This uses a simple "SignHash" type that just contains the hash
        let typed_data = serde_json::json!({
            "domain": {
                "name": "Starkbot",
                "version": "1",
                "chainId": 1
            },
            "types": {
                "EIP712Domain": [
                    {"name": "name", "type": "string"},
                    {"name": "version", "type": "string"},
                    {"name": "chainId", "type": "uint256"}
                ],
                "SignHash": [
                    {"name": "hash", "type": "bytes32"}
                ]
            },
            "primaryType": "SignHash",
            "message": {
                "hash": format!("0x{}", hex::encode(hash.as_bytes()))
            },
            "_hash": format!("0x{}", hex::encode(hash.as_bytes()))
        });

        self.sign_typed_data(&typed_data).await
    }

    async fn sign_typed_data(&self, typed_data: &serde_json::Value) -> Result<Signature, String> {
        log::debug!("Signing typed data via Flash control plane");

        let url = format!("{}/api/keystore/sign-typed-data", self.keystore_url);

        let request = SignTypedDataRequest {
            typed_data: typed_data.clone(),
        };

        let response = self.post_with_retry(&url, &request).await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Sign typed data failed ({}): {}", status, body));
        }

        let data: SignTypedDataResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse sign response: {}", e))?;

        Self::parse_signature(&data.signature)
    }

    fn get_address(&self) -> String {
        self.address.clone()
    }

    async fn refresh(&self) -> Result<(), String> {
        log::info!("Flash wallet refresh requested (no-op - wallet ID is stable)");
        Ok(())
    }

    fn mode_name(&self) -> &'static str {
        "flash"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_vars_defined() {
        // Just ensure the env var names are consistent
        assert_eq!(env_vars::FLASH_KEYSTORE_URL, "FLASH_KEYSTORE_URL");
        assert_eq!(env_vars::FLASH_TENANT_ID, "FLASH_TENANT_ID");
        assert_eq!(env_vars::FLASH_INSTANCE_TOKEN, "FLASH_INSTANCE_TOKEN");
    }

    #[test]
    fn test_parse_signature() {
        // A valid 65-byte signature in hex (130 chars)
        let sig_hex = "0x".to_string() + &"a".repeat(128) + "1b";
        let sig = FlashWalletProvider::parse_signature(&sig_hex).unwrap();
        assert_eq!(sig.v, 27); // 0x1b = 27
    }
}
