use crate::ai::{
    AiClient, ArchetypeId, ArchetypeRegistry, Message, MessageRole, ModelArchetype,
    ThinkingLevel, ToolCall, ToolHistoryEntry, ToolResponse,
};
use crate::channels::types::{DispatchResult, NormalizedMessage};
use crate::config::MemoryConfig;
use crate::context::{self, estimate_tokens, ContextManager};
use crate::db::Database;
use crate::execution::ExecutionTracker;
use crate::gateway::events::EventBroadcaster;
use crate::gateway::protocol::GatewayEvent;
use crate::models::{MemoryType, SessionScope};
use crate::models::session_message::MessageRole as DbMessageRole;
use crate::tools::{ToolConfig, ToolContext, ToolDefinition, ToolExecution, ToolRegistry};
use chrono::Utc;
use regex::Regex;
use serde_json::Value;
use std::sync::Arc;

/// Maximum number of tool execution iterations
/// Set to 25 to allow for async jobs that require polling (e.g., Bankr API)
const MAX_TOOL_ITERATIONS: usize = 25;

/// Configuration for a memory marker pattern
struct MemoryMarkerConfig {
    pattern: Regex,
    memory_type: MemoryType,
    importance: i32,
    name: &'static str,
    use_today_date: bool,
}

impl MemoryMarkerConfig {
    fn new(
        pattern: &str,
        memory_type: MemoryType,
        importance: i32,
        name: &'static str,
        use_today_date: bool,
    ) -> Self {
        Self {
            pattern: Regex::new(pattern).unwrap(),
            memory_type,
            importance,
            name,
            use_today_date,
        }
    }
}

/// Create all memory marker configurations
fn create_memory_markers() -> Vec<MemoryMarkerConfig> {
    vec![
        MemoryMarkerConfig::new(
            r"\[DAILY_LOG:\s*(.+?)\]",
            MemoryType::DailyLog,
            5,
            "daily log",
            true,
        ),
        MemoryMarkerConfig::new(
            r"\[REMEMBER:\s*(.+?)\]",
            MemoryType::LongTerm,
            7,
            "long-term memory",
            false,
        ),
        MemoryMarkerConfig::new(
            r"\[REMEMBER_IMPORTANT:\s*(.+?)\]",
            MemoryType::LongTerm,
            9,
            "important memory",
            false,
        ),
        // Phase 2: New memory types
        MemoryMarkerConfig::new(
            r"\[PREFERENCE:\s*(.+?)\]",
            MemoryType::Preference,
            7,
            "user preference",
            false,
        ),
        MemoryMarkerConfig::new(
            r"\[FACT:\s*(.+?)\]",
            MemoryType::Fact,
            7,
            "user fact",
            false,
        ),
        MemoryMarkerConfig::new(
            r"\[TASK:\s*(.+?)\]",
            MemoryType::Task,
            8,
            "task/commitment",
            true, // Use today's date for tasks
        ),
    ]
}

/// Dispatcher routes messages to the AI and returns responses
pub struct MessageDispatcher {
    db: Arc<Database>,
    broadcaster: Arc<EventBroadcaster>,
    tool_registry: Arc<ToolRegistry>,
    execution_tracker: Arc<ExecutionTracker>,
    burner_wallet_private_key: Option<String>,
    context_manager: ContextManager,
    archetype_registry: ArchetypeRegistry,
    /// Memory marker configurations
    memory_markers: Vec<MemoryMarkerConfig>,
    /// Regex pattern for thinking directives
    thinking_directive_pattern: Regex,
    /// Memory configuration for cross-session and other features
    memory_config: MemoryConfig,
}

impl MessageDispatcher {
    pub fn new(
        db: Arc<Database>,
        broadcaster: Arc<EventBroadcaster>,
        tool_registry: Arc<ToolRegistry>,
        execution_tracker: Arc<ExecutionTracker>,
    ) -> Self {
        Self::new_with_wallet(db, broadcaster, tool_registry, execution_tracker, None)
    }

    pub fn new_with_wallet(
        db: Arc<Database>,
        broadcaster: Arc<EventBroadcaster>,
        tool_registry: Arc<ToolRegistry>,
        execution_tracker: Arc<ExecutionTracker>,
        burner_wallet_private_key: Option<String>,
    ) -> Self {
        let memory_config = MemoryConfig::from_env();
        let context_manager = ContextManager::new(db.clone())
            .with_memory_config(memory_config.clone());
        Self {
            db,
            broadcaster,
            tool_registry,
            execution_tracker,
            burner_wallet_private_key,
            context_manager,
            archetype_registry: ArchetypeRegistry::new(),
            memory_markers: create_memory_markers(),
            thinking_directive_pattern: Regex::new(r"(?i)^/(?:t|think|thinking)(?::(\w+))?$").unwrap(),
            memory_config,
        }
    }

    /// Create a dispatcher without tool support (for backwards compatibility)
    pub fn new_without_tools(db: Arc<Database>, broadcaster: Arc<EventBroadcaster>) -> Self {
        // Create a minimal execution tracker for legacy use
        let execution_tracker = Arc::new(ExecutionTracker::new(broadcaster.clone()));
        let memory_config = MemoryConfig::from_env();
        let context_manager = ContextManager::new(db.clone())
            .with_memory_config(memory_config.clone());
        Self {
            db: db.clone(),
            broadcaster,
            tool_registry: Arc::new(ToolRegistry::new()),
            execution_tracker,
            burner_wallet_private_key: None,
            context_manager,
            archetype_registry: ArchetypeRegistry::new(),
            memory_markers: create_memory_markers(),
            thinking_directive_pattern: Regex::new(r"(?i)^/(?:t|think|thinking)(?::(\w+))?$").unwrap(),
            memory_config,
        }
    }

    /// Dispatch a normalized message to the AI and return the response
    pub async fn dispatch(&self, message: NormalizedMessage) -> DispatchResult {
        // Emit message received event
        self.broadcaster.broadcast(GatewayEvent::channel_message(
            message.channel_id,
            &message.channel_type,
            &message.user_name,
            &message.text,
        ));

        // Check for reset commands
        let text_lower = message.text.trim().to_lowercase();
        if text_lower == "/new" || text_lower == "/reset" {
            return self.handle_reset_command(&message).await;
        }

        // Check for thinking directives (session-level setting)
        if let Some(thinking_response) = self.handle_thinking_directive(&message).await {
            return thinking_response;
        }

        // Parse inline thinking directive and extract clean message
        let (thinking_level, clean_text) = self.parse_inline_thinking(&message.text);

        // Start execution tracking with user message for descriptive display
        let user_msg = clean_text.as_deref().unwrap_or(&message.text);
        let execution_id = self.execution_tracker.start_execution(
            message.channel_id,
            "execute",
            Some(user_msg),
        );

        // Get or create identity for the user
        let identity = match self.db.get_or_create_identity(
            &message.channel_type,
            &message.user_id,
            Some(&message.user_name),
        ) {
            Ok(id) => id,
            Err(e) => {
                log::error!("Failed to get/create identity: {}", e);
                self.execution_tracker.complete_execution(message.channel_id);
                return DispatchResult::error(format!("Identity error: {}", e));
            }
        };

        // Determine session scope (group if chat_id != user_id, otherwise dm)
        let scope = if message.chat_id != message.user_id {
            SessionScope::Group
        } else {
            SessionScope::Dm
        };

        // Get or create chat session
        let session = match self.db.get_or_create_chat_session(
            &message.channel_type,
            message.channel_id,
            &message.chat_id,
            scope,
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to get/create session: {}", e);
                self.execution_tracker.complete_execution(message.channel_id);
                return DispatchResult::error(format!("Session error: {}", e));
            }
        };

        // Use clean text (with inline thinking directive removed) for storage
        let message_text = clean_text.as_deref().unwrap_or(&message.text);

        // Estimate tokens for the user message
        let user_tokens = estimate_tokens(message_text);

        // Store user message in session with token count
        if let Err(e) = self.db.add_session_message(
            session.id,
            DbMessageRole::User,
            message_text,
            Some(&message.user_id),
            Some(&message.user_name),
            message.message_id.as_deref(),
            Some(user_tokens),
        ) {
            log::error!("Failed to store user message: {}", e);
        } else {
            // Update context tokens
            self.context_manager.update_context_tokens(session.id, user_tokens);
        }

        // Get active agent settings from database
        let settings = match self.db.get_active_agent_settings() {
            Ok(Some(settings)) => settings,
            Ok(None) => {
                let error = "No AI provider configured. Please configure agent settings.".to_string();
                log::error!("{}", error);
                self.execution_tracker.complete_execution(message.channel_id);
                return DispatchResult::error(error);
            }
            Err(e) => {
                let error = format!("Database error: {}", e);
                log::error!("{}", error);
                self.execution_tracker.complete_execution(message.channel_id);
                return DispatchResult::error(error);
            }
        };

        // Infer archetype from settings
        let archetype_id = AiClient::infer_archetype(&settings);
        log::info!(
            "Using endpoint {} for message dispatch (archetype={}, max_tokens={})",
            settings.endpoint,
            archetype_id,
            settings.max_tokens
        );

        // Create AI client from settings with x402 wallet support
        let client = match AiClient::from_settings_with_wallet(
            &settings,
            self.burner_wallet_private_key.as_deref(),
        ) {
            Ok(c) => c,
            Err(e) => {
                let error = format!("Failed to create AI client: {}", e);
                log::error!("{}", error);
                self.execution_tracker.complete_execution(message.channel_id);
                return DispatchResult::error(error);
            }
        };

        // Add thinking event before AI generation
        self.execution_tracker.add_thinking(message.channel_id, "Processing request...");

        // Get tool configuration for this channel (needed for system prompt)
        let tool_config = self.db.get_effective_tool_config(Some(message.channel_id))
            .unwrap_or_default();

        // Debug: Log tool configuration
        log::info!(
            "[DISPATCH] Tool config - profile: {:?}, allowed_groups: {:?}",
            tool_config.profile,
            tool_config.allowed_groups
        );

        // Build context from memories, tools, skills, and session history
        let system_prompt = self.build_system_prompt(&message, &identity.identity_id, &tool_config);

        // Debug: Log full system prompt
        log::debug!("[DISPATCH] System prompt:\n{}", system_prompt);

        // Get recent session messages for conversation context
        let history = self.db.get_recent_session_messages(session.id, 20).unwrap_or_default();

        // Build messages for the AI
        let mut messages = vec![Message {
            role: MessageRole::System,
            content: system_prompt.clone(),
        }];

        // Add compaction summary if available (provides context from earlier in conversation)
        if let Some(compaction_summary) = self.context_manager.get_compaction_summary(session.id) {
            messages.push(Message {
                role: MessageRole::System,
                content: format!("## Previous Conversation Summary\n{}", compaction_summary),
            });
        }

        // Add conversation history (skip the last one since it's the current message)
        for msg in history.iter().take(history.len().saturating_sub(1)) {
            let role = match msg.role {
                DbMessageRole::User => MessageRole::User,
                DbMessageRole::Assistant => MessageRole::Assistant,
                DbMessageRole::System => MessageRole::System,
            };
            messages.push(Message {
                role,
                content: msg.content.clone(),
            });
        }

        // Add current user message (use clean text without thinking directive)
        messages.push(Message {
            role: MessageRole::User,
            content: message_text.to_string(),
        });

        // Debug: Log user message
        log::info!("[DISPATCH] User message: {}", message_text);

        // Apply thinking level if set (for Claude models)
        if let Some(level) = thinking_level {
            if client.supports_thinking() {
                log::info!("[DISPATCH] Applying thinking level: {}", level);
                client.set_thinking_level(level);
            }
        }

        // Check if the client supports tools and tools are configured
        let use_tools = client.supports_tools() && !self.tool_registry.is_empty();

        // Debug: Log tool availability
        log::info!(
            "[DISPATCH] Tool support - client_supports: {}, registry_count: {}, use_tools: {}",
            client.supports_tools(),
            self.tool_registry.len(),
            use_tools
        );

        // Build tool context with API keys from database
        let workspace_dir = crate::config::workspace_dir();

        let mut tool_context = ToolContext::new()
            .with_channel(message.channel_id, message.channel_type.clone())
            .with_user(message.user_id.clone())
            .with_workspace(workspace_dir.clone())
            .with_broadcaster(self.broadcaster.clone());

        // Ensure workspace directory exists
        let _ = std::fs::create_dir_all(&workspace_dir);

        // Load API keys from database for tools that need them
        // Each key is stored individually (e.g., "GITHUB_TOKEN", "TWITTER_CLIENT_ID")
        if let Ok(keys) = self.db.list_api_keys() {
            for key in keys {
                tool_context = tool_context.with_api_key(&key.service_name, key.api_key.clone());
            }
        }

        // Load bot config from bot_settings for git commits etc.
        if let Ok(bot_settings) = self.db.get_bot_settings() {
            tool_context = tool_context.with_bot_config(bot_settings.bot_name, bot_settings.bot_email);
        }

        // Generate response with optional tool execution loop
        let final_response = if use_tools {
            self.generate_with_tool_loop(
                &client,
                messages,
                &tool_config,
                &tool_context,
                &identity.identity_id,
                session.id,
                &message,
                archetype_id,
            ).await
        } else {
            // Simple generation without tools - with x402 event emission
            match client.generate_text_with_events(messages, &self.broadcaster, message.channel_id).await {
                Ok((content, payment)) => {
                    // Save x402 payment if one was made
                    if let Some(ref payment_info) = payment {
                        if let Err(e) = self.db.record_x402_payment(
                            Some(message.channel_id),
                            None,
                            payment_info.resource.as_deref(),
                            &payment_info.amount,
                            &payment_info.amount_formatted,
                            &payment_info.asset,
                            &payment_info.pay_to,
                        ) {
                            log::error!("[DISPATCH] Failed to record x402 payment: {}", e);
                        }
                    }
                    Ok(content)
                }
                Err(e) => Err(e),
            }
        };

        match final_response {
            Ok(response) => {
                // Parse and create memories from the response
                self.process_memory_markers(
                    &response,
                    &identity.identity_id,
                    session.id,
                    &message.channel_type,
                    message.message_id.as_deref(),
                );

                // Clean response by removing memory markers before storing/returning
                let clean_response = self.clean_response(&response);

                // Estimate tokens for the response
                let response_tokens = estimate_tokens(&clean_response);

                // Store AI response in session with token count
                if let Err(e) = self.db.add_session_message(
                    session.id,
                    DbMessageRole::Assistant,
                    &clean_response,
                    None,
                    None,
                    None,
                    Some(response_tokens),
                ) {
                    log::error!("Failed to store AI response: {}", e);
                } else {
                    // Update context tokens
                    self.context_manager.update_context_tokens(session.id, response_tokens);

                    // Check if compaction is needed
                    if self.context_manager.needs_compaction(session.id) {
                        log::info!("[COMPACTION] Context limit reached for session {}, triggering compaction", session.id);
                        if let Err(e) = self.context_manager.compact_session(
                            session.id,
                            &client,
                            Some(&identity.identity_id),
                        ).await {
                            log::error!("[COMPACTION] Failed to compact session: {}", e);
                        }
                    }
                }

                // Emit response event
                self.broadcaster.broadcast(GatewayEvent::agent_response(
                    message.channel_id,
                    &message.user_name,
                    &clean_response,
                ));

                log::info!(
                    "Generated response for {} on channel {} using {} archetype",
                    message.user_name,
                    message.channel_id,
                    archetype_id
                );

                // Complete execution tracking
                self.execution_tracker.complete_execution(message.channel_id);

                DispatchResult::success(clean_response)
            }
            Err(e) => {
                let error = format!("AI generation error ({}): {}", archetype_id, e);
                log::error!("{}", error);

                // Complete execution tracking on error
                self.execution_tracker.complete_execution(message.channel_id);

                DispatchResult::error(error)
            }
        }
    }

    /// Generate a response with tool execution loop (supports both native and text-based tool calling)
    async fn generate_with_tool_loop(
        &self,
        client: &AiClient,
        messages: Vec<Message>,
        tool_config: &ToolConfig,
        tool_context: &ToolContext,
        _identity_id: &str,
        _session_id: i64,
        original_message: &NormalizedMessage,
        archetype_id: ArchetypeId,
    ) -> Result<String, String> {
        let mut tools = self.tool_registry.get_tool_definitions(tool_config);

        // Add skills as a "use_skill" pseudo-tool if any are enabled
        if let Some(skill_tool) = self.create_skill_tool_definition() {
            tools.push(skill_tool);
        }

        // Debug: Log available tools
        log::info!(
            "[TOOL_LOOP] Available tools ({}): {:?}",
            tools.len(),
            tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );

        if tools.is_empty() {
            log::warn!("[TOOL_LOOP] No tools available, falling back to text-only generation");
            let (content, payment) = client.generate_text_with_events(messages, &self.broadcaster, original_message.channel_id).await?;
            // Save x402 payment if one was made
            if let Some(ref payment_info) = payment {
                if let Err(e) = self.db.record_x402_payment(
                    Some(original_message.channel_id),
                    None,
                    payment_info.resource.as_deref(),
                    &payment_info.amount,
                    &payment_info.amount_formatted,
                    &payment_info.asset,
                    &payment_info.pay_to,
                ) {
                    log::error!("[TOOL_LOOP] Failed to record x402 payment: {}", e);
                }
            }
            return Ok(content);
        }

        // Get the archetype for this request
        let archetype = self.archetype_registry.get(archetype_id)
            .unwrap_or_else(|| self.archetype_registry.default_archetype());

        log::info!(
            "[TOOL_LOOP] Using archetype: {} (native_tool_calling: {})",
            archetype.id(),
            archetype.uses_native_tool_calling()
        );

        // Branch based on archetype type
        if archetype.uses_native_tool_calling() {
            self.generate_with_native_tools(client, messages, tools, tool_config, tool_context, original_message, archetype).await
        } else {
            self.generate_with_text_tools(client, messages, tools, tool_config, tool_context, original_message, archetype).await
        }
    }

    /// Create a "use_skill" tool definition if skills are enabled
    fn create_skill_tool_definition(&self) -> Option<ToolDefinition> {
        use crate::tools::{PropertySchema, ToolGroup, ToolInputSchema};

        let skills = self.db.list_enabled_skills().ok()?;
        let active_skills: Vec<_> = skills.iter().filter(|s| s.enabled).collect();

        if active_skills.is_empty() {
            return None;
        }

        let skill_names: Vec<String> = active_skills.iter().map(|s| s.name.clone()).collect();

        let mut properties = std::collections::HashMap::new();
        properties.insert(
            "skill_name".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: format!("The skill to execute. Options: {}", skill_names.join(", ")),
                default: None,
                items: None,
                enum_values: Some(skill_names),
            },
        );
        properties.insert(
            "input".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Input or query for the skill".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        // Format skill descriptions with newlines for better readability
        let formatted_skills = active_skills
            .iter()
            .map(|s| format!("  - {}: {}", s.name, s.description))
            .collect::<Vec<_>>()
            .join("\n");

        Some(ToolDefinition {
            name: "use_skill".to_string(),
            description: format!(
                "Execute a specialized skill. YOU MUST use this tool when a user asks for something that matches a skill.\n\nAvailable skills:\n{}",
                formatted_skills
            ),
            input_schema: ToolInputSchema {
                schema_type: "object".to_string(),
                properties,
                required: vec!["skill_name".to_string(), "input".to_string()],
            },
            group: ToolGroup::System,
        })
    }

    /// Generate response using native API tool calling (Kimi, OpenAI, Claude)
    async fn generate_with_native_tools(
        &self,
        client: &AiClient,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        tool_config: &ToolConfig,
        tool_context: &ToolContext,
        original_message: &NormalizedMessage,
        archetype: &dyn ModelArchetype,
    ) -> Result<String, String> {
        // Enhance system prompt with tool awareness (even for native tool calling)
        let mut conversation = messages.clone();
        if let Some(system_msg) = conversation.first_mut() {
            if system_msg.role == MessageRole::System {
                system_msg.content = archetype.enhance_system_prompt(&system_msg.content, &tools);
            }
        }

        let mut tool_history: Vec<ToolHistoryEntry> = Vec::new();
        let mut iterations = 0;
        // Track tool calls to include in the response
        let mut tool_call_log: Vec<String> = Vec::new();

        loop {
            iterations += 1;
            log::info!("[NATIVE_TOOL_LOOP] Iteration {} starting", iterations);

            if iterations > MAX_TOOL_ITERATIONS {
                log::warn!("Native tool loop exceeded max iterations ({})", MAX_TOOL_ITERATIONS);
                break;
            }

            // Generate with native tool support
            let ai_response = client.generate_with_tools(
                conversation.clone(),
                tool_history.clone(),
                tools.clone(),
            ).await?;

            log::info!(
                "[NATIVE_TOOL_LOOP] Response - content_len: {}, tool_calls: {}, stop_reason: {:?}",
                ai_response.content.len(),
                ai_response.tool_calls.len(),
                ai_response.stop_reason
            );

            // Emit x402 payment event and save to database if payment was made
            if let Some(ref payment_info) = ai_response.x402_payment {
                log::info!("[NATIVE_TOOL_LOOP] x402 payment made: {} {}", payment_info.amount_formatted, payment_info.asset);
                self.broadcaster.broadcast(GatewayEvent::x402_payment(
                    original_message.channel_id,
                    &payment_info.amount,
                    &payment_info.amount_formatted,
                    &payment_info.asset,
                    &payment_info.pay_to,
                    payment_info.resource.as_deref(),
                ));
                // Save payment to database for history tracking
                if let Err(e) = self.db.record_x402_payment(
                    Some(original_message.channel_id),
                    None, // tool_name - this is an AI call, not a tool
                    payment_info.resource.as_deref(),
                    &payment_info.amount,
                    &payment_info.amount_formatted,
                    &payment_info.asset,
                    &payment_info.pay_to,
                ) {
                    log::error!("[NATIVE_TOOL_LOOP] Failed to record x402 payment: {}", e);
                }
            }

            // If no tool calls, return the content with tool call log prepended
            if ai_response.tool_calls.is_empty() {
                if tool_call_log.is_empty() {
                    return Ok(ai_response.content);
                } else {
                    // Prepend tool call log to the final response
                    let tool_log_text = tool_call_log.join("\n");
                    return Ok(format!("{}\n\n{}", tool_log_text, ai_response.content));
                }
            }

            // Log and broadcast each tool call for real-time display in chat
            for call in &ai_response.tool_calls {
                let args_pretty = serde_json::to_string_pretty(&call.arguments)
                    .unwrap_or_else(|_| call.arguments.to_string());
                tool_call_log.push(format!(
                    "🔧 **Tool Call:** `{}`\n```json\n{}\n```",
                    call.name,
                    args_pretty
                ));
                // Emit real-time event for frontend to display immediately
                self.broadcaster.broadcast(GatewayEvent::agent_tool_call(
                    original_message.channel_id,
                    &call.name,
                    &call.arguments,
                ));
            }

            // Execute tool calls
            let tool_responses = self.execute_tool_calls(
                &ai_response.tool_calls,
                tool_config,
                tool_context,
                original_message.channel_id,
                0,
                &original_message.user_id,
            ).await;

            // Add to tool history for next iteration
            tool_history.push(ToolHistoryEntry::new(
                ai_response.tool_calls,
                tool_responses,
            ));
        }

        // If we hit max iterations, return whatever content we have with tool log
        if tool_call_log.is_empty() {
            Err(format!(
                "Tool loop hit max iterations ({}) without final response. This may happen with long-running async jobs.",
                MAX_TOOL_ITERATIONS
            ))
        } else {
            let tool_log_text = tool_call_log.join("\n");
            Err(format!(
                "Tool loop hit max iterations ({}). Last tool calls:\n{}",
                MAX_TOOL_ITERATIONS,
                tool_log_text
            ))
        }
    }

    /// Generate response using text-based JSON tool calling (Llama archetype)
    async fn generate_with_text_tools(
        &self,
        client: &AiClient,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        tool_config: &ToolConfig,
        tool_context: &ToolContext,
        original_message: &NormalizedMessage,
        archetype: &dyn ModelArchetype,
    ) -> Result<String, String> {
        // Enhance system prompt with tool-calling instructions
        let mut conversation = messages.clone();
        if let Some(system_msg) = conversation.first_mut() {
            if system_msg.role == MessageRole::System {
                system_msg.content = archetype.enhance_system_prompt(&system_msg.content, &tools);
            }
        }

        let mut final_response = String::new();
        let mut iterations = 0;
        // Track tool calls to include in the response
        let mut tool_call_log: Vec<String> = Vec::new();

        loop {
            iterations += 1;
            log::info!("[TEXT_TOOL_LOOP] Iteration {} starting", iterations);

            if iterations > MAX_TOOL_ITERATIONS {
                log::warn!("Text-based tool loop exceeded max iterations ({})", MAX_TOOL_ITERATIONS);
                break;
            }

            // Generate response (text-only) - with x402 events
            let (ai_content, payment) = client.generate_text_with_events(
                conversation.clone(),
                &self.broadcaster,
                original_message.channel_id,
            ).await?;

            // Save x402 payment if one was made
            if let Some(ref payment_info) = payment {
                if let Err(e) = self.db.record_x402_payment(
                    Some(original_message.channel_id),
                    None,
                    payment_info.resource.as_deref(),
                    &payment_info.amount,
                    &payment_info.amount_formatted,
                    &payment_info.asset,
                    &payment_info.pay_to,
                ) {
                    log::error!("[TEXT_TOOL_LOOP] Failed to record x402 payment: {}", e);
                }
            }

            log::info!("[TEXT_TOOL_LOOP] Raw AI response: {}", ai_content);

            // Parse response using archetype's parser
            let parsed = archetype.parse_response(&ai_content);

            match parsed {
                Some(agent_response) => {
                    log::info!(
                        "[TEXT_TOOL_LOOP] Parsed response - body_len: {}, has_tool_call: {}",
                        agent_response.body.len(),
                        agent_response.tool_call.is_some()
                    );

                    // Check if there's a tool call
                    if let Some(tool_call) = agent_response.tool_call {
                        log::info!(
                            "[TEXT_TOOL_LOOP] Text-based tool call: {} with params: {}",
                            tool_call.tool_name,
                            tool_call.tool_params
                        );

                        // Log and broadcast tool call for real-time display in chat
                        let args_pretty = serde_json::to_string_pretty(&tool_call.tool_params)
                            .unwrap_or_else(|_| tool_call.tool_params.to_string());
                        tool_call_log.push(format!(
                            "🔧 **Tool Call:** `{}`\n```json\n{}\n```",
                            tool_call.tool_name,
                            args_pretty
                        ));
                        // Emit real-time event for frontend to display immediately
                        self.broadcaster.broadcast(GatewayEvent::agent_tool_call(
                            original_message.channel_id,
                            &tool_call.tool_name,
                            &tool_call.tool_params,
                        ));

                        // Handle special "use_skill" tool
                        let tool_result = if tool_call.tool_name == "use_skill" {
                            self.execute_skill_tool(&tool_call.tool_params).await
                        } else {
                            // Execute regular tool
                            self.tool_registry.execute(
                                &tool_call.tool_name,
                                tool_call.tool_params.clone(),
                                tool_context,
                                Some(tool_config),
                            ).await
                        };

                        log::info!("[TEXT_TOOL_LOOP] Tool result success: {}", tool_result.success);
                        log::debug!("[TEXT_TOOL_LOOP] Tool result content: {}", tool_result.content);

                        // Broadcast tool execution event with result content
                        let _ = self.broadcaster.broadcast(GatewayEvent::tool_result(
                            original_message.channel_id,
                            &tool_call.tool_name,
                            tool_result.success,
                            0, // duration_ms - not tracked for text-based tool calls
                            &tool_result.content,
                        ));

                        // Add the assistant's response and tool result to conversation
                        conversation.push(Message {
                            role: MessageRole::Assistant,
                            content: ai_content.clone(),
                        });

                        // Add tool result as user message for next iteration
                        let followup_prompt = archetype.format_tool_followup(
                            &tool_call.tool_name,
                            &tool_result.content,
                            tool_result.success,
                        );
                        conversation.push(Message {
                            role: MessageRole::User,
                            content: followup_prompt,
                        });

                        // Continue loop to get final response
                        continue;
                    } else {
                        // No tool call, this is the final response
                        // Prepend tool call log if any tools were called
                        if tool_call_log.is_empty() {
                            final_response = agent_response.body;
                        } else {
                            let tool_log_text = tool_call_log.join("\n");
                            final_response = format!("{}\n\n{}", tool_log_text, agent_response.body);
                        }
                        break;
                    }
                }
                None => {
                    // Couldn't parse as structured response, return raw content
                    log::warn!("[TEXT_TOOL_LOOP] Could not parse response, returning raw content");
                    if tool_call_log.is_empty() {
                        final_response = ai_content;
                    } else {
                        let tool_log_text = tool_call_log.join("\n");
                        final_response = format!("{}\n\n{}", tool_log_text, ai_content);
                    }
                    break;
                }
            }
        }

        if final_response.is_empty() {
            return Err("AI returned empty response".to_string());
        }

        Ok(final_response)
    }

    /// Execute the special "use_skill" tool
    async fn execute_skill_tool(&self, params: &Value) -> crate::tools::ToolResult {
        let skill_name = params.get("skill_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let input = params.get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        log::info!("[SKILL] Executing skill '{}' with input: {}", skill_name, input);

        // Look up the specific skill by name (more efficient than loading all skills)
        let skill = match self.db.get_enabled_skill_by_name(skill_name) {
            Ok(s) => s,
            Err(e) => {
                return crate::tools::ToolResult::error(format!("Failed to load skill: {}", e));
            }
        };

        match skill {
            Some(skill) => {
                // Determine the skills directory path
                let skills_dir = crate::config::skills_dir();
                let skill_base_dir = format!("{}/{}", skills_dir, skill.name);

                // Return the skill's instructions/body along with context
                let mut result = format!("## Skill: {}\n\n", skill.name);
                result.push_str(&format!("Description: {}\n\n", skill.description));

                if !skill.body.is_empty() {
                    // Replace {baseDir} placeholder with actual skill directory
                    let body_with_paths = skill.body.replace("{baseDir}", &skill_base_dir);
                    result.push_str("### Instructions:\n");
                    result.push_str(&body_with_paths);
                    result.push_str("\n\n");
                }

                result.push_str(&format!("### User Query:\n{}\n\n", input));
                result.push_str("Use the appropriate tools (like `exec` for commands) to fulfill this skill request based on the instructions above.");

                crate::tools::ToolResult::success(&result)
            }
            None => {
                // Fetch available skills for the error message
                let available = self.db.list_enabled_skills()
                    .map(|skills| skills.iter().map(|s| s.name.clone()).collect::<Vec<_>>().join(", "))
                    .unwrap_or_else(|_| "unknown".to_string());
                crate::tools::ToolResult::error(format!(
                    "Skill '{}' not found or not enabled. Available skills: {}",
                    skill_name,
                    available
                ))
            }
        }
    }

    /// Execute a list of tool calls and return responses (for native tool calling)
    async fn execute_tool_calls(
        &self,
        tool_calls: &[ToolCall],
        tool_config: &ToolConfig,
        tool_context: &ToolContext,
        channel_id: i64,
        _session_id: i64,
        _user_id: &str,
    ) -> Vec<ToolResponse> {
        let mut responses = Vec::new();

        // Get the current execution ID for tracking
        let execution_id = self.execution_tracker.get_execution_id(channel_id);

        for call in tool_calls {
            let start = std::time::Instant::now();

            // Start tracking this tool execution with context from arguments
            let task_id = if let Some(ref exec_id) = execution_id {
                Some(self.execution_tracker.start_tool(channel_id, exec_id, &call.name, &call.arguments))
            } else {
                None
            };

            // Emit tool execution event (legacy event for backwards compatibility)
            self.broadcaster.broadcast(GatewayEvent::tool_execution(
                channel_id,
                &call.name,
                &call.arguments,
            ));

            // Execute the tool (handle special "use_skill" pseudo-tool)
            let result = if call.name == "use_skill" {
                self.execute_skill_tool(&call.arguments).await
            } else {
                self.tool_registry
                    .execute(&call.name, call.arguments.clone(), tool_context, Some(tool_config))
                    .await
            };

            let duration_ms = start.elapsed().as_millis() as i64;

            // Complete the tool tracking
            if let Some(ref tid) = task_id {
                if result.success {
                    self.execution_tracker.complete_task(tid);
                } else {
                    self.execution_tracker.complete_task_with_error(tid, &result.content);
                }
            }

            // Emit tool result event with content for UI display
            self.broadcaster.broadcast(GatewayEvent::tool_result(
                channel_id,
                &call.name,
                result.success,
                duration_ms,
                &result.content,
            ));

            // Log the execution
            if let Err(e) = self.db.log_tool_execution(&ToolExecution {
                id: None,
                channel_id,
                tool_name: call.name.clone(),
                parameters: call.arguments.clone(),
                success: result.success,
                result: Some(result.content.clone()),
                duration_ms: Some(duration_ms),
                executed_at: Utc::now().to_rfc3339(),
            }) {
                log::error!("Failed to log tool execution: {}", e);
            }

            log::info!(
                "Tool '{}' executed in {}ms, success: {}",
                call.name,
                duration_ms,
                result.success
            );

            // Create tool response
            responses.push(if result.success {
                ToolResponse::success(call.id.clone(), result.content)
            } else {
                ToolResponse::error(call.id.clone(), result.content)
            });
        }

        responses
    }

    /// Load SOUL.md content if it exists
    fn load_soul() -> Option<String> {
        // Try multiple locations for SOUL.md
        let paths = [
            "SOUL.md",
            "./SOUL.md",
            "/app/SOUL.md",
        ];

        for path in paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                log::debug!("[SOUL] Loaded from {}", path);
                return Some(content);
            }
        }

        log::debug!("[SOUL] No SOUL.md found, using default personality");
        None
    }

    /// Build the base system prompt with context from memories and user info
    /// Note: Tool-related instructions are added by the archetype's enhance_system_prompt
    fn build_system_prompt(
        &self,
        message: &NormalizedMessage,
        identity_id: &str,
        _tool_config: &ToolConfig,
    ) -> String {
        let mut prompt = String::new();

        // Load SOUL.md if available, otherwise use default intro
        if let Some(soul) = Self::load_soul() {
            prompt.push_str(&soul);
            prompt.push_str("\n\n");
        } else {
            prompt.push_str("You are StarkBot, an AI agent who can respond to users and operate tools.\n\n");
        }

        // Add daily logs context
        if let Ok(daily_logs) = self.db.get_todays_daily_logs(Some(identity_id)) {
            if !daily_logs.is_empty() {
                prompt.push_str("## Today's Notes\n");
                for log in daily_logs {
                    prompt.push_str(&format!("- {}\n", log.content));
                }
                prompt.push('\n');
            }
        }

        // Add relevant long-term memories
        if let Ok(memories) = self.db.get_long_term_memories(Some(identity_id), Some(5), 10) {
            if !memories.is_empty() {
                prompt.push_str("## User Context\n");
                for mem in memories {
                    prompt.push_str(&format!("- {}\n", mem.content));
                }
                prompt.push('\n');
            }
        }

        // Add recent session summaries (past conversations)
        if let Ok(summaries) = self.db.get_session_summaries(Some(identity_id), 3) {
            if !summaries.is_empty() {
                prompt.push_str("## Previous Sessions\n");
                for summary in summaries {
                    prompt.push_str(&format!("{}\n\n", summary.content));
                }
            }
        }

        // Phase 6: Add cross-session memories from other channels
        if self.memory_config.enable_cross_session_memory {
            if let Ok(cross_memories) = self.db.get_cross_channel_memories(
                identity_id,
                Some(&message.channel_type),
                self.memory_config.cross_session_memory_limit,
            ) {
                if !cross_memories.is_empty() {
                    prompt.push_str("## Context from Other Channels\n");
                    for mem in cross_memories {
                        let channel_label = mem.source_channel_type
                            .as_deref()
                            .unwrap_or("unknown");
                        let type_label = match mem.memory_type {
                            MemoryType::Preference => "preference",
                            MemoryType::Fact => "fact",
                            MemoryType::Task => "task",
                            _ => "memory",
                        };
                        prompt.push_str(&format!("- [{}:{}] {}\n", channel_label, type_label, mem.content));
                    }
                    prompt.push('\n');
                }
            }
        }

        // Add available API keys (so the agent knows what credentials are configured)
        if let Ok(keys) = self.db.list_api_keys() {
            if !keys.is_empty() {
                prompt.push_str("## Available API Keys\n");
                prompt.push_str("The following API keys are configured and available as environment variables when using the exec tool:\n");
                for key in &keys {
                    prompt.push_str(&format!("- ${}\n", key.service_name));
                }
                prompt.push('\n');
            }
        }

        // Add context
        prompt.push_str(&format!(
            "## Current Request\nUser: {} | Channel: {}\n",
            message.user_name, message.channel_type
        ));

        prompt
    }

    /// Process memory markers in the AI response
    fn process_memory_markers(
        &self,
        response: &str,
        identity_id: &str,
        session_id: i64,
        channel_type: &str,
        message_id: Option<&str>,
    ) {
        let today = Utc::now().date_naive();

        for marker in &self.memory_markers {
            for cap in marker.pattern.captures_iter(response) {
                if let Some(content) = cap.get(1) {
                    let content_str = content.as_str().trim();
                    if !content_str.is_empty() {
                        let date = if marker.use_today_date { Some(today) } else { None };
                        if let Err(e) = self.db.create_memory(
                            marker.memory_type.clone(),
                            content_str,
                            None,
                            None,
                            marker.importance,
                            Some(identity_id),
                            Some(session_id),
                            Some(channel_type),
                            message_id,
                            date,
                            None,
                        ) {
                            log::error!("Failed to create {}: {}", marker.name, e);
                        } else {
                            log::info!("Created {}: {}", marker.name, content_str);
                        }
                    }
                }
            }
        }
    }

    /// Remove memory markers from the response before returning to user
    fn clean_response(&self, response: &str) -> String {
        let mut clean = response.to_string();
        for marker in &self.memory_markers {
            clean = marker.pattern.replace_all(&clean, "").to_string();
        }
        // Clean up any double spaces or trailing whitespace
        clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
        clean.trim().to_string()
    }

    /// Handle thinking directive messages (e.g., "/think:medium" sets session default)
    async fn handle_thinking_directive(&self, message: &NormalizedMessage) -> Option<DispatchResult> {
        let text = message.text.trim();

        // Check if this is a standalone thinking directive
        if let Some(captures) = self.thinking_directive_pattern.captures(text) {
            let level_str = captures.get(1).map(|m| m.as_str()).unwrap_or("low");

            if let Some(level) = ThinkingLevel::from_str(level_str) {
                // Store the thinking level preference for this session
                // For now, we just acknowledge it (session storage could be added later)
                let response = format!(
                    "Thinking level set to **{}**. {}",
                    level,
                    match level {
                        ThinkingLevel::Off => "Extended thinking is now disabled.",
                        ThinkingLevel::Minimal => "Using minimal thinking (~1K tokens).",
                        ThinkingLevel::Low => "Using low thinking (~4K tokens).",
                        ThinkingLevel::Medium => "Using medium thinking (~10K tokens).",
                        ThinkingLevel::High => "Using high thinking (~32K tokens).",
                        ThinkingLevel::XHigh => "Using maximum thinking (~64K tokens).",
                    }
                );

                self.broadcaster.broadcast(GatewayEvent::agent_response(
                    message.channel_id,
                    &message.user_name,
                    &response,
                ));

                log::info!(
                    "Thinking level set to {} for user {} on channel {}",
                    level,
                    message.user_name,
                    message.channel_id
                );

                return Some(DispatchResult::success(response));
            } else {
                // Invalid level specified
                let response = format!(
                    "Invalid thinking level '{}'. Valid options: off, minimal, low, medium, high, xhigh",
                    level_str
                );
                self.broadcaster.broadcast(GatewayEvent::agent_response(
                    message.channel_id,
                    &message.user_name,
                    &response,
                ));
                return Some(DispatchResult::success(response));
            }
        }

        None
    }

    /// Parse inline thinking directive from message (e.g., "/think:high What is...")
    /// Returns the thinking level and the clean message text
    fn parse_inline_thinking(&self, text: &str) -> (Option<ThinkingLevel>, Option<String>) {
        let text = text.trim();

        // Pattern: /think:level followed by the actual message
        let inline_pattern = Regex::new(r"(?i)^/(?:t|think|thinking):(\w+)\s+(.+)$").unwrap();

        if let Some(captures) = inline_pattern.captures(text) {
            let level_str = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let clean_text = captures.get(2).map(|m| m.as_str().to_string());

            if let Some(level) = ThinkingLevel::from_str(level_str) {
                return (Some(level), clean_text);
            }
        }

        // No inline thinking directive found
        (None, None)
    }

    /// Handle /new or /reset commands
    async fn handle_reset_command(&self, message: &NormalizedMessage) -> DispatchResult {
        // Determine session scope
        let scope = if message.chat_id != message.user_id {
            SessionScope::Group
        } else {
            SessionScope::Dm
        };

        // Get the current session
        match self.db.get_or_create_chat_session(
            &message.channel_type,
            message.channel_id,
            &message.chat_id,
            scope,
            None,
        ) {
            Ok(session) => {
                // Get identity for memory storage
                let identity_id = self.db.get_or_create_identity(
                    &message.channel_type,
                    &message.user_id,
                    Some(&message.user_name),
                ).ok().map(|id| id.identity_id);

                // Save session memory before reset (session memory hook)
                let message_count = self.db.count_session_messages(session.id).unwrap_or(0);
                if message_count >= 2 {
                    // Only save if there are meaningful messages
                    if let Ok(Some(settings)) = self.db.get_active_agent_settings() {
                        if let Ok(client) = AiClient::from_settings(&settings) {
                            match context::save_session_memory(
                                &self.db,
                                &client,
                                session.id,
                                identity_id.as_deref(),
                                15, // Save last 15 messages
                            ).await {
                                Ok(memory_id) => {
                                    log::info!("[SESSION_MEMORY] Saved session memory (id={}) before reset", memory_id);
                                }
                                Err(e) => {
                                    log::warn!("[SESSION_MEMORY] Failed to save session memory: {}", e);
                                }
                            }
                        }
                    }
                }

                // Reset the session
                match self.db.reset_chat_session(session.id) {
                    Ok(_) => {
                        let response = "Session reset. Let's start fresh!".to_string();
                        self.broadcaster.broadcast(GatewayEvent::agent_response(
                            message.channel_id,
                            &message.user_name,
                            &response,
                        ));
                        DispatchResult::success(response)
                    }
                    Err(e) => {
                        log::error!("Failed to reset session: {}", e);
                        DispatchResult::error(format!("Failed to reset session: {}", e))
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to get session for reset: {}", e);
                DispatchResult::error(format!("Session error: {}", e))
            }
        }
    }
}
