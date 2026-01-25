use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Read file tool - reads contents of files within a sandboxed directory
pub struct ReadFileTool {
    definition: ToolDefinition,
}

impl ReadFileTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            "path".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Path to the file to read (relative to workspace directory)"
                    .to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );
        properties.insert(
            "max_lines".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "Maximum number of lines to read (default: 500)".to_string(),
                default: Some(json!(500)),
                items: None,
                enum_values: None,
            },
        );
        properties.insert(
            "offset".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "Line number to start reading from (0-based, default: 0)".to_string(),
                default: Some(json!(0)),
                items: None,
                enum_values: None,
            },
        );

        ReadFileTool {
            definition: ToolDefinition {
                name: "read_file".to_string(),
                description: "Read the contents of a file. The path must be within the allowed workspace directory.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["path".to_string()],
                },
                group: ToolGroup::Filesystem,
            },
        }
    }
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct ReadFileParams {
    path: String,
    max_lines: Option<usize>,
    offset: Option<usize>,
}

#[async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: ReadFileParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let max_lines = params.max_lines.unwrap_or(500);
        let offset = params.offset.unwrap_or(0);

        // Get workspace directory from context or use current directory
        let workspace = context
            .workspace_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Resolve the path
        let requested_path = Path::new(&params.path);
        let full_path = if requested_path.is_absolute() {
            requested_path.to_path_buf()
        } else {
            workspace.join(requested_path)
        };

        // Canonicalize paths for comparison
        let canonical_workspace = match workspace.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::error(format!("Cannot resolve workspace directory: {}", e))
            }
        };

        let canonical_path = match full_path.canonicalize() {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Cannot resolve file path: {}", e)),
        };

        // Security check: ensure path is within workspace
        if !canonical_path.starts_with(&canonical_workspace) {
            return ToolResult::error(format!(
                "Access denied: path '{}' is outside the workspace directory",
                params.path
            ));
        }

        // Check if file exists and is a file
        if !canonical_path.exists() {
            return ToolResult::error(format!("File not found: {}", params.path));
        }

        if !canonical_path.is_file() {
            return ToolResult::error(format!("Path is not a file: {}", params.path));
        }

        // Read the file
        let content = match tokio::fs::read_to_string(&canonical_path).await {
            Ok(c) => c,
            Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
        };

        // Apply offset and max_lines
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        if offset >= total_lines {
            return ToolResult::success(format!(
                "[Empty: offset {} exceeds total lines {}]",
                offset, total_lines
            ))
            .with_metadata(json!({
                "path": params.path,
                "total_lines": total_lines,
                "offset": offset,
                "lines_returned": 0
            }));
        }

        let end = (offset + max_lines).min(total_lines);
        let selected_lines: Vec<String> = lines[offset..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:>5}│ {}", offset + i + 1, line))
            .collect();

        let result = selected_lines.join("\n");
        let truncated = end < total_lines;

        let output = if truncated {
            format!(
                "{}\n\n[Showing lines {}-{} of {}. Use offset parameter to read more.]",
                result,
                offset + 1,
                end,
                total_lines
            )
        } else {
            result
        };

        ToolResult::success(output).with_metadata(json!({
            "path": params.path,
            "total_lines": total_lines,
            "offset": offset,
            "lines_returned": end - offset,
            "truncated": truncated
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_outside_workspace() {
        let tool = ReadFileTool::new();
        let temp_dir = TempDir::new().unwrap();
        let context = ToolContext::new().with_workspace(temp_dir.path().to_string_lossy().to_string());

        let result = tool
            .execute(json!({ "path": "/etc/passwd" }), &context)
            .await;

        assert!(!result.success);
        assert!(result.error.unwrap().contains("outside the workspace"));
    }
}
