use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::env;
use std::io;

use crate::config::Config;
use crate::llm::{LlamaClient, ServerManager, Session, ApprovalSystem};
use crate::llm::tools::{get_available_tools, execute_tool};
use crate::agents::Agent;

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

struct App {
    input: String,
    messages: Vec<(String, String)>, // (role, content)
    scroll: u16,
    session: Session,
    client: LlamaClient,
    approval_system: ApprovalSystem,
    status: String,
    tokens_used: usize,
    processing: bool,
    spinner_state: usize,
    tokens_per_second: f64,
    last_token_time: std::time::Instant,
    agent_color: Color,
    header_title: String,
    agent_explicitly_set: bool,
    first_message: bool,
    input_history: Vec<String>,
    history_index: Option<usize>,
    current_input_backup: String,
}

impl App {
    fn new(server_url: String, model: String, config: Config, agent: Option<Agent>) -> Self {
        let working_dir = env::current_dir().unwrap_or_default();
        let mut session = Session::new(working_dir);
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
            "🚀 VORK - AI Coding Assistant".to_string()
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

        let mut app = Self {
            input: String::new(),
            messages: vec![],
            scroll: 0,
            session,
            client,
            approval_system,
            status: format!("Connected to {} | Model: {}{}", server_url, model, agent_info),
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

    async fn send_message(&mut self) -> Result<()> {
        if self.input.trim().is_empty() {
            return Ok(());
        }

        let user_message = self.input.clone();

        // Auto-select agent based on first message if no agent was explicitly set
        if self.first_message && !self.agent_explicitly_set {
            if let Ok(Some(agent)) = Agent::auto_select(&user_message) {
                // Update session with agent's system prompt
                self.session.conversation.messages[0].content = agent.system_prompt.clone();

                // Update UI with agent's color and title
                self.agent_color = parse_color(&agent.color);
                self.header_title = agent.title.clone().unwrap_or_else(|| format!("🤖 {}", agent.name));

                // Show agent selection message
                self.messages.push((
                    "system".to_string(),
                    format!("🎯 Auto-selected agent: {} - {}", agent.name, agent.description),
                ));
            }
            self.first_message = false;
        }

        self.messages.push(("user".to_string(), user_message.clone()));

        // Add to input history
        self.input_history.push(user_message.clone());
        self.history_index = None;
        self.current_input_backup.clear();

        self.input.clear();
        self.processing = true;

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
                // Filter out slot access messages
                let filtered_content: String = content
                    .lines()
                    .filter(|line| !line.contains("slot") && !line.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");

                if !filtered_content.is_empty() {
                    self.messages
                        .push(("assistant".to_string(), filtered_content.clone()));
                    self.session
                        .conversation
                        .add_assistant_message(filtered_content.clone());
                    total_tokens += filtered_content.len() / 4; // Rough estimate
                }
            }

            break;
        }

        // Calculate tokens/second
        let elapsed = start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 && total_tokens > 0 {
            self.tokens_per_second = total_tokens as f64 / elapsed;
        }
        self.tokens_used += total_tokens;

        self.session.save()?;
        self.processing = false;
        self.status = format!(
            "Session: {} | Messages: {} | Tokens: {}",
            self.session.id,
            self.messages.len(),
            self.tokens_used
        );

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
            }
            Some(index) => {
                if index > 0 {
                    self.history_index = Some(index - 1);
                    self.input = self.input_history[self.history_index.unwrap()].clone();
                }
            }
        }
    }

    fn history_next(&mut self) {
        if let Some(index) = self.history_index {
            if index < self.input_history.len() - 1 {
                self.history_index = Some(index + 1);
                self.input = self.input_history[self.history_index.unwrap()].clone();
            } else {
                // Reached the end, restore backup
                self.history_index = None;
                self.input = self.current_input_backup.clone();
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
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Update spinner animation when processing
        if app.processing {
            app.spinner_state = (app.spinner_state + 1) % 10;
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
                            if !app.processing {
                                if app.input.trim() == "exit" || app.input.trim() == "quit" {
                                    return Ok(());
                                }
                                app.send_message().await?;
                            }
                        }
                        KeyCode::Up => {
                            if !app.processing {
                                // Navigate to previous command in history
                                app.history_prev();
                            }
                        }
                        KeyCode::Down => {
                            if !app.processing {
                                // Navigate to next command in history
                                app.history_next();
                            }
                        }
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {
                    // Handle terminal resize by redrawing
                    terminal.autoresize()?;
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Header
            Constraint::Min(5),          // Messages (reduced from 10 for better resize)
            Constraint::Length(3),       // Input
            Constraint::Length(3),       // Status
        ])
        .split(size);

    // Header with agent-specific color and title
    let header = Paragraph::new(app.header_title.clone())
        .style(
            Style::default()
                .fg(app.agent_color)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Messages
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
                "assistant" => "🤖 Assistant",
                "tool" => "🔧 Tool",
                "tool_result" => "📄 Result",
                "error" => "❌ Error",
                "system" => "ℹ️  System",
                _ => role,
            };

            let lines: Vec<Line> = content
                .lines()
                .map(|line| {
                    Line::from(vec![
                        Span::styled(
                            format!("{}: ", prefix),
                            style.add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(line, style),
                    ])
                })
                .collect();

            ListItem::new(lines)
        })
        .collect();

    let messages_widget = List::new(messages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Conversation"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(messages_widget, chunks[1]);

    // Input with animated spinner and clear status
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let (input_text, input_style, input_title) = if app.processing {
        (
            format!("{} AI is thinking...", spinner_frames[app.spinner_state]),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            "⏳ Processing... (please wait)",
        )
    } else {
        (
            format!("💬 {}", app.input),
            Style::default().fg(Color::White),
            "✅ Ready for input (Ctrl+C to exit)",
        )
    };

    let input = Paragraph::new(input_text)
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(input_title)
                .border_style(if app.processing {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }),
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
}
