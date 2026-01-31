//! Built-in hooks for common functionality
//!
//! This module provides hooks that are commonly needed:
//! - Logging - Event recording and debugging
//! - Rate limiting - Request throttling and abuse prevention
//! - Auto-memory - Ephemeral memory creation for tool activity

mod auto_memory_hook;
mod logging_hook;
mod rate_limit_hook;

pub use auto_memory_hook::{AutoMemoryConfig, AutoMemoryHook, TrackedTools};
pub use logging_hook::{LogLevel, LoggingHook};
pub use rate_limit_hook::{RateLimitConfig, RateLimitHook};
