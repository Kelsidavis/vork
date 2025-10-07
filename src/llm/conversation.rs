use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::client::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub messages: Vec<Message>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: vec![Message {
                role: "system".to_string(),
                content: SYSTEM_PROMPT.to_string(),
            }],
        }
    }

    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(Message {
            role: "user".to_string(),
            content,
        });
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.messages.push(Message {
            role: "assistant".to_string(),
            content,
        });
    }

    pub fn add_tool_result(&mut self, tool_name: &str, result: &str) {
        // Add tool results as user messages since many models don't support "tool" role
        self.messages.push(Message {
            role: "user".to_string(),
            content: format!("Tool execution result:\nTool: {}\nResult:\n{}", tool_name, result),
        });
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &PathBuf) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let conversation = serde_json::from_str(&json)?;
        Ok(conversation)
    }

    pub fn get_messages(&self) -> Vec<Message> {
        self.messages.clone()
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

const SYSTEM_PROMPT: &str = r#"You are Vork, an AI coding assistant powered by a local LLM. Your purpose is to help with software development tasks.

You have access to the following tools:
- read_file: Read the contents of files
- write_file: Create or overwrite files with new content
- list_files: List files in a directory
- bash_exec: Execute bash commands
- search_files: Search for patterns in files using grep

When helping with code:
1. Always read existing files before modifying them
2. Provide clear explanations for your changes
3. Use bash_exec to run tests or check compilation
4. Be precise and avoid breaking existing functionality
5. When writing code, include proper error handling and documentation

When the user asks you to modify code:
1. First read the file to understand the current state
2. Make the changes carefully
3. Write the updated file
4. Optionally run tests to verify the changes

You should be proactive in using tools to help solve problems. Don't just suggest changes - actually make them using the available tools.
"#;
