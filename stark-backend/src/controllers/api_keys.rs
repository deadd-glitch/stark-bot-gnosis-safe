use actix_web::{web, HttpRequest, HttpResponse, Responder};
use ethers::signers::{LocalWallet, Signer};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter, EnumString, IntoEnumIterator};

use crate::keystore_client::KEYSTORE_CLIENT;
use crate::models::ApiKeyResponse;
use crate::AppState;

/// Derive wallet address from private key
fn get_wallet_address(private_key: &str) -> Option<String> {
    let wallet: LocalWallet = private_key.parse().ok()?;
    Some(format!("{:?}", wallet.address()))
}

/// Enum of all valid API key identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumString, AsRefStr)]
pub enum ApiKeyId {
    #[strum(serialize = "GITHUB_TOKEN")]
    GithubToken,
    #[strum(serialize = "BANKR_API_KEY")]
    BankrApiKey,
    #[strum(serialize = "MOLTX_API_KEY")]
    MoltxApiKey,
    #[strum(serialize = "DISCORD_BOT_TOKEN")]
    DiscordBotToken,
    #[strum(serialize = "TELEGRAM_BOT_TOKEN")]
    TelegramBotToken,
    #[strum(serialize = "SLACK_BOT_TOKEN")]
    SlackBotToken,
    #[strum(serialize = "MOLTBOOK_TOKEN")]
    MoltbookToken,
    #[strum(serialize = "FOURCLAW_TOKEN")]
    FourclawToken,
    #[strum(serialize = "X402BOOK_TOKEN")]
    X402bookToken,
    #[strum(serialize = "TWITTER_CONSUMER_KEY")]
    TwitterConsumerKey,
    #[strum(serialize = "TWITTER_CONSUMER_SECRET")]
    TwitterConsumerSecret,
    #[strum(serialize = "TWITTER_ACCESS_TOKEN")]
    TwitterAccessToken,
    #[strum(serialize = "TWITTER_ACCESS_TOKEN_SECRET")]
    TwitterAccessTokenSecret,
}

impl ApiKeyId {
    /// The key name as stored in the database
    pub fn as_str(&self) -> &'static str {
        // AsRefStr from strum provides static string references
        match self {
            Self::GithubToken => "GITHUB_TOKEN",
            Self::BankrApiKey => "BANKR_API_KEY",
            Self::MoltxApiKey => "MOLTX_API_KEY",
            Self::DiscordBotToken => "DISCORD_BOT_TOKEN",
            Self::TelegramBotToken => "TELEGRAM_BOT_TOKEN",
            Self::SlackBotToken => "SLACK_BOT_TOKEN",
            Self::MoltbookToken => "MOLTBOOK_TOKEN",
            Self::FourclawToken => "FOURCLAW_TOKEN",
            Self::X402bookToken => "X402BOOK_TOKEN",
            Self::TwitterConsumerKey => "TWITTER_CONSUMER_KEY",
            Self::TwitterConsumerSecret => "TWITTER_CONSUMER_SECRET",
            Self::TwitterAccessToken => "TWITTER_ACCESS_TOKEN",
            Self::TwitterAccessTokenSecret => "TWITTER_ACCESS_TOKEN_SECRET",
        }
    }

    /// Environment variable names to set when this key is available
    pub fn env_vars(&self) -> Option<&'static [&'static str]> {
        match self {
            Self::GithubToken => Some(&["GH_TOKEN", "GITHUB_TOKEN"]),
            Self::BankrApiKey => Some(&["BANKR_API_KEY"]),
            Self::MoltxApiKey => Some(&["MOLTX_API_KEY"]),
            Self::DiscordBotToken => Some(&["DISCORD_BOT_TOKEN", "DISCORD_TOKEN"]),
            Self::TelegramBotToken => Some(&["TELEGRAM_BOT_TOKEN", "TELEGRAM_TOKEN"]),
            Self::SlackBotToken => Some(&["SLACK_BOT_TOKEN", "SLACK_TOKEN"]),
            Self::MoltbookToken => Some(&["MOLTBOOK_TOKEN"]),
            Self::FourclawToken => Some(&["FOURCLAW_TOKEN"]),
            Self::X402bookToken => Some(&["X402BOOK_TOKEN"]),
            Self::TwitterConsumerKey => Some(&["TWITTER_CONSUMER_KEY", "TWITTER_API_KEY"]),
            Self::TwitterConsumerSecret => Some(&["TWITTER_CONSUMER_SECRET", "TWITTER_API_SECRET"]),
            Self::TwitterAccessToken => Some(&["TWITTER_ACCESS_TOKEN"]),
            Self::TwitterAccessTokenSecret => Some(&["TWITTER_ACCESS_TOKEN_SECRET"]),
        }
    }

    /// Whether this key requires special git configuration when set
    pub fn requires_git_config(&self) -> bool {
        matches!(self, Self::GithubToken)
    }

    /// Iterate over all API key variants
    pub fn iter() -> impl Iterator<Item = ApiKeyId> {
        <Self as IntoEnumIterator>::iter()
    }

    /// Get all variants as a slice (for backwards compatibility)
    pub fn all() -> Vec<ApiKeyId> {
        Self::iter().collect()
    }

    /// Get all key names as strings
    pub fn all_names() -> Vec<&'static str> {
        Self::iter().map(|k| k.as_str()).collect()
    }
}

/// Configuration for a single key within a service group
#[derive(Debug, Clone, Serialize)]
pub struct KeyConfig {
    pub name: &'static str,
    pub label: &'static str,
    pub secret: bool,
}

/// Configuration for a service group (e.g., "github" groups GITHUB_TOKEN)
#[derive(Debug, Clone, Serialize)]
pub struct ServiceConfig {
    pub group: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub url: &'static str,
    pub keys: Vec<KeyConfig>,
}

/// Get all service configurations
pub fn get_service_configs() -> Vec<ServiceConfig> {
    vec![
        ServiceConfig {
            group: "github",
            label: "GitHub",
            description: "Create a Personal Access Token with repo scope",
            url: "https://github.com/settings/tokens",
            keys: vec![KeyConfig {
                name: "GITHUB_TOKEN",
                label: "Personal Access Token",
                secret: true,
            }],
        },
        ServiceConfig {
            group: "moltx",
            label: "MoltX",
            description: "X for agents. Get an API key from moltx.io after registering your agent.",
            url: "https://moltx.io",
            keys: vec![KeyConfig {
                name: "MOLTX_API_KEY",
                label: "API Key",
                secret: true,
            }],
        },
        ServiceConfig {
            group: "bankr",
            label: "Bankr",
            description: "Generate an API key with Agent API access enabled",
            url: "https://bankr.bot/api",
            keys: vec![KeyConfig {
                name: "BANKR_API_KEY",
                label: "API Key",
                secret: true,
            }],
        },
        ServiceConfig {
            group: "discord",
            label: "Discord",
            description: "Create a Bot application and copy its token",
            url: "https://discord.com/developers/applications",
            keys: vec![KeyConfig {
                name: "DISCORD_BOT_TOKEN",
                label: "Bot Token",
                secret: true,
            }],
        },
        ServiceConfig {
            group: "telegram",
            label: "Telegram",
            description: "Create a bot via @BotFather and copy the token",
            url: "https://t.me/BotFather",
            keys: vec![KeyConfig {
                name: "TELEGRAM_BOT_TOKEN",
                label: "Bot Token",
                secret: true,
            }],
        },
        ServiceConfig {
            group: "slack",
            label: "Slack",
            description: "Create a Slack App and copy the Bot User OAuth Token",
            url: "https://api.slack.com/apps",
            keys: vec![KeyConfig {
                name: "SLACK_BOT_TOKEN",
                label: "Bot User OAuth Token",
                secret: true,
            }],
        },
        ServiceConfig {
            group: "moltbook",
            label: "Moltbook",
            description: "Social network for AI agents. Register via API or get token from moltbook.com",
            url: "https://www.moltbook.com",
            keys: vec![KeyConfig {
                name: "MOLTBOOK_TOKEN",
                label: "API Token",
                secret: true,
            }],
        },
        ServiceConfig {
            group: "4claw",
            label: "4claw",
            description: "4claw network for AI agents. Get your API token from 4claw.org",
            url: "https://4claw.org",
            keys: vec![KeyConfig {
                name: "FOURCLAW_TOKEN",
                label: "API Token",
                secret: true,
            }],
        },
        ServiceConfig {
            group: "x402book",
            label: "x402book",
            description: "x402book network for AI agents. Get your API token from x402book.com",
            url: "https://api.x402book.com",
            keys: vec![KeyConfig {
                name: "X402BOOK_TOKEN",
                label: "API Token",
                secret: true,
            }],
        },
        ServiceConfig {
            group: "twitter",
            label: "Twitter/X",
            description: "OAuth 1.0a credentials for posting tweets. Get all 4 keys from your Twitter Developer App's 'Keys and Tokens' tab.",
            url: "https://developer.twitter.com/en/portal/projects-and-apps",
            keys: vec![
                KeyConfig {
                    name: "TWITTER_CONSUMER_KEY",
                    label: "API Key (Consumer Key)",
                    secret: true,
                },
                KeyConfig {
                    name: "TWITTER_CONSUMER_SECRET",
                    label: "API Secret (Consumer Secret)",
                    secret: true,
                },
                KeyConfig {
                    name: "TWITTER_ACCESS_TOKEN",
                    label: "Access Token",
                    secret: true,
                },
                KeyConfig {
                    name: "TWITTER_ACCESS_TOKEN_SECRET",
                    label: "Access Token Secret",
                    secret: true,
                },
            ],
        },
    ]
}

/// Get all valid key names
pub fn get_valid_key_names() -> Vec<&'static str> {
    ApiKeyId::all().iter().map(|k| k.as_str()).collect()
}

/// Get key config by key name
pub fn get_key_config(key_name: &str) -> Option<(&'static str, KeyConfig)> {
    for config in get_service_configs() {
        for key in &config.keys {
            if key.name == key_name {
                return Some((config.group, KeyConfig {
                    name: key.name,
                    label: key.label,
                    secret: key.secret,
                }));
            }
        }
    }
    None
}

#[derive(Debug, Deserialize)]
pub struct UpsertApiKeyRequest {
    pub key_name: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteApiKeyRequest {
    pub key_name: String,
}

#[derive(Serialize)]
pub struct ApiKeysListResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<ApiKeyResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ApiKeyOperationResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<ApiKeyResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for service configs endpoint
#[derive(Serialize)]
pub struct ServiceConfigsResponse {
    pub success: bool,
    pub configs: Vec<ServiceConfig>,
}

/// Key data for backup/restore (internal use only)
#[derive(Serialize, Deserialize)]
struct BackupKey {
    key_name: String,
    key_value: String,
}

/// Response for backup/restore operations
#[derive(Serialize)]
pub struct BackupResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Key preview for cloud keys preview
#[derive(Serialize)]
pub struct CloudKeyPreview {
    pub key_name: String,
    pub key_preview: String,
}

/// Response for preview cloud keys
#[derive(Serialize)]
pub struct PreviewKeysResponse {
    pub success: bool,
    pub key_count: usize,
    pub keys: Vec<CloudKeyPreview>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request/response for keystore API
#[derive(Serialize, Deserialize)]
struct KeystoreBackupRequest {
    wallet_id: String,
    encrypted_data: String,
    key_count: usize,
    timestamp: i64,
    signature: String,
}

#[derive(Deserialize)]
struct KeystoreBackupResponse {
    encrypted_data: String,
}

/// Sign a message with the burner wallet private key
fn sign_message(private_key: &str, message: &str) -> Result<String, String> {
    use ethers::signers::{LocalWallet, Signer};

    let wallet: LocalWallet = private_key
        .parse()
        .map_err(|e| format!("Invalid private key: {}", e))?;

    // Sign synchronously using the blocking runtime
    let signature = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            wallet.sign_message(message).await
        })
    }).map_err(|e| format!("Failed to sign message: {}", e))?;

    Ok(format!("0x{}", hex::encode(signature.to_vec())))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/keys")
            .route("", web::get().to(list_api_keys))
            .route("", web::post().to(upsert_api_key))
            .route("", web::delete().to(delete_api_key))
            .route("/config", web::get().to(get_configs))
            .route("/cloud_backup", web::post().to(backup_to_cloud))
            .route("/cloud_restore", web::post().to(restore_from_cloud))
            .route("/cloud_preview", web::get().to(preview_cloud_keys)),
    );
}

async fn get_configs(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    HttpResponse::Ok().json(ServiceConfigsResponse {
        success: true,
        configs: get_service_configs(),
    })
}

fn validate_session_from_request(
    state: &web::Data<AppState>,
    req: &HttpRequest,
) -> Result<(), HttpResponse> {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string());

    let token = match token {
        Some(t) => t,
        None => {
            return Err(HttpResponse::Unauthorized().json(ApiKeysListResponse {
                success: false,
                keys: None,
                error: Some("No authorization token provided".to_string()),
            }));
        }
    };

    match state.db.validate_session(&token) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpResponse::Unauthorized().json(ApiKeysListResponse {
            success: false,
            keys: None,
            error: Some("Invalid or expired session".to_string()),
        })),
        Err(e) => {
            log::error!("Failed to validate session: {}", e);
            Err(HttpResponse::InternalServerError().json(ApiKeysListResponse {
                success: false,
                keys: None,
                error: Some("Internal server error".to_string()),
            }))
        }
    }
}

async fn list_api_keys(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    match state.db.list_api_keys() {
        Ok(keys) => {
            let key_responses: Vec<ApiKeyResponse> = keys
                .into_iter()
                .map(|k| k.to_response())
                .collect();
            HttpResponse::Ok().json(ApiKeysListResponse {
                success: true,
                keys: Some(key_responses),
                error: None,
            })
        }
        Err(e) => {
            log::error!("Failed to list API keys: {}", e);
            HttpResponse::InternalServerError().json(ApiKeysListResponse {
                success: false,
                keys: None,
                error: Some("Failed to retrieve API keys".to_string()),
            })
        }
    }
}

async fn upsert_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<UpsertApiKeyRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    // Validate key name
    let valid_keys = get_valid_key_names();
    if !valid_keys.contains(&body.key_name.as_str()) {
        return HttpResponse::BadRequest().json(ApiKeyOperationResponse {
            success: false,
            key: None,
            error: Some(format!(
                "Invalid key name. Valid options: {}",
                valid_keys.join(", ")
            )),
        });
    }

    // Validate api_key is not empty
    if body.api_key.trim().is_empty() {
        return HttpResponse::BadRequest().json(ApiKeyOperationResponse {
            success: false,
            key: None,
            error: Some("API key cannot be empty".to_string()),
        });
    }

    // Store the key (key_name is the service_name in the database)
    match state.db.upsert_api_key(&body.key_name, &body.api_key) {
        Ok(key) => HttpResponse::Ok().json(ApiKeyOperationResponse {
            success: true,
            key: Some(key.to_response()),
            error: None,
        }),
        Err(e) => {
            log::error!("Failed to save API key: {}", e);
            HttpResponse::InternalServerError().json(ApiKeyOperationResponse {
                success: false,
                key: None,
                error: Some("Failed to save API key".to_string()),
            })
        }
    }
}

async fn delete_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<DeleteApiKeyRequest>,
) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    match state.db.delete_api_key(&body.key_name) {
        Ok(deleted) => {
            if deleted {
                HttpResponse::Ok().json(ApiKeyOperationResponse {
                    success: true,
                    key: None,
                    error: None,
                })
            } else {
                HttpResponse::NotFound().json(ApiKeyOperationResponse {
                    success: false,
                    key: None,
                    error: Some("API key not found".to_string()),
                })
            }
        }
        Err(e) => {
            log::error!("Failed to delete API key: {}", e);
            HttpResponse::InternalServerError().json(ApiKeyOperationResponse {
                success: false,
                key: None,
                error: Some("Failed to delete API key".to_string()),
            })
        }
    }
}

/// Backup API keys to cloud (encrypted with burner wallet key)
async fn backup_to_cloud(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    // Get burner wallet private key from config
    let private_key = match &state.config.burner_wallet_private_key {
        Some(pk) => pk.clone(),
        None => {
            return HttpResponse::BadRequest().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some("Burner wallet not configured".to_string()),
            });
        }
    };

    // Get all keys with values
    let keys = match state.db.list_api_keys_with_values() {
        Ok(k) => k,
        Err(e) => {
            log::error!("Failed to list API keys: {}", e);
            return HttpResponse::InternalServerError().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some("Failed to export keys".to_string()),
            });
        }
    };

    if keys.is_empty() {
        return HttpResponse::BadRequest().json(BackupResponse {
            success: false,
            key_count: None,
            message: None,
            error: Some("No API keys to backup".to_string()),
        });
    }

    // Serialize keys to JSON
    let keys_for_backup: Vec<BackupKey> = keys
        .iter()
        .map(|(name, value)| BackupKey {
            key_name: name.clone(),
            key_value: value.clone(),
        })
        .collect();
    let keys_json = match serde_json::to_string(&keys_for_backup) {
        Ok(j) => j,
        Err(e) => {
            log::error!("Failed to serialize keys: {}", e);
            return HttpResponse::InternalServerError().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some("Failed to serialize keys".to_string()),
            });
        }
    };

    // Encrypt with ECIES using the burner wallet's public key
    let encrypted_data = match encrypt_with_private_key(&private_key, &keys_json) {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to encrypt keys: {}", e);
            return HttpResponse::InternalServerError().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some("Failed to encrypt keys".to_string()),
            });
        }
    };

    // Upload to keystore API (with SIWE authentication)
    match KEYSTORE_CLIENT.store_keys(&private_key, &encrypted_data, keys.len()).await {
        Ok(resp) if resp.success => {
            HttpResponse::Ok().json(BackupResponse {
                success: true,
                key_count: Some(keys.len()),
                message: Some(format!("Backed up {} keys to cloud", keys.len())),
                error: None,
            })
        }
        Ok(resp) => {
            log::error!("Keystore store_keys failed: {:?}", resp.error);
            HttpResponse::BadGateway().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: resp.error.or(Some("Failed to upload to keystore".to_string())),
            })
        }
        Err(e) => {
            log::error!("Failed to connect to keystore: {}", e);
            HttpResponse::BadGateway().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some(format!("Keystore error: {}", e)),
            })
        }
    }
}

/// Restore API keys from cloud backup
async fn restore_from_cloud(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    // Get burner wallet private key from config
    let private_key = match &state.config.burner_wallet_private_key {
        Some(pk) => pk.clone(),
        None => {
            return HttpResponse::BadRequest().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some("Burner wallet not configured".to_string()),
            });
        }
    };

    // Fetch from keystore API (with SIWE authentication)
    let keystore_resp = match KEYSTORE_CLIENT.get_keys(&private_key).await {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("Failed to connect to keystore: {}", e);
            return HttpResponse::BadGateway().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some(format!("Keystore error: {}", e)),
            });
        }
    };

    if !keystore_resp.success {
        let error = keystore_resp.error.unwrap_or_else(|| "Unknown error".to_string());
        if error.contains("No backup found") {
            return HttpResponse::NotFound().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some(error),
            });
        }
        return HttpResponse::BadGateway().json(BackupResponse {
            success: false,
            key_count: None,
            message: None,
            error: Some(error),
        });
    }

    let encrypted_data = match keystore_resp.encrypted_data {
        Some(data) => data,
        None => {
            return HttpResponse::BadGateway().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some("No encrypted data in response".to_string()),
            });
        }
    };

    // Decrypt with ECIES using the burner wallet's private key
    let decrypted_json = match decrypt_with_private_key(&private_key, &encrypted_data) {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to decrypt backup: {}", e);
            return HttpResponse::BadRequest().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some("Failed to decrypt backup (wrong wallet?)".to_string()),
            });
        }
    };

    // Parse the decrypted keys
    let restored_keys: Vec<BackupKey> = match serde_json::from_str(&decrypted_json) {
        Ok(keys) => keys,
        Err(e) => {
            log::error!("Failed to parse decrypted keys: {}", e);
            return HttpResponse::BadRequest().json(BackupResponse {
                success: false,
                key_count: None,
                message: None,
                error: Some("Invalid backup data format".to_string()),
            });
        }
    };

    // Restore each key to the database
    let mut restored_count = 0;
    for key in &restored_keys {
        // Only restore valid key names
        if get_valid_key_names().contains(&key.key_name.as_str()) {
            if let Err(e) = state.db.upsert_api_key(&key.key_name, &key.key_value) {
                log::error!("Failed to restore key {}: {}", key.key_name, e);
            } else {
                restored_count += 1;
            }
        }
    }

    HttpResponse::Ok().json(BackupResponse {
        success: true,
        key_count: Some(restored_count),
        message: Some(format!("Restored {} keys from backup", restored_count)),
        error: None,
    })
}

/// Encrypt data using ECIES with the public key derived from private key
fn encrypt_with_private_key(private_key: &str, data: &str) -> Result<String, String> {
    use ecies::{encrypt, PublicKey, SecretKey};

    // Parse private key (remove 0x prefix if present)
    let pk_hex = private_key.trim_start_matches("0x");
    let pk_bytes = hex::decode(pk_hex).map_err(|e| format!("Invalid private key hex: {}", e))?;

    // Create secret key and derive public key
    let secret_key = SecretKey::parse_slice(&pk_bytes)
        .map_err(|e| format!("Invalid private key: {:?}", e))?;
    let public_key = PublicKey::from_secret_key(&secret_key);

    // Encrypt the data
    let encrypted = encrypt(&public_key.serialize(), data.as_bytes())
        .map_err(|e| format!("Encryption failed: {:?}", e))?;

    Ok(hex::encode(encrypted))
}

/// Decrypt data using ECIES with the private key
fn decrypt_with_private_key(private_key: &str, encrypted_hex: &str) -> Result<String, String> {
    use ecies::{decrypt, SecretKey};

    // Parse private key (remove 0x prefix if present)
    let pk_hex = private_key.trim_start_matches("0x");
    let pk_bytes = hex::decode(pk_hex).map_err(|e| format!("Invalid private key hex: {}", e))?;

    // Parse encrypted data
    let encrypted = hex::decode(encrypted_hex).map_err(|e| format!("Invalid encrypted data: {}", e))?;

    // Create secret key
    let secret_key = SecretKey::parse_slice(&pk_bytes)
        .map_err(|e| format!("Invalid private key: {:?}", e))?;

    // Decrypt the data
    let decrypted = decrypt(&secret_key.serialize(), &encrypted)
        .map_err(|e| format!("Decryption failed: {:?}", e))?;

    String::from_utf8(decrypted).map_err(|e| format!("Invalid UTF-8 in decrypted data: {}", e))
}

/// Create a preview string from an API key value (e.g., "sk-abc...xyz")
fn create_key_preview(value: &str) -> String {
    if value.len() <= 8 {
        "*".repeat(value.len())
    } else {
        format!("{}...{}", &value[..4], &value[value.len()-4..])
    }
}

/// Preview API keys from cloud backup (without restoring)
async fn preview_cloud_keys(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Err(resp) = validate_session_from_request(&state, &req) {
        return resp;
    }

    // Get burner wallet private key from config
    let private_key = match &state.config.burner_wallet_private_key {
        Some(pk) => pk.clone(),
        None => {
            return HttpResponse::BadRequest().json(PreviewKeysResponse {
                success: false,
                key_count: 0,
                keys: vec![],
                message: None,
                error: Some("Burner wallet not configured".to_string()),
            });
        }
    };

    // Fetch from keystore API (with SIWE authentication)
    let keystore_resp = match KEYSTORE_CLIENT.get_keys(&private_key).await {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("Failed to connect to keystore: {}", e);
            return HttpResponse::BadGateway().json(PreviewKeysResponse {
                success: false,
                key_count: 0,
                keys: vec![],
                message: None,
                error: Some(format!("Keystore error: {}", e)),
            });
        }
    };

    if !keystore_resp.success {
        let error = keystore_resp.error.unwrap_or_else(|| "Unknown error".to_string());
        if error.contains("No backup found") {
            return HttpResponse::NotFound().json(PreviewKeysResponse {
                success: false,
                key_count: 0,
                keys: vec![],
                message: None,
                error: Some(error),
            });
        }
        return HttpResponse::BadGateway().json(PreviewKeysResponse {
            success: false,
            key_count: 0,
            keys: vec![],
            message: None,
            error: Some(error),
        });
    }

    let encrypted_data = match keystore_resp.encrypted_data {
        Some(data) => data,
        None => {
            return HttpResponse::BadGateway().json(PreviewKeysResponse {
                success: false,
                key_count: 0,
                keys: vec![],
                message: None,
                error: Some("No encrypted data in response".to_string()),
            });
        }
    };

    // Decrypt with ECIES using the burner wallet's private key
    let decrypted_json = match decrypt_with_private_key(&private_key, &encrypted_data) {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to decrypt backup: {}", e);
            return HttpResponse::BadRequest().json(PreviewKeysResponse {
                success: false,
                key_count: 0,
                keys: vec![],
                message: None,
                error: Some("Failed to decrypt backup (wrong wallet?)".to_string()),
            });
        }
    };

    // Parse the decrypted keys
    let cloud_keys: Vec<BackupKey> = match serde_json::from_str(&decrypted_json) {
        Ok(keys) => keys,
        Err(e) => {
            log::error!("Failed to parse decrypted keys: {}", e);
            return HttpResponse::BadRequest().json(PreviewKeysResponse {
                success: false,
                key_count: 0,
                keys: vec![],
                message: None,
                error: Some("Invalid backup data format".to_string()),
            });
        }
    };

    // Convert to previews (only valid key names, with masked values)
    let valid_keys = get_valid_key_names();
    let previews: Vec<CloudKeyPreview> = cloud_keys
        .iter()
        .filter(|k| valid_keys.contains(&k.key_name.as_str()))
        .map(|k| CloudKeyPreview {
            key_name: k.key_name.clone(),
            key_preview: create_key_preview(&k.key_value),
        })
        .collect();

    HttpResponse::Ok().json(PreviewKeysResponse {
        success: true,
        key_count: previews.len(),
        keys: previews,
        message: Some("Cloud keys retrieved successfully".to_string()),
        error: None,
    })
}
