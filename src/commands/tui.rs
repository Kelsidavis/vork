use anyhow::{Context, Result};
use arboard::Clipboard;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::env;
use std::io;

use crate::config::Config;
use crate::llm::{LlamaClient, ServerManager, Session, ApprovalSystem};
use crate::llm::tools::{get_available_tools, execute_tool};
use crate::agents::Agent;

fn detect_current_preset(config: &Config) -> String {
    // Try to match current config against available presets
    if let Ok(config_dir) = Config::config_dir() {
        let presets_dir = config_dir.join("presets");
        if let Ok(entries) = std::fs::read_dir(&presets_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Some(name) = entry.path().file_stem() {
                        let name_str = name.to_string_lossy();
                        if name_str == "README" {
                            continue;
                        }
                        // Read preset and compare key fields
                        if let Ok(preset_content) = std::fs::read_to_string(entry.path()) {
                            if let Ok(preset_config) = toml::from_str::<Config>(&preset_content) {
                                // Match on context_size and cuda_visible_devices as key identifiers
                                if preset_config.llamacpp.context_size == config.llamacpp.context_size
                                    && preset_config.llamacpp.cuda_visible_devices == config.llamacpp.cuda_visible_devices {
                                    return name_str.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    "custom".to_string()
}

fn parse_color(color_name: &str) -> Color {
    match color_name.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::Cyan, // default
    }
}

fn fetch_gpu_stats() -> Vec<GpuStats> {
    use std::process::Command;

    let output = Command::new("nvidia-smi")
        .args(&[
            "--query-gpu=name,memory.used,memory.total,utilization.gpu,temperature.gpu",
            "--format=csv,noheader,nounits"
        ])
        .output();

    let Ok(output) = output else {
        return vec![];
    };

    if !output.status.success() {
        return vec![];
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 5 {
                Some(GpuStats {
                    name: parts[0].to_string(),
                    memory_used: parts[1].parse().unwrap_or(0),
                    memory_total: parts[2].parse().unwrap_or(0),
                    utilization: parts[3].parse().unwrap_or(0),
                    temperature: parts[4].parse().unwrap_or(0),
                })
            } else {
                None
            }
        })
        .collect()
}

struct GpuStats {
    name: String,
    memory_used: u32,
    memory_total: u32,
    utilization: u32,
    temperature: u32,
}

struct App {
    input: String,
    messages: Vec<(String, String)>, // (role, content)
    scroll: u16,
    input_scroll: u16,  // Vertical scroll offset for input box
    auto_scroll: bool,  // Auto-scroll to follow new messages
    session: Session,
    client: LlamaClient,
    approval_system: ApprovalSystem,
    status: String,
    tokens_used: usize,
    processing: bool,
    spinner_state: usize,
    tokens_per_second: f64,
    #[allow(dead_code)]
    last_token_time: std::time::Instant,
    agent_color: Color,
    header_title: String,
    agent_explicitly_set: bool,
    first_message: bool,
    input_history: Vec<String>,
    history_index: Option<usize>,
    current_input_backup: String,
    gpu_stats: Vec<GpuStats>,
    model_selector_active: bool,
    available_presets: Vec<String>,
    selected_preset_index: usize,
    model_override: Option<String>,  // None = auto, Some = forced preset
    current_preset_name: String,  // Track current preset for display
}

impl App {
    fn new(server_url: String, model: String, config: Config, agent: Option<Agent>) -> Self {
        let working_dir = env::current_dir().unwrap_or_default();
        let mut session = Session::new(working_dir);
        session.conversation.set_max_context(config.llamacpp.context_limit);
        let client = LlamaClient::new(server_url.clone(), model.clone());
        let approval_system = ApprovalSystem::new(
            config.assistant.approval_policy.clone(),
            config.assistant.sandbox_mode.clone(),
        );

        // Extract agent color and title
        let agent_color = if let Some(ref agent) = agent {
            parse_color(&agent.color)
        } else {
            Color::Cyan
        };

        let header_title = if let Some(ref agent) = agent {
            agent.title.clone().unwrap_or_else(|| format!("🤖 {}", agent.name))
        } else {
            "🐴 VORK - AI Coding Assistant".to_string()
        };

        // Use agent's system prompt if provided
        if let Some(ref agent) = agent {
            session.conversation.messages[0].content = agent.system_prompt.clone();
        }

        let agent_info = if let Some(ref agent) = agent {
            format!(" | Agent: {}", agent.name)
        } else {
            String::new()
        };

        // Find available presets - automatically discover all .toml files in presets directory
        let mut available_presets = vec!["auto".to_string()];  // Start with "auto" option
        if let Ok(config_dir) = Config::config_dir() {
            let presets_dir = config_dir.join("presets");
            if presets_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&presets_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.path().file_stem() {
                            if entry.path().extension().and_then(|s| s.to_str()) == Some("toml")
                                && name != "README" {
                                available_presets.push(name.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
        // Sort presets but keep "auto" first
        let mut other_presets = available_presets[1..].to_vec();
        other_presets.sort();
        available_presets = vec!["auto".to_string()];
        available_presets.extend(other_presets);

        // Detect current preset by comparing config file
        let current_preset_name = detect_current_preset(&config);
        let context_info = format!("{}k ctx", config.llamacpp.context_size / 1024);

        let mut app = Self {
            input: String::new(),
            messages: vec![],
            scroll: 0,
            input_scroll: 0,
            auto_scroll: true,  // Start with auto-scroll enabled
            session,
            client,
            approval_system,
            status: format!("Preset: {} ({}) | Mode: auto{}", current_preset_name, context_info, agent_info),
            tokens_used: 0,
            processing: false,
            spinner_state: 0,
            tokens_per_second: 0.0,
            last_token_time: std::time::Instant::now(),
            agent_color,
            header_title: header_title.clone(),
            agent_explicitly_set: agent.is_some(),
            first_message: true,
            input_history: vec![],
            history_index: None,
            current_input_backup: String::new(),
            gpu_stats: vec![],
            model_selector_active: false,
            available_presets,
            selected_preset_index: 0,
            model_override: None,  // Start in auto mode
            current_preset_name: current_preset_name.clone(),
        };

        // Add system message with agent info
        let welcome_msg = if let Some(agent) = agent {
            format!("🤖 {} - {}", agent.name, agent.description)
        } else {
            "Vork AI Coding Assistant - Type your message and press Enter".to_string()
        };

        app.messages.push((
            "system".to_string(),
            welcome_msg,
        ));

        app
    }

    // Prepare UI for sending message (synchronous part)
    fn prepare_send_message(&mut self) {
        if self.input.trim().is_empty() {
            return;
        }

        let user_message = self.input.clone();

        // Add user message to display immediately
        self.messages.push(("user".to_string(), user_message.clone()));

        // Add to input history
        self.input_history.push(user_message);
        self.history_index = None;
        self.current_input_backup.clear();

        // Clear input and mark as processing
        self.input.clear();
        self.input_scroll = 0;
        self.processing = true;

        // Add immediate "thinking" feedback
        self.messages.push((
            "system".to_string(),
            "💭 Thinking...".to_string(),
        ));

        // Auto-scroll to bottom so user sees the messages (only if auto-scroll enabled)
        if self.auto_scroll {
            self.scroll = u16::MAX;
        }
    }

    // Do the actual LLM work (async part)
    async fn do_send_message(&mut self) -> Result<()> {
        // Get the last user message (the one we just added in prepare)
        let user_message = self.input_history.last().unwrap().clone();

        // Auto-select agent based on first message if no agent was explicitly set
        if self.first_message && !self.agent_explicitly_set {
            if let Ok(Some(agent)) = Agent::auto_select(&user_message) {
                // Update session with agent's system prompt
                self.session.conversation.messages[0].content = agent.system_prompt.clone();

                // Update UI with agent's color and title
                self.agent_color = parse_color(&agent.color);
                self.header_title = agent.title.clone().unwrap_or_else(|| format!("🤖 {}", agent.name));

                // Switch model preset if agent has a preference AND no manual override is set
                if self.model_override.is_none() {
                    if let Some(ref preferred_preset) = agent.preferred_preset {
                        // Attempt to switch preset (no intermediate messages during switch)
                        if let Err(e) = self.switch_to_preset(preferred_preset).await {
                            self.messages.push((
                                "system".to_string(),
                                format!("🎯 Agent: {} - {} | ⚠️  Failed to switch model: {}",
                                    agent.name, agent.description, e),
                            ));
                        } else {
                            // Update current preset tracking
                            self.current_preset_name = preferred_preset.clone();

                            // Update status bar
                            if let Ok(config) = Config::load() {
                                let context_info = format!("{}k ctx", config.llamacpp.context_size / 1024);
                                self.status = format!("Preset: {} ({}) | Mode: auto", preferred_preset, context_info);
                            }

                            // Single message after successful switch
                            self.messages.push((
                                "system".to_string(),
                                format!("🎯 Agent: {} - {} | ✅ Model: {}",
                                    agent.name, agent.description, preferred_preset),
                            ));
                        }
                    } else {
                        // Show agent selection message
                        self.messages.push((
                            "system".to_string(),
                            format!("🎯 Auto-selected agent: {} - {}", agent.name, agent.description),
                        ));
                    }
                } else {
                    // Model override is set, don't auto-switch
                    self.messages.push((
                        "system".to_string(),
                        format!("🎯 Auto-selected agent: {} - {} (using forced model: {})",
                            agent.name, agent.description, self.model_override.as_ref().unwrap()),
                    ));
                }
            }
            self.first_message = false;
        }

        let start_time = std::time::Instant::now();
        let mut total_tokens = 0usize;

        self.session.conversation.add_user_message(user_message);

        // Process with LLM
        loop {
            let response = self
                .client
                .chat_completion(
                    self.session.conversation.get_messages(),
                    Some(get_available_tools()),
                )
                .await
                .context("Failed to get response from LLM")?;

            let choice = response
                .choices
                .first()
                .ok_or_else(|| anyhow::anyhow!("No response from LLM"))?;

            // Check if there are tool calls
            if let Some(tool_calls) = &choice.message.tool_calls {
                // Remove the "Thinking..." message before showing tool execution
                if let Some(last_msg) = self.messages.last() {
                    if last_msg.0 == "system" && last_msg.1 == "💭 Thinking..." {
                        self.messages.pop();
                    }
                }

                for tool_call in tool_calls {
                    let tool_name = &tool_call.function.name;
                    let arguments: serde_json::Value =
                        serde_json::from_str(&tool_call.function.arguments)
                            .context("Failed to parse tool arguments")?;

                    self.messages.push((
                        "tool".to_string(),
                        format!("🔧 Executing: {}", tool_name),
                    ));

                    match execute_tool(tool_name, arguments, Some(&self.approval_system)).await {
                        Ok(result) => {
                            self.session.conversation.add_tool_result(tool_name, &result);
                            // Show truncated result
                            let truncated = if result.len() > 200 {
                                format!("{}...", &result[..200])
                            } else {
                                result
                            };
                            self.messages
                                .push(("tool_result".to_string(), truncated));
                        }
                        Err(e) => {
                            let error_msg = format!("Error: {}", e);
                            self.session
                                .conversation
                                .add_tool_result(tool_name, &error_msg);
                            self.messages
                                .push(("error".to_string(), error_msg));
                        }
                    }
                }
                continue;
            }

            // If no tool calls, process the assistant's message
            if let Some(content) = &choice.message.content {
                // Remove the "Thinking..." message
                if let Some(last_msg) = self.messages.last() {
                    if last_msg.0 == "system" && last_msg.1 == "💭 Thinking..." {
                        self.messages.pop();
                    }
                }

                // Filter out llama.cpp internal slot messages only
                let filtered_content: String = content
                    .lines()
                    .filter(|line| {
                        let line_lower = line.to_lowercase();
                        // Only filter lines that look like llama.cpp internal messages
                        !line_lower.starts_with("slot ") &&
                        !line_lower.contains("slot processing") &&
                        !line_lower.contains("slot released") &&
                        !line.trim().is_empty()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if !filtered_content.is_empty() {
                    self.messages
                        .push(("assistant".to_string(), filtered_content.clone()));
                    self.session
                        .conversation
                        .add_assistant_message(filtered_content.clone());
                    total_tokens += filtered_content.len() / 4; // Rough estimate
                } else if content.trim().is_empty() {
                    // If content is empty or only whitespace, show a warning
                    self.messages.push((
                        "system".to_string(),
                        "⚠️  Assistant sent empty response - this may indicate a model issue".to_string()
                    ));
                }
            } else {
                // No content at all in the response
                self.messages.push((
                    "system".to_string(),
                    "⚠️  No content in response - model may have sent only tool calls or empty message".to_string()
                ));
            }

            break;
        }

        // Calculate tokens/second
        let elapsed = start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 && total_tokens > 0 {
            self.tokens_per_second = total_tokens as f64 / elapsed;
        }
        self.tokens_used += total_tokens;

        // Compact conversation if needed
        let compacted = self.session.conversation.compact_if_needed(&self.client).await?;
        if compacted {
            self.messages.push((
                "system".to_string(),
                "🔄 Context compaction completed: Older messages have been summarized to save space while preserving key information.".to_string()
            ));
        }

        self.session.save()?;
        self.processing = false;

        // Auto-scroll to bottom after new messages (only if auto-scroll enabled)
        if self.auto_scroll {
            self.scroll = u16::MAX;
        }

        let (used, max, percentage) = self.session.conversation.get_context_usage();
        self.status = format!(
            "Session: {} | Messages: {} | Tokens: {} | Context: {}/{}tok ({:.1}%)",
            self.session.id,
            self.messages.len(),
            self.tokens_used,
            used,
            max,
            percentage
        );

        Ok(())
    }

    async fn handle_compact_command(&mut self) -> Result<()> {
        self.input.clear();
        self.input_scroll = 0;

        let msg_count_before = self.session.conversation.messages.len();
        let (used_before, _, _) = self.session.conversation.get_context_usage();

        // Force compaction even if below threshold
        if self.session.conversation.messages.len() <= 11 {
            self.messages.push((
                "system".to_string(),
                "❌ Cannot compact: Need at least 12 messages (system prompt + at least 11 messages) to perform compaction.".to_string()
            ));
            return Ok(());
        }

        self.messages.push((
            "system".to_string(),
            "🔄 Starting manual context compaction...".to_string()
        ));

        // Temporarily override the needs_compaction check by setting a flag
        let system_msg = self.session.conversation.messages[0].clone();
        let messages_to_compact: Vec<_> = self.session.conversation.messages.iter()
            .skip(1)
            .take(self.session.conversation.messages.len() - 11)
            .cloned()
            .collect();
        let recent_messages: Vec<_> = self.session.conversation.messages.iter()
            .skip(self.session.conversation.messages.len() - 10)
            .cloned()
            .collect();

        if messages_to_compact.is_empty() {
            self.messages.push((
                "system".to_string(),
                "❌ Cannot compact: Not enough messages to summarize.".to_string()
            ));
            return Ok(());
        }

        // Create summarization prompt
        let conversation_text = messages_to_compact.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let summary_prompt = format!(
            "Summarize the following conversation history concisely, preserving key facts, decisions, and context. Focus on:\n\
            - Important technical details and decisions\n\
            - File modifications and their purposes\n\
            - Commands executed and their results\n\
            - Any errors or issues encountered\n\n\
            Conversation:\n{}\n\n\
            Provide a concise summary in 2-3 paragraphs:",
            conversation_text
        );

        // Get summary from LLM
        let response = self.client.chat_completion(vec![
            super::super::llm::client::Message {
                role: "user".to_string(),
                content: summary_prompt,
            }
        ], None).await?;

        let summary_response = response.choices[0].message.content.clone()
            .unwrap_or_default();

        // Rebuild conversation with summary
        let summary_msg = super::super::llm::client::Message {
            role: "assistant".to_string(),
            content: format!("[Conversation summary of {} messages]\n\n{}",
                messages_to_compact.len(), summary_response),
        };

        // Recalculate tokens
        self.session.conversation.estimated_tokens =
            (system_msg.content.len() / 4) + 10 +
            (summary_msg.content.len() / 4) + 10;
        for msg in &recent_messages {
            self.session.conversation.estimated_tokens += (msg.content.len() / 4) + 10;
        }

        // Rebuild messages
        self.session.conversation.messages = vec![system_msg, summary_msg];
        self.session.conversation.messages.extend(recent_messages);

        let (used_after, _, _) = self.session.conversation.get_context_usage();
        let saved_tokens = used_before.saturating_sub(used_after);

        self.messages.push((
            "system".to_string(),
            format!("✅ Compaction complete: {} messages → {} messages. Saved ~{} tokens.",
                msg_count_before, self.session.conversation.messages.len(), saved_tokens)
        ));

        if self.auto_scroll {
            self.scroll = u16::MAX;
        }
        self.session.save()?;
        Ok(())
    }

    async fn handle_model_command(&mut self) -> Result<()> {
        self.input.clear();
        self.input_scroll = 0;

        if self.available_presets.is_empty() {
            self.messages.push((
                "system".to_string(),
                "❌ No model presets found in ~/.vork/presets/".to_string()
            ));
            return Ok(());
        }

        // Activate model selector
        self.model_selector_active = true;
        self.messages.push((
            "system".to_string(),
            "🔧 Model Selection Mode: Use ↑/↓ arrows to navigate, Enter to select, Esc to cancel".to_string()
        ));

        if self.auto_scroll {
            self.scroll = u16::MAX;
        }
        Ok(())
    }

    async fn switch_to_preset(&mut self, preset_name: &str) -> Result<()> {
        // Copy preset to config
        let config_dir = Config::config_dir()?;
        let presets_dir = config_dir.join("presets");
        let preset_path = presets_dir.join(format!("{}.toml", preset_name));
        let config_path = Config::config_path()?;

        if !preset_path.exists() {
            anyhow::bail!("Preset file not found: {:?}", preset_path);
        }

        // Copy preset to config for persistence
        std::fs::copy(&preset_path, &config_path)
            .context("Failed to copy preset to config")?;

        // Kill existing llama-server
        let _ = std::process::Command::new("pkill")
            .arg("llama-server")
            .output();

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Start new server with the new config
        crate::backends::llamacpp::LlamaCppBackend::start_server(8080)?;

        // Give server time to initialize
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        Ok(())
    }

    async fn confirm_model_selection(&mut self) -> Result<()> {
        if self.available_presets.is_empty() {
            return Ok(());
        }

        let preset_name = self.available_presets[self.selected_preset_index].clone();
        self.model_selector_active = false;

        // Handle "auto" selection
        if preset_name == "auto" {
            self.model_override = None;

            // Update status to show auto mode
            if let Ok(config) = Config::load() {
                let context_info = format!("{}k ctx", config.llamacpp.context_size / 1024);
                self.status = format!("Preset: {} ({}) | Mode: auto", self.current_preset_name, context_info);
            }

            self.messages.push((
                "system".to_string(),
                "✅ Model selection set to AUTO - agents will use their preferred models".to_string()
            ));
            if self.auto_scroll {
                self.scroll = u16::MAX;
            }
            return Ok(());
        }

        // Manual preset selection - set override and switch (no intermediate messages)
        match self.switch_to_preset(&preset_name).await {
            Ok(_) => {
                // Set override so this model is used for all agents
                self.model_override = Some(preset_name.clone());
                self.current_preset_name = preset_name.clone();

                // Reload config to get new context size
                if let Ok(new_config) = Config::load() {
                    let context_info = format!("{}k ctx", new_config.llamacpp.context_size / 1024);
                    self.status = format!("Preset: {} ({}) | Mode: forced", preset_name, context_info);
                }

                self.messages.push((
                    "system".to_string(),
                    format!("✅ Model FORCED to {} - all agents will use this model", preset_name)
                ));
            }
            Err(e) => {
                self.messages.push((
                    "system".to_string(),
                    format!("❌ Failed to switch model: {}", e)
                ));
            }
        }

        if self.auto_scroll {
            self.scroll = u16::MAX;
        }
        Ok(())
    }

    fn handle_copy_command(&mut self) -> Result<()> {
        self.input.clear();
        self.input_scroll = 0;

        // Build the full conversation text
        let mut conversation_text = String::new();

        for (role, content) in &self.messages {
            let prefix = match role.as_str() {
                "user" => "👤 You",
                "assistant" => "🐴 Vork",
                "tool" => "🔧 Tool",
                "tool_result" => "📄 Result",
                "error" => "❌ Error",
                "system" => "ℹ️  System",
                _ => role,
            };

            conversation_text.push_str(&format!("{}: {}\n\n", prefix, content));
        }

        // Copy to clipboard
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.set_text(&conversation_text) {
                    Ok(_) => {
                        self.messages.push((
                            "system".to_string(),
                            format!("✅ Copied {} messages to clipboard", self.messages.len())
                        ));
                    }
                    Err(e) => {
                        self.messages.push((
                            "system".to_string(),
                            format!("❌ Failed to copy to clipboard: {}", e)
                        ));
                    }
                }
            }
            Err(e) => {
                self.messages.push((
                    "system".to_string(),
                    format!("❌ Failed to access clipboard: {}", e)
                ));
            }
        }

        if self.auto_scroll {
            self.scroll = u16::MAX;
        }
        Ok(())
    }

    fn history_prev(&mut self) {
        if self.input_history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // First time navigating history, save current input
                self.current_input_backup = self.input.clone();
                self.history_index = Some(self.input_history.len() - 1);
                self.input = self.input_history[self.history_index.unwrap()].clone();
                self.input_scroll = 0;
            }
            Some(index) => {
                if index > 0 {
                    self.history_index = Some(index - 1);
                    self.input = self.input_history[self.history_index.unwrap()].clone();
                    self.input_scroll = 0;
                }
            }
        }
    }

    fn history_next(&mut self) {
        if let Some(index) = self.history_index {
            if index < self.input_history.len() - 1 {
                self.history_index = Some(index + 1);
                self.input = self.input_history[self.history_index.unwrap()].clone();
                self.input_scroll = 0;
            } else {
                // Reached the end, restore backup
                self.history_index = None;
                self.input = self.current_input_backup.clone();
                self.input_scroll = 0;
            }
        }
    }
}

pub async fn execute(server_url: Option<String>, model: Option<String>, agent_name: Option<String>) -> Result<()> {
    let config = Config::load()?;

    // Load agent if specified
    let agent = if let Some(name) = agent_name {
        Some(Agent::load(&name)?)
    } else {
        None
    };

    // Auto-start server if not specified
    let server_url = if let Some(url) = server_url {
        url
    } else {
        let mut server_manager = ServerManager::new()?;
        server_manager.start_server().await?
    };

    let model = model.unwrap_or_else(|| config.assistant.model.clone());

    // Warm up model with a tiny prompt (async, non-blocking)
    let warmup_client = LlamaClient::new(server_url.clone(), model.clone());
    tokio::spawn(async move {
        let _ = warmup_client.chat_completion(
            vec![crate::llm::client::Message {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            None,
        ).await;
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(server_url, model, config, agent);

    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    let mut gpu_update_counter = 0;
    loop {
        // Clamp scroll before drawing
        let max_scroll = app.messages.len().saturating_sub(1);
        if app.scroll as usize > max_scroll {
            app.scroll = max_scroll as u16;
        }

        terminal.draw(|f| ui(f, app))?;

        // Update spinner animation when processing
        if app.processing {
            app.spinner_state = (app.spinner_state + 1) % 10;
        }

        // Update GPU stats every 1 second (10 iterations * 100ms)
        gpu_update_counter += 1;
        if gpu_update_counter >= 10 {
            app.gpu_stats = fetch_gpu_stats();
            gpu_update_counter = 0;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Char(c) => {
                            if !app.processing {
                                app.input.push(c);
                                // Reset history navigation when typing
                                app.history_index = None;
                            }
                        }
                        KeyCode::Backspace => {
                            if !app.processing {
                                app.input.pop();
                                // Reset history navigation when editing
                                app.history_index = None;
                            }
                        }
                        KeyCode::Enter => {
                            if app.model_selector_active {
                                // Confirm model selection
                                app.confirm_model_selection().await?;
                            } else if !app.processing {
                                let input = app.input.trim();
                                if input == "exit" || input == "quit" {
                                    return Ok(());
                                }
                                if input == "/compact" {
                                    app.handle_compact_command().await?;
                                } else if input == "/model" {
                                    app.handle_model_command().await?;
                                } else if input == "/copy" {
                                    app.handle_copy_command()?;
                                } else {
                                    // Prepare UI for processing before async call
                                    app.prepare_send_message();
                                    // Force immediate redraw to show processing state
                                    terminal.draw(|f| ui(f, app))?;
                                    // Now do the async LLM work
                                    app.do_send_message().await?;
                                }
                            }
                        }
                        KeyCode::Up => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                // Ctrl+Up: Scroll input box up
                                app.input_scroll = app.input_scroll.saturating_sub(1);
                            } else if app.model_selector_active {
                                // Navigate model selection
                                if app.selected_preset_index > 0 {
                                    app.selected_preset_index -= 1;
                                }
                            } else if !app.processing {
                                // Navigate to previous command in history
                                app.history_prev();
                            }
                        }
                        KeyCode::Down => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                // Ctrl+Down: Scroll input box down
                                app.input_scroll = app.input_scroll.saturating_add(1);
                            } else if app.model_selector_active {
                                // Navigate model selection
                                if app.selected_preset_index < app.available_presets.len().saturating_sub(1) {
                                    app.selected_preset_index += 1;
                                }
                            } else if !app.processing {
                                // Navigate to next command in history
                                app.history_next();
                            }
                        }
                        KeyCode::Tab => {
                            if !app.processing && !app.model_selector_active {
                                app.model_selector_active = true;
                            }
                        }
                        KeyCode::Esc => {
                            if app.model_selector_active {
                                app.model_selector_active = false;
                                app.messages.push((
                                    "system".to_string(),
                                    "❌ Model selection cancelled".to_string()
                                ));
                            }
                        }
                        KeyCode::PageUp => {
                            // Scroll up - disable auto-scroll
                            app.auto_scroll = false;
                            app.scroll = app.scroll.saturating_sub(5);
                        }
                        KeyCode::PageDown => {
                            // Scroll down - disable auto-scroll
                            app.auto_scroll = false;
                            app.scroll = app.scroll.saturating_add(5);
                        }
                        KeyCode::Home => {
                            // Return to bottom and resume auto-scroll
                            app.auto_scroll = true;
                            app.scroll = u16::MAX;
                        }
                        KeyCode::End => {
                            // Scroll to bottom and enable auto-scroll
                            app.auto_scroll = true;
                            app.scroll = u16::MAX;
                        }
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {
                    // Handle terminal resize by redrawing
                    terminal.autoresize()?;
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        event::MouseEventKind::ScrollUp => {
                            // Scroll up - disable auto-scroll
                            app.auto_scroll = false;
                            app.scroll = app.scroll.saturating_sub(3);
                        }
                        event::MouseEventKind::ScrollDown => {
                            // Scroll down - disable auto-scroll
                            app.auto_scroll = false;
                            app.scroll = app.scroll.saturating_add(3);
                        }
                        event::MouseEventKind::Down(MouseButton::Right) => {
                            // Right-click paste from clipboard
                            if let Ok(mut clipboard) = Clipboard::new() {
                                if let Ok(text) = clipboard.get_text() {
                                    app.input.push_str(&text);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Handle very small terminals gracefully
    if size.height < 10 || size.width < 20 {
        let error_msg = Paragraph::new("Terminal too small! Please resize.")
            .style(Style::default().fg(Color::Red));
        f.render_widget(error_msg, size);
        return;
    }

    // Calculate GPU panel height based on number of GPUs (0 if none detected)
    let gpu_height = if app.gpu_stats.is_empty() {
        0
    } else {
        (app.gpu_stats.len() as u16 * 2) + 2 // 2 lines per GPU + 2 for borders
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Header
            Constraint::Min(5),          // Messages
            Constraint::Length(4),       // Input (2 text rows + borders)
            Constraint::Length(3),       // Status
            Constraint::Length(3),       // Context usage
            Constraint::Length(gpu_height), // GPU stats (dynamic)
        ])
        .split(size);

    // Header with agent-specific color and title
    let header = Paragraph::new(app.header_title.clone())
        .style(
            Style::default()
                .fg(app.agent_color)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Messages with text wrapping
    let available_width = chunks[1].width.saturating_sub(4); // Account for borders and padding
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|(role, content)| {
            let style = match role.as_str() {
                "user" => Style::default().fg(Color::Blue),
                "assistant" => Style::default().fg(app.agent_color),
                "tool" => Style::default().fg(Color::Yellow),
                "tool_result" => Style::default().fg(Color::Gray),
                "error" => Style::default().fg(Color::Red),
                "system" => Style::default().fg(app.agent_color),
                _ => Style::default(),
            };

            let prefix = match role.as_str() {
                "user" => "👤 You",
                "assistant" => "🐴 Vork",
                "tool" => "🔧 Tool",
                "tool_result" => "📄 Result",
                "error" => "❌ Error",
                "system" => "ℹ️  System",
                _ => role,
            };

            let prefix_text = format!("{}: ", prefix);
            let prefix_len = prefix_text.chars().count();
            let wrap_width = available_width.saturating_sub(prefix_len as u16).max(20) as usize;

            let mut lines: Vec<Line> = Vec::new();

            // Wrap each line of content
            for (line_idx, line) in content.lines().enumerate() {
                if line_idx == 0 {
                    // First line includes the prefix
                    for wrapped_line in textwrap::wrap(line, wrap_width) {
                        if lines.is_empty() {
                            // Very first line with prefix
                            lines.push(Line::from(vec![
                                Span::styled(
                                    prefix_text.clone(),
                                    style.add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(wrapped_line.to_string(), style),
                            ]));
                        } else {
                            // Continuation lines indented
                            lines.push(Line::from(vec![
                                Span::styled(
                                    " ".repeat(prefix_len),
                                    style,
                                ),
                                Span::styled(wrapped_line.to_string(), style),
                            ]));
                        }
                    }
                } else {
                    // Subsequent lines (newlines in original content)
                    for wrapped_line in textwrap::wrap(line, wrap_width) {
                        lines.push(Line::from(vec![
                            Span::styled(
                                " ".repeat(prefix_len),
                                style,
                            ),
                            Span::styled(wrapped_line.to_string(), style),
                        ]));
                    }
                }
            }

            // Handle empty content
            if lines.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(
                        prefix_text,
                        style.add_modifier(Modifier::BOLD),
                    ),
                ]));
            }

            ListItem::new(lines)
        })
        .collect();

    // Scroll position already clamped in run_app

    let conversation_title = if app.auto_scroll {
        "Conversation (Auto-scroll ON | Home/End: scroll to bottom | PgUp/PgDn: manual scroll)"
    } else {
        "Conversation (Auto-scroll OFF | Home/End: return to bottom & resume auto-scroll)"
    };

    let messages_widget = List::new(messages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(conversation_title),
        )
        .style(Style::default().fg(Color::White));

    // Create a stateful widget to enable scrolling
    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(app.scroll as usize));

    f.render_stateful_widget(messages_widget, chunks[1], &mut list_state);

    // Input with animated spinner and clear status
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let (input_text, input_style, input_title, border_color) = if app.processing {
        (
            format!("{} AI is analyzing your request and generating response...", spinner_frames[app.spinner_state]),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            "⏳ PROCESSING - Please wait, response incoming...",
            Color::Yellow,
        )
    } else {
        (
            format!("💬 {}", app.input),
            Style::default().fg(Color::White),
            "✅ Ready (Ctrl+↑↓ scroll input | Right-click paste | /compact /model /copy)",
            Color::Green,
        )
    };

    let input = Paragraph::new(input_text)
        .style(input_style)
        .wrap(Wrap { trim: false })
        .scroll((app.input_scroll, 0))  // Vertical scroll support
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(input_title)
                .border_style(Style::default().fg(border_color)),
        );
    f.render_widget(input, chunks[2]);

    // Status bar with processing indicator and tokens/s
    let status_text = if app.processing {
        let spinner = spinner_frames[app.spinner_state];
        if app.tokens_per_second > 0.0 {
            format!("{} {} │ {:.1} tok/s │ ⏳ Processing...", spinner, app.status, app.tokens_per_second)
        } else {
            format!("{} {} │ ⏳ Processing...", spinner, app.status)
        }
    } else if app.tokens_per_second > 0.0 {
        format!("{} │ {:.1} tok/s │ ✅ Idle", app.status, app.tokens_per_second)
    } else {
        format!("{} │ ✅ Ready", app.status)
    };

    let status_style = if app.processing {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.agent_color)
    };

    let status = Paragraph::new(status_text)
        .style(status_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Status")
                .border_style(if app.processing {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                })
        );
    f.render_widget(status, chunks[3]);

    // Context usage panel
    let (used, max, percentage) = app.session.conversation.get_context_usage();
    let context_color = if percentage >= 75.0 {
        Color::Red
    } else if percentage >= 50.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let remaining = max.saturating_sub(used);
    let context_text = format!(
        "Used: {}tok │ Remaining: {}tok │ Total: {}tok │ Usage: {:.1}%",
        used, remaining, max, percentage
    );

    // Check if compaction will happen
    let needs_compaction = app.session.conversation.needs_compaction();
    let compaction_notice = if needs_compaction {
        " │ ⚠️  Compaction will occur after next message"
    } else {
        ""
    };

    let full_context_text = format!("{}{}", context_text, compaction_notice);

    let context_widget = Paragraph::new(full_context_text)
        .style(Style::default().fg(context_color))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📊 Context Usage")
                .border_style(Style::default().fg(context_color))
        );
    f.render_widget(context_widget, chunks[4]);

    // GPU stats (if available)
    if !app.gpu_stats.is_empty() {
        let gpu_lines: Vec<Line> = app.gpu_stats.iter().enumerate().flat_map(|(idx, gpu)| {
            let mem_percent = if gpu.memory_total > 0 {
                (gpu.memory_used as f64 / gpu.memory_total as f64 * 100.0) as u32
            } else {
                0
            };

            let mem_color = if mem_percent >= 90 {
                Color::Red
            } else if mem_percent >= 75 {
                Color::Yellow
            } else {
                Color::Green
            };

            let temp_color = if gpu.temperature >= 80 {
                Color::Red
            } else if gpu.temperature >= 70 {
                Color::Yellow
            } else {
                Color::Green
            };

            vec![
                Line::from(vec![
                    Span::styled(
                        format!("GPU{}: ", idx),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &gpu.name,
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        format!("  VRAM: {}/{}MB ({}%) ", gpu.memory_used, gpu.memory_total, mem_percent),
                        Style::default().fg(mem_color),
                    ),
                    Span::styled(
                        format!("│ Load: {}% ", gpu.utilization),
                        Style::default().fg(if gpu.utilization >= 80 { Color::Green } else { Color::Gray }),
                    ),
                    Span::styled(
                        format!("│ Temp: {}°C", gpu.temperature),
                        Style::default().fg(temp_color),
                    ),
                ]),
            ]
        }).collect();

        let gpu_widget = Paragraph::new(gpu_lines)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🎮 GPU Stats")
                    .border_style(Style::default().fg(Color::Cyan))
            );
        f.render_widget(gpu_widget, chunks[5]);
    }

    // Render model selector popup if active
    if app.model_selector_active {
        let popup_height = (app.available_presets.len() as u16).min(10) + 2; // Max 10 items visible + borders
        let popup_width = 60;

        let popup_area = centered_rect(popup_width, popup_height, size);

        // Create list items for model selector
        let model_items: Vec<ListItem> = app.available_presets.iter().enumerate()
            .map(|(idx, preset)| {
                let content = if idx == app.selected_preset_index {
                    format!("→ {}", preset)
                } else {
                    format!("  {}", preset)
                };
                let style = if idx == app.selected_preset_index {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(content).style(style)
            })
            .collect();

        let model_list = List::new(model_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🔧 Select Model (↑/↓: Navigate, Enter: Switch, Esc: Cancel)")
                    .border_style(Style::default().fg(Color::Cyan))
            )
            .style(Style::default().bg(Color::Black));

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(app.selected_preset_index));

        // Clear the area behind the popup
        let clear_widget = Block::default().style(Style::default().bg(Color::Black));
        f.render_widget(clear_widget, popup_area);

        f.render_stateful_widget(model_list, popup_area, &mut list_state);
    }
}

// Helper function to create a centered rectangle
fn centered_rect(width: u16, height: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
}
