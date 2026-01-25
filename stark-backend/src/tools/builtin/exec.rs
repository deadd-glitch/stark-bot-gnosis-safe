use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Command execution tool with security restrictions
pub struct ExecTool {
    definition: ToolDefinition,
    /// Commands that are explicitly allowed (empty means use deny list only)
    allow_list: Vec<String>,
    /// Commands that are explicitly denied
    deny_list: Vec<String>,
    /// Maximum execution time in seconds
    max_timeout: u64,
}

impl ExecTool {
    pub fn new() -> Self {
        Self::with_restrictions(vec![], Self::default_deny_list(), 60)
    }

    pub fn with_restrictions(
        allow_list: Vec<String>,
        deny_list: Vec<String>,
        max_timeout: u64,
    ) -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            "command".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The command to execute".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );
        properties.insert(
            "args".to_string(),
            PropertySchema {
                schema_type: "array".to_string(),
                description: "Arguments to pass to the command".to_string(),
                default: Some(json!([])),
                items: Some(Box::new(PropertySchema {
                    schema_type: "string".to_string(),
                    description: "Command argument".to_string(),
                    default: None,
                    items: None,
                    enum_values: None,
                })),
                enum_values: None,
            },
        );
        properties.insert(
            "working_dir".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Working directory for command execution (relative to workspace)"
                    .to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );
        properties.insert(
            "timeout".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: format!(
                    "Timeout in seconds (default: 30, max: {})",
                    max_timeout
                ),
                default: Some(json!(30)),
                items: None,
                enum_values: None,
            },
        );

        ExecTool {
            definition: ToolDefinition {
                name: "exec".to_string(),
                description: "Execute a shell command. Commands are restricted for security. The command runs in the workspace directory.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["command".to_string()],
                },
                group: ToolGroup::Exec,
            },
            allow_list,
            deny_list,
            max_timeout,
        }
    }

    fn default_deny_list() -> Vec<String> {
        vec![
            // Dangerous system commands
            "rm".to_string(),
            "rmdir".to_string(),
            "dd".to_string(),
            "mkfs".to_string(),
            "fdisk".to_string(),
            "parted".to_string(),
            // Network attack tools
            "nc".to_string(),
            "netcat".to_string(),
            "nmap".to_string(),
            // Privilege escalation
            "sudo".to_string(),
            "su".to_string(),
            "doas".to_string(),
            "pkexec".to_string(),
            // Service management
            "systemctl".to_string(),
            "service".to_string(),
            "init".to_string(),
            // Package management (could install malware)
            "apt".to_string(),
            "apt-get".to_string(),
            "yum".to_string(),
            "dnf".to_string(),
            "pacman".to_string(),
            "brew".to_string(),
            // Shell spawning
            "sh".to_string(),
            "bash".to_string(),
            "zsh".to_string(),
            "fish".to_string(),
            "csh".to_string(),
            "tcsh".to_string(),
            // Dangerous file operations
            "chmod".to_string(),
            "chown".to_string(),
            "chgrp".to_string(),
            // Process manipulation
            "kill".to_string(),
            "killall".to_string(),
            "pkill".to_string(),
            // Cron/scheduling
            "crontab".to_string(),
            "at".to_string(),
            // Dangerous utilities
            "eval".to_string(),
            "exec".to_string(),
            "source".to_string(),
            // Environment manipulation
            "export".to_string(),
            "unset".to_string(),
            "env".to_string(),
        ]
    }

    fn is_command_allowed(&self, command: &str) -> Result<(), String> {
        // Extract the base command (without path)
        let base_command = command
            .split('/')
            .last()
            .unwrap_or(command)
            .split_whitespace()
            .next()
            .unwrap_or(command);

        // If allow list is non-empty, command must be in it
        if !self.allow_list.is_empty() {
            if !self.allow_list.iter().any(|c| c == base_command) {
                return Err(format!(
                    "Command '{}' is not in the allowed commands list",
                    base_command
                ));
            }
            return Ok(());
        }

        // Check deny list
        if self.deny_list.iter().any(|c| c == base_command) {
            return Err(format!("Command '{}' is not allowed for security reasons", base_command));
        }

        // Check for shell metacharacters that could be used for injection
        let dangerous_chars = ['|', ';', '&', '$', '`', '(', ')', '{', '}', '<', '>', '!', '\\'];
        if command.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(
                "Command contains shell metacharacters which are not allowed".to_string()
            );
        }

        Ok(())
    }
}

impl Default for ExecTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct ExecParams {
    command: String,
    args: Option<Vec<String>>,
    working_dir: Option<String>,
    timeout: Option<u64>,
}

#[async_trait]
impl Tool for ExecTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: ExecParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate command
        if let Err(e) = self.is_command_allowed(&params.command) {
            return ToolResult::error(e);
        }

        // Also validate args for dangerous patterns
        if let Some(ref args) = params.args {
            for arg in args {
                // Check for shell injection in arguments
                let dangerous_chars = ['|', ';', '&', '$', '`', '(', ')', '<', '>'];
                if arg.chars().any(|c| dangerous_chars.contains(&c)) {
                    return ToolResult::error(format!(
                        "Argument '{}' contains potentially dangerous characters",
                        arg
                    ));
                }
            }
        }

        let timeout_secs = params.timeout.unwrap_or(30).min(self.max_timeout);

        // Determine working directory
        let workspace = context
            .workspace_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let working_dir = if let Some(ref wd) = params.working_dir {
            let wd_path = PathBuf::from(wd);
            if wd_path.is_absolute() {
                wd_path
            } else {
                workspace.join(wd_path)
            }
        } else {
            workspace.clone()
        };

        // Verify working directory is within workspace
        let canonical_workspace = match workspace.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::error(format!("Cannot resolve workspace directory: {}", e))
            }
        };

        let canonical_working_dir = match working_dir.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::error(format!("Cannot resolve working directory: {}", e))
            }
        };

        if !canonical_working_dir.starts_with(&canonical_workspace) {
            return ToolResult::error(
                "Working directory must be within the workspace".to_string()
            );
        }

        // Find the command executable
        let command_path = match which::which(&params.command) {
            Ok(p) => p,
            Err(_) => {
                return ToolResult::error(format!("Command '{}' not found", params.command))
            }
        };

        // Build the command
        let mut cmd = Command::new(&command_path);
        cmd.current_dir(&canonical_working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Inject API keys as environment variables for CLI tools
        // GitHub CLI (gh) uses GH_TOKEN or GITHUB_TOKEN for authentication
        if let Some(github_token) = context.get_api_key("github") {
            cmd.env("GH_TOKEN", &github_token);
            cmd.env("GITHUB_TOKEN", &github_token);
        }

        if let Some(ref args) = params.args {
            cmd.args(args);
        }

        // Execute with timeout
        let start = std::time::Instant::now();
        let output = match timeout(Duration::from_secs(timeout_secs), cmd.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return ToolResult::error(format!("Failed to execute command: {}", e)),
            Err(_) => {
                return ToolResult::error(format!(
                    "Command timed out after {} seconds",
                    timeout_secs
                ))
            }
        };
        let duration_ms = start.elapsed().as_millis() as i64;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        // Build response
        let success = output.status.success();
        let mut result_text = String::new();

        if !stdout.is_empty() {
            result_text.push_str(&stdout);
        }

        if !stderr.is_empty() {
            if !result_text.is_empty() {
                result_text.push_str("\n\n--- stderr ---\n");
            }
            result_text.push_str(&stderr);
        }

        if result_text.is_empty() {
            result_text = if success {
                "Command completed successfully with no output.".to_string()
            } else {
                format!("Command failed with exit code: {}", exit_code)
            };
        }

        // Truncate if too long
        const MAX_OUTPUT: usize = 50000;
        if result_text.len() > MAX_OUTPUT {
            result_text = format!(
                "{}\n\n[Output truncated at {} characters]",
                &result_text[..MAX_OUTPUT],
                MAX_OUTPUT
            );
        }

        let result = if success {
            ToolResult::success(result_text)
        } else {
            ToolResult::error(result_text)
        };

        result.with_metadata(json!({
            "command": params.command,
            "args": params.args,
            "exit_code": exit_code,
            "duration_ms": duration_ms,
            "working_dir": canonical_working_dir.to_string_lossy()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_deny_list() {
        let tool = ExecTool::new();

        assert!(tool.is_command_allowed("rm").is_err());
        assert!(tool.is_command_allowed("sudo").is_err());
        assert!(tool.is_command_allowed("bash").is_err());

        // These should be allowed
        assert!(tool.is_command_allowed("ls").is_ok());
        assert!(tool.is_command_allowed("cat").is_ok());
        assert!(tool.is_command_allowed("echo").is_ok());
    }

    #[test]
    fn test_shell_metacharacter_detection() {
        let tool = ExecTool::new();

        assert!(tool.is_command_allowed("cat | grep").is_err());
        assert!(tool.is_command_allowed("echo; rm -rf").is_err());
        assert!(tool.is_command_allowed("$(whoami)").is_err());
        assert!(tool.is_command_allowed("echo `id`").is_err());
    }

    #[test]
    fn test_allow_list() {
        let tool = ExecTool::with_restrictions(
            vec!["git".to_string(), "npm".to_string()],
            vec![],
            60,
        );

        assert!(tool.is_command_allowed("git").is_ok());
        assert!(tool.is_command_allowed("npm").is_ok());
        assert!(tool.is_command_allowed("ls").is_err()); // Not in allow list
    }

    #[tokio::test]
    async fn test_exec_simple_command() {
        let tool = ExecTool::new();
        let context = ToolContext::new();

        let result = tool
            .execute(
                json!({
                    "command": "echo",
                    "args": ["hello", "world"]
                }),
                &context,
            )
            .await;

        assert!(result.success);
        assert!(result.content.contains("hello world"));
    }
}
