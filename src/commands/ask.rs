use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::Config;
use crate::llm::{LlamaClient, Conversation, ApprovalSystem};
use crate::llm::tools::{get_available_tools, execute_tool};

pub async fn execute(
    question: &str,
    server_url: Option<String>,
    model: Option<String>,
    no_tools: bool,
) -> Result<()> {
    let config = Config::load()?;
    let server_url = server_url.unwrap_or_else(|| config.assistant.server_url.clone());
    let model = model.unwrap_or_else(|| config.assistant.model.clone());

    let client = LlamaClient::new(server_url, model);
    let mut conversation = Conversation::new();
    let approval_system = ApprovalSystem::new(
        config.assistant.approval_policy.clone(),
        config.assistant.sandbox_mode.clone(),
    );

    conversation.add_user_message(question.to_string());

    // Main loop: keep calling LLM until it stops requesting tool calls
    loop {
        let tools = if no_tools {
            None
        } else {
            Some(get_available_tools())
        };

        let response = client
            .chat_completion(conversation.get_messages(), tools)
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
                        conversation.add_tool_result(tool_name, &result);
                    }
                    Err(e) => {
                        let error_msg = format!("Error: {}", e);
                        conversation.add_tool_result(tool_name, &error_msg);
                    }
                }
            }

            // Continue the loop to let the LLM process tool results
            continue;
        }

        // If no tool calls, print the assistant's message and exit
        if let Some(content) = &choice.message.content {
            println!("{}", content);
        }

        break;
    }

    Ok(())
}
