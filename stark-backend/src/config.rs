use std::env;

/// Environment variable names - single source of truth
pub mod env_vars {
    pub const LOGIN_ADMIN_PUBLIC_ADDRESS: &str = "LOGIN_ADMIN_PUBLIC_ADDRESS";
    pub const BURNER_WALLET_PRIVATE_KEY: &str = "BURNER_WALLET_BOT_PRIVATE_KEY";
    pub const PORT: &str = "PORT";
    pub const DATABASE_URL: &str = "DATABASE_URL";
    pub const WORKSPACE_DIR: &str = "STARK_WORKSPACE_DIR";
    pub const SKILLS_DIR: &str = "STARK_SKILLS_DIR";
    pub const JOURNAL_DIR: &str = "STARK_JOURNAL_DIR";
    // Memory configuration
    pub const MEMORY_ENABLE_PRE_COMPACTION_FLUSH: &str = "STARK_MEMORY_ENABLE_PRE_COMPACTION_FLUSH";
    pub const MEMORY_ENABLE_ENTITY_EXTRACTION: &str = "STARK_MEMORY_ENABLE_ENTITY_EXTRACTION";
    pub const MEMORY_ENABLE_VECTOR_SEARCH: &str = "STARK_MEMORY_ENABLE_VECTOR_SEARCH";
    pub const MEMORY_EMBEDDING_PROVIDER: &str = "STARK_MEMORY_EMBEDDING_PROVIDER";
    pub const MEMORY_ENABLE_AUTO_CONSOLIDATION: &str = "STARK_MEMORY_ENABLE_AUTO_CONSOLIDATION";
    pub const MEMORY_ENABLE_CROSS_SESSION: &str = "STARK_MEMORY_ENABLE_CROSS_SESSION";
    pub const MEMORY_CROSS_SESSION_LIMIT: &str = "STARK_MEMORY_CROSS_SESSION_LIMIT";
}

/// Default values
pub mod defaults {
    pub const PORT: u16 = 8080;
    pub const DATABASE_URL: &str = "./.db/stark.db";
    pub const WORKSPACE_DIR: &str = "./workspace";
    pub const SKILLS_DIR: &str = "./skills";
    pub const JOURNAL_DIR: &str = "./journal";
}

/// Get the workspace directory from environment or default
pub fn workspace_dir() -> String {
    env::var(env_vars::WORKSPACE_DIR).unwrap_or_else(|_| defaults::WORKSPACE_DIR.to_string())
}

/// Get the skills directory from environment or default
pub fn skills_dir() -> String {
    env::var(env_vars::SKILLS_DIR).unwrap_or_else(|_| defaults::SKILLS_DIR.to_string())
}

/// Get the journal directory from environment or default
pub fn journal_dir() -> String {
    env::var(env_vars::JOURNAL_DIR).unwrap_or_else(|_| defaults::JOURNAL_DIR.to_string())
}

/// Get the burner wallet private key from environment (for tools)
pub fn burner_wallet_private_key() -> Option<String> {
    env::var(env_vars::BURNER_WALLET_PRIVATE_KEY).ok()
}

#[derive(Clone)]
pub struct Config {
    pub login_admin_public_address: String,
    pub burner_wallet_private_key: Option<String>,
    pub port: u16,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            login_admin_public_address: env::var(env_vars::LOGIN_ADMIN_PUBLIC_ADDRESS)
                .expect("LOGIN_ADMIN_PUBLIC_ADDRESS must be set"),
            burner_wallet_private_key: env::var(env_vars::BURNER_WALLET_PRIVATE_KEY).ok(),
            port: env::var(env_vars::PORT)
                .unwrap_or_else(|_| defaults::PORT.to_string())
                .parse()
                .expect("PORT must be a valid number"),
            database_url: env::var(env_vars::DATABASE_URL)
                .unwrap_or_else(|_| defaults::DATABASE_URL.to_string()),
        }
    }
}

/// Configuration for memory system features
#[derive(Clone, Debug)]
pub struct MemoryConfig {
    /// Enable pre-compaction memory flush (AI extracts memories before summarization)
    pub enable_pre_compaction_flush: bool,
    /// Enable entity extraction from conversations
    pub enable_entity_extraction: bool,
    /// Enable vector search (requires embedding provider)
    pub enable_vector_search: bool,
    /// Embedding provider: "openai", "local", or "none"
    pub embedding_provider: String,
    /// Enable automatic memory consolidation
    pub enable_auto_consolidation: bool,
    /// Enable cross-session memory sharing (same identity across channels)
    pub enable_cross_session_memory: bool,
    /// Maximum number of cross-session memories to include
    pub cross_session_memory_limit: i32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enable_pre_compaction_flush: true,
            enable_entity_extraction: true,
            enable_vector_search: false,
            embedding_provider: "none".to_string(),
            enable_auto_consolidation: false,
            enable_cross_session_memory: true,
            cross_session_memory_limit: 5,
        }
    }
}

impl MemoryConfig {
    pub fn from_env() -> Self {
        Self {
            enable_pre_compaction_flush: env::var(env_vars::MEMORY_ENABLE_PRE_COMPACTION_FLUSH)
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            enable_entity_extraction: env::var(env_vars::MEMORY_ENABLE_ENTITY_EXTRACTION)
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            enable_vector_search: env::var(env_vars::MEMORY_ENABLE_VECTOR_SEARCH)
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            embedding_provider: env::var(env_vars::MEMORY_EMBEDDING_PROVIDER)
                .unwrap_or_else(|_| "none".to_string()),
            enable_auto_consolidation: env::var(env_vars::MEMORY_ENABLE_AUTO_CONSOLIDATION)
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            enable_cross_session_memory: env::var(env_vars::MEMORY_ENABLE_CROSS_SESSION)
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
            cross_session_memory_limit: env::var(env_vars::MEMORY_CROSS_SESSION_LIMIT)
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
        }
    }
}

/// Get the memory configuration
pub fn memory_config() -> MemoryConfig {
    MemoryConfig::from_env()
}
