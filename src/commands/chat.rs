use anyhow::{Context, Result};
use colored::Colorize;
use std::io::{self, Write};
use std::env;

use crate::config::Config;
use crate::llm::{LlamaClient, Session, ApprovalSystem};
use crate::llm::tools::{get_available_tools, execute_tool};

pub async fn execute(server_url: Option<String>, model: Option<String>, initial_prompt: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let server_url = server_url.unwrap_or_else(|| config.assistant.server_url.clone());
    let model = model.unwrap_or_else(|| config.assistant.model.clone());

    println!("{}", "=== Vork Chat - AI Coding Assistant ===".green().bold());
    println!("{} {}", "Server:".cyan(), server_url);
    println!("{} {}", "Model:".cyan(), model);
    println!("{} {:?}", "Sandbox:".cyan(), config.assistant.sandbox_mode);
    println!("{} {:?}", "Approval:".cyan(), config.assistant.approval_policy);
    println!("{}", "Type 'exit' or 'quit' to end the session".yellow());
    println!("{}", "Type 'clear' to start a new conversation".yellow());
    println!();

    let client = LlamaClient::new(server_url, model);
    let working_dir = env::current_dir()?;
    let mut session = Session::new(working_dir);
    let approval_system = ApprovalSystem::new(
        config.assistant.approval_policy.clone(),
        config.assistant.sandbox_mode.clone(),
    );

    // Handle initial prompt if provided
    if let Some(prompt) = initial_prompt {
        println!("{} {}", "You:".blue().bold(), prompt);
        session.conversation.add_user_message(prompt);

        // Process initial prompt
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

            if let Some(tool_calls) = &choice.message.tool_calls {
                for tool_call in tool_calls {
                    let tool_name = &tool_call.function.name;
                    let arguments: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                        .context("Failed to parse tool arguments")?;

                    println!("{} {} {}", "🔧".yellow(), "Executing:".yellow(), tool_name.yellow().bold());

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
                continue;
            }

            if let Some(content) = &choice.message.content {
                println!("{} {}", "Assistant:".green().bold(), content);
                session.conversation.add_assistant_message(content.clone());
            }

            break;
        }

        session.save()?;
        println!();
    }

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
                println!("{} Session saved as {}", "✓".green(), session.id);
                println!("{}", "Goodbye!".green());
                break;
            }
            "clear" => {
                let working_dir = env::current_dir()?;
                session = Session::new(working_dir);
                println!("{}", "Conversation cleared".yellow());
                continue;
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
                    let arguments: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                        .context("Failed to parse tool arguments")?;

                    println!("{} {} {}", "🔧".yellow(), "Executing:".yellow(), tool_name.yellow().bold());

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
