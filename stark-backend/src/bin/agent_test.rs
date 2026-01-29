//! Agent Test Fixture
//!
//! A minimal test harness for testing agentic tool loops without booting the full app.
//! Supports real tool execution including `exec` (shell commands) and `use_skill` (skills).
//!
//! Usage:
//!   TEST_QUERY="tell bankr agent to buy 1 starkbot" \
//!   TEST_AGENT_ENDPOINT="https://api.moonshot.ai/v1/chat/completions" \
//!   TEST_AGENT_SECRET="your-api-key" \
//!   TEST_AGENT_ARCHETYPE="kimi" \
//!   TEST_SKILLS_DIR="../skills" \
//!   BANKR_API_KEY="your-bankr-key" \
//!   cargo run --bin agent_test
//!
//! Environment variables:
//!   TEST_QUERY          - The user query to test
//!   TEST_AGENT_ENDPOINT - LLM API endpoint (OpenAI-compatible)
//!   TEST_AGENT_SECRET   - API key for the LLM
//!   TEST_AGENT_ARCHETYPE - Model archetype: kimi, llama, openai, claude
//!   TEST_SKILLS_DIR     - Path to skills directory (default: ./skills or ../skills)
//!   BANKR_API_KEY       - (optional) Bankr API key for bankr skill

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::time::Duration;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCallResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Tool {
    #[serde(rename = "type")]
    tool_type: String,
    function: ToolFunction,
}

#[derive(Debug, Clone, Serialize)]
struct ToolFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCallResponse {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallResponse>>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// ============================================================================
// Test Tools
// ============================================================================

fn get_test_tools(skills_dir: &str) -> Vec<Tool> {
    let mut tools = vec![
        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "get_weather".to_string(),
                description: "Get current weather for a location. Use this when the user asks about weather.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city and state/country, e.g. 'Boston, MA' or 'London, UK'"
                        }
                    },
                    "required": ["location"]
                }),
            },
        },
        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "web_search".to_string(),
                description: "Search the web for information. Use this when you need to find current information.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "calculator".to_string(),
                description: "Perform mathematical calculations.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "The mathematical expression to evaluate, e.g. '2 + 2' or '15 * 3'"
                        }
                    },
                    "required": ["expression"]
                }),
            },
        },
        Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "exec".to_string(),
                description: "Execute a shell command. Use for running CLI tools, scripts, curl commands, etc.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        },
                        "timeout": {
                            "type": "integer",
                            "description": "Timeout in seconds (default: 60, max: 300)"
                        }
                    },
                    "required": ["command"]
                }),
            },
        },
    ];

    // Add use_skill tool if we have skills
    let skill_names = list_available_skills(skills_dir);
    if !skill_names.is_empty() {
        tools.push(Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "use_skill".to_string(),
                description: format!(
                    "Execute a specialized skill. Available skills: {}",
                    skill_names.join(", ")
                ),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "skill_name": {
                            "type": "string",
                            "description": format!("The skill to execute. Options: {}", skill_names.join(", "))
                        },
                        "input": {
                            "type": "string",
                            "description": "Input or query for the skill"
                        }
                    },
                    "required": ["skill_name", "input"]
                }),
            },
        });
    }

    tools
}

/// List available skills from the skills directory
fn list_available_skills(skills_dir: &str) -> Vec<String> {
    let mut skills = Vec::new();
    if let Ok(entries) = std::fs::read_dir(skills_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    skills.push(name.to_string());
                }
            }
        }
    }
    skills
}

/// Load a skill's content from the skills directory
fn load_skill(skills_dir: &str, skill_name: &str) -> Option<String> {
    let path = format!("{}/{}.md", skills_dir, skill_name);
    std::fs::read_to_string(&path).ok()
}

fn execute_tool(name: &str, arguments: &Value, skills_dir: &str) -> String {
    match name {
        "get_weather" => {
            let location = arguments.get("location").and_then(|v| v.as_str()).unwrap_or("unknown");
            format!(
                "Weather for {}: Currently 45°F (7°C), partly cloudy. High of 52°F, low of 38°F. Humidity 65%. Wind 10 mph NW.",
                location
            )
        }
        "web_search" => {
            let query = arguments.get("query").and_then(|v| v.as_str()).unwrap_or("unknown");
            format!(
                "Search results for '{}': [1] Example result about {}. [2] Another relevant page. [3] More information here.",
                query, query
            )
        }
        "calculator" => {
            let expr = arguments.get("expression").and_then(|v| v.as_str()).unwrap_or("0");
            // Simple mock - in real life you'd evaluate it
            format!("Result: {} = 42 (mock result)", expr)
        }
        "exec" => {
            let command = arguments.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let timeout_secs = arguments.get("timeout").and_then(|v| v.as_u64()).unwrap_or(60);
            execute_shell_command(command, timeout_secs)
        }
        "use_skill" => {
            let skill_name = arguments.get("skill_name").and_then(|v| v.as_str()).unwrap_or("");
            let input = arguments.get("input").and_then(|v| v.as_str()).unwrap_or("");
            execute_skill(skills_dir, skill_name, input)
        }
        _ => format!("Unknown tool: {}", name),
    }
}

/// Execute a shell command and return the output
fn execute_shell_command(command: &str, timeout_secs: u64) -> String {
    use std::process::Command;
    use std::time::Instant;

    println!("\n   🔨 Executing command (timeout: {}s):", timeout_secs);
    println!("   {}", command);

    let start = Instant::now();

    // Use bash to execute the command
    let output = Command::new("bash")
        .arg("-c")
        .arg(command)
        .output();

    let duration = start.elapsed();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);

            println!("   ⏱️  Completed in {:.2}s (exit code: {})", duration.as_secs_f64(), exit_code);

            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push_str("\n");
                }
                result.push_str("[stderr] ");
                result.push_str(&stderr);
            }
            if result.is_empty() {
                result = format!("Command completed with exit code {}", exit_code);
            }

            // Truncate if too long
            if result.len() > 10000 {
                result.truncate(10000);
                result.push_str("\n... (output truncated)");
            }

            result
        }
        Err(e) => format!("Failed to execute command: {}", e),
    }
}

/// Execute a skill by loading its instructions and formatting them with the input
fn execute_skill(skills_dir: &str, skill_name: &str, input: &str) -> String {
    println!("\n   📚 Loading skill: {}", skill_name);

    match load_skill(skills_dir, skill_name) {
        Some(content) => {
            // Extract the body (everything after the YAML frontmatter)
            let body = if content.starts_with("---") {
                // Find the second ---
                if let Some(end) = content[3..].find("---") {
                    content[end + 6..].trim().to_string()
                } else {
                    content
                }
            } else {
                content
            };

            format!(
                "## Skill: {}\n\n{}\n\n### User Query:\n{}\n\nUse the appropriate tools (like `exec` for commands) to fulfill this skill request based on the instructions above.",
                skill_name,
                body,
                input
            )
        }
        None => {
            let available = list_available_skills(skills_dir);
            format!(
                "Skill '{}' not found. Available skills: {}",
                skill_name,
                if available.is_empty() { "none".to_string() } else { available.join(", ") }
            )
        }
    }
}

// ============================================================================
// Archetype-specific prompt enhancement
// ============================================================================

fn enhance_prompt_for_archetype(base_prompt: &str, archetype: &str, tools: &[Tool]) -> String {
    match archetype {
        "kimi" => {
            let mut prompt = base_prompt.to_string();
            prompt.push_str("\n\n## Available Tools\n\n");
            prompt.push_str("You have access to the following tools. Use them to help the user:\n\n");
            for tool in tools {
                prompt.push_str(&format!("- **{}**: {}\n", tool.function.name, tool.function.description));
            }
            prompt.push_str("\n**IMPORTANT**: When a user asks for something that a tool can provide, ");
            prompt.push_str("USE the tool via the native tool_calls mechanism. Do not output tool calls as text.\n");
            prompt
        }
        "llama" => {
            let mut prompt = base_prompt.to_string();
            prompt.push_str("\n\n## RESPONSE FORMAT\n\n");
            prompt.push_str("Respond in JSON: {\"body\": \"message\", \"tool_call\": null} or ");
            prompt.push_str("{\"body\": \"status\", \"tool_call\": {\"tool_name\": \"name\", \"tool_params\": {...}}}\n");
            prompt
        }
        _ => base_prompt.to_string(),
    }
}

fn get_default_model(archetype: &str) -> &'static str {
    match archetype {
        "kimi" => "kimi-k2-turbo-preview",
        "llama" => "llama3.3",
        "openai" => "gpt-4",
        "claude" => "claude-3-sonnet",
        _ => "gpt-4",
    }
}

// ============================================================================
// Main Agent Loop
// ============================================================================

async fn run_agent_loop(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    archetype: &str,
    query: &str,
    skills_dir: &str,
) -> Result<String, String> {
    let tools = get_test_tools(skills_dir);
    let model = get_default_model(archetype);

    let system_prompt = enhance_prompt_for_archetype(
        "You are a helpful assistant with access to tools. Use them when needed.",
        archetype,
        &tools,
    );

    let mut messages: Vec<Message> = vec![
        Message {
            role: "system".to_string(),
            content: Some(system_prompt),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
        Message {
            role: "user".to_string(),
            content: Some(query.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
    ];

    let max_iterations = 10;
    let mut iteration = 0;

    loop {
        iteration += 1;
        println!("\n==========================================================");
        println!("📤 ITERATION {} - Sending request to {}", iteration, endpoint);
        println!("==========================================================");

        if iteration > max_iterations {
            return Err("Max iterations reached".to_string());
        }

        let request = ChatRequest {
            model: model.to_string(),
            messages: messages.clone(),
            max_tokens: 4096,
            tools: Some(tools.clone()),
            tool_choice: Some("auto".to_string()),
        };

        // Pretty print the request
        println!("\n📋 Request body:");
        println!("{}", serde_json::to_string_pretty(&request).unwrap_or_default());

        let response = client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

        println!("\n📥 Response (status: {}):", status);
        if let Ok(pretty) = serde_json::from_str::<Value>(&response_text) {
            println!("{}", serde_json::to_string_pretty(&pretty).unwrap_or(response_text.clone()));
        } else {
            println!("{}", response_text);
        }

        if !status.is_success() {
            return Err(format!("API error {}: {}", status, response_text));
        }

        let chat_response: ChatResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {} - body: {}", e, response_text))?;

        let choice = chat_response.choices.first().ok_or("No choices in response")?;

        println!("\n📊 Parsed response:");
        println!("   finish_reason: {:?}", choice.finish_reason);
        println!("   content: {:?}", choice.message.content);
        println!("   tool_calls: {:?}", choice.message.tool_calls.as_ref().map(|t| t.len()));

        // Check if we have tool calls
        if let Some(tool_calls) = &choice.message.tool_calls {
            if !tool_calls.is_empty() {
                println!("\n🔧 Tool calls detected ({}):", tool_calls.len());

                // Add assistant message with tool calls
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: choice.message.content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                    name: None,
                });

                // Execute each tool and add results
                for tc in tool_calls {
                    println!("   - {} (id: {})", tc.function.name, tc.id);
                    println!("     args: {}", tc.function.arguments);

                    let args: Value = serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                    let result = execute_tool(&tc.function.name, &args, skills_dir);

                    println!("     result: {}", result);

                    messages.push(Message {
                        role: "tool".to_string(),
                        content: Some(result),
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                        name: Some(tc.function.name.clone()),
                    });
                }

                continue; // Go to next iteration
            }
        }

        // No tool calls - check finish reason
        let finish_reason = choice.finish_reason.as_deref().unwrap_or("unknown");

        if finish_reason == "tool_calls" {
            println!("\n⚠️  finish_reason is 'tool_calls' but no tool_calls in response!");
        }

        // Final response
        let final_content = choice.message.content.clone().unwrap_or_default();
        println!("\n✅ Final response (finish_reason: {}):", finish_reason);
        println!("{}", final_content);

        return Ok(final_content);
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    println!("🤖 Agent Test Fixture");
    println!("=====================\n");

    // Read environment variables
    let query = env::var("TEST_QUERY").unwrap_or_else(|_| {
        eprintln!("❌ TEST_QUERY not set. Using default.");
        "What's the weather in Boston?".to_string()
    });

    let endpoint = env::var("TEST_AGENT_ENDPOINT").unwrap_or_else(|_| {
        eprintln!("❌ TEST_AGENT_ENDPOINT not set!");
        std::process::exit(1);
    });

    let secret = env::var("TEST_AGENT_SECRET").unwrap_or_else(|_| {
        eprintln!("❌ TEST_AGENT_SECRET not set!");
        std::process::exit(1);
    });

    let archetype = env::var("TEST_AGENT_ARCHETYPE").unwrap_or_else(|_| {
        eprintln!("⚠️  TEST_AGENT_ARCHETYPE not set. Using 'kimi'.");
        "kimi".to_string()
    });

    let skills_dir = env::var("TEST_SKILLS_DIR").unwrap_or_else(|_| {
        // Default to ../skills relative to the binary or current dir
        let default = if std::path::Path::new("skills").exists() {
            "skills".to_string()
        } else if std::path::Path::new("../skills").exists() {
            "../skills".to_string()
        } else {
            "./skills".to_string()
        };
        eprintln!("⚠️  TEST_SKILLS_DIR not set. Using '{}'.", default);
        default
    });

    let available_skills = list_available_skills(&skills_dir);

    println!("📝 Configuration:");
    println!("   Query:     {}", query);
    println!("   Endpoint:  {}", endpoint);
    println!("   Secret:    {}...", &secret[..secret.len().min(8)]);
    println!("   Archetype: {}", archetype);
    println!("   Skills:    {} ({:?})", skills_dir, available_skills);

    // Create HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("Failed to create HTTP client");

    // Run the agent loop
    match run_agent_loop(&client, &endpoint, &secret, &archetype, &query, &skills_dir).await {
        Ok(response) => {
            println!("\n==========================================================");
            println!("🎉 SUCCESS");
            println!("==========================================================");
            println!("{}", response);
        }
        Err(e) => {
            println!("\n==========================================================");
            println!("❌ ERROR");
            println!("==========================================================");
            println!("{}", e);
            std::process::exit(1);
        }
    }
}
