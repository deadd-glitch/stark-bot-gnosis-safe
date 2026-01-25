//! Execution tracking module
//!
//! This module provides real-time execution progress tracking for AI agent tasks.
//! It manages a hierarchical task tree and emits gateway events for frontend
//! display of execution progress (similar to Claude Code's CLI display).

mod tracker;

pub use tracker::ExecutionTracker;
