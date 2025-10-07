use anyhow::{Context, Result};
use colored::Colorize;
use std::env;

use crate::config::{Config, ApprovalPolicy, SandboxMode};
use crate::llm::{LlamaClient, Session, ApprovalSystem};
use crate::llm::tools::{get_available_tools, execute_tool};

pub async fn execute(
    prompt: &str,
    server_url: Option<String>,
    model: Option<String>,
    full_auto: bool,
    json_output: bool,
) -> Result<()> {
    let mut config = Config::load()?;
    let server_url = server_url.unwrap_or_else(|| config.assistant.server_url.clone());
    let model = model.unwrap_or_else(|| config.assistant.model.clone());

    // In exec mode, default to read-only unless --full-auto is specified
    if full_auto {
        config.assistant.sandbox_mode = SandboxMode::DangerFullAccess;
        config.assistant.approval_policy = ApprovalPolicy::Never;
    } else {
        config.assistant.sandbox_mode = SandboxMode::ReadOnly;
    }

    let client = LlamaClient::new(server_url, model);
    let working_dir = env::current_dir()?;
    let mut session = Session::new(working_dir);
    let approval_system = ApprovalSystem::new(
        config.assistant.approval_policy.clone(),
        config.assistant.sandbox_mode.clone(),
    );

    session.conversation.add_user_message(prompt.to_string());

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

                if !json_output {
                    eprintln!(
                        "{} {} {}",
                        "🔧".yellow(),
                        "Executing:".yellow(),
                        tool_name.yellow().bold()
                    );
                }

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

        // If no tool calls, output the assistant's message and exit
        if let Some(content) = &choice.message.content {
            if json_output {
                let output = serde_json::json!({
                    "session_id": session.id,
                    "message": content,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("{}", content);
            }

            session.conversation.add_assistant_message(content.clone());
        }

        break;
    }

    // Save session for potential resume
    session.save()?;

    if !json_output {
        eprintln!("{} Session saved as {}", "✓".green(), session.id);
    }

    Ok(())
}
