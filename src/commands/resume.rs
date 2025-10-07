use anyhow::{Context, Result};
use colored::Colorize;
use std::io::{self, Write};

use crate::config::Config;
use crate::llm::{LlamaClient, Session, ApprovalSystem};
use crate::llm::tools::{get_available_tools, execute_tool};

pub async fn execute(session_id: Option<String>, last: bool) -> Result<()> {
    let config = Config::load()?;

    let mut session = if last {
        Session::get_last_session()?
            .ok_or_else(|| anyhow::anyhow!("No previous sessions found"))?
    } else if let Some(id) = session_id {
        Session::load(&id)?
    } else {
        // List sessions and let user choose
        let sessions = Session::list_sessions()?;
        if sessions.is_empty() {
            anyhow::bail!("No sessions found");
        }

        println!("{}", "Available sessions:".cyan().bold());
        for (i, sess) in sessions.iter().enumerate() {
            println!(
                "{}. {} (updated: {})",
                i + 1,
                sess.id.yellow(),
                sess.updated_at.format("%Y-%m-%d %H:%M:%S")
            );
        }

        print!("\n{} ", "Select session number:".cyan());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let index: usize = input
            .trim()
            .parse()
            .context("Invalid session number")?;

        sessions
            .get(index - 1)
            .ok_or_else(|| anyhow::anyhow!("Invalid session number"))?
            .clone()
    };

    println!("{}", "=== Resuming Session ===".green().bold());
    println!("{} {}", "Session ID:".cyan(), session.id);
    println!("{} {}", "Working Dir:".cyan(), session.working_directory.display());
    println!();

    let client = LlamaClient::new(
        config.assistant.server_url.clone(),
        config.assistant.model.clone(),
    );
    let approval_system = ApprovalSystem::new(
        config.assistant.approval_policy.clone(),
        config.assistant.sandbox_mode.clone(),
    );

    // Continue conversation
    loop {
        print!("{} ", "You:".blue().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input.to_lowercase().as_str() {
            "exit" | "quit" => {
                session.save()?;
                println!("{} Session saved", "✓".green());
                println!("{}", "Goodbye!".green());
                break;
            }
            _ => {}
        }

        session.conversation.add_user_message(input.to_string());

        // Main loop: keep calling LLM until it stops requesting tool calls
        loop {
            let response = client
                .chat_completion(
                    session.conversation.get_messages(),
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
                // Execute each tool call
                for tool_call in tool_calls {
                    let tool_name = &tool_call.function.name;
                    let arguments: serde_json::Value =
                        serde_json::from_str(&tool_call.function.arguments)
                            .context("Failed to parse tool arguments")?;

                    println!(
                        "{} {} {}",
                        "🔧".yellow(),
                        "Executing:".yellow(),
                        tool_name.yellow().bold()
                    );

                    match execute_tool(tool_name, arguments, Some(&approval_system)).await {
                        Ok(result) => {
                            session.conversation.add_tool_result(tool_name, &result);
                        }
                        Err(e) => {
                            let error_msg = format!("Error: {}", e);
                            session.conversation.add_tool_result(tool_name, &error_msg);
                        }
                    }
                }

                // Continue the loop to let the LLM process tool results
                continue;
            }

            // If no tool calls, process the assistant's message
            if let Some(content) = &choice.message.content {
                println!("{} {}", "Assistant:".green().bold(), content);
                session.conversation.add_assistant_message(content.clone());
            }

            // Break the inner loop - wait for next user input
            break;
        }

        // Auto-save session after each exchange
        session.save()?;

        println!();
    }

    Ok(())
}
