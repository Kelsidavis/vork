use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::client::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub messages: Vec<Message>,
    #[serde(skip)]
    pub estimated_tokens: usize,
    #[serde(skip)]
    pub max_context: usize,
}

impl Conversation {
    pub fn new() -> Self {
        let system_message = Message {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
        };
        let estimated_tokens = estimate_tokens(&system_message.content);

        Self {
            messages: vec![system_message],
            estimated_tokens,
            max_context: 32768, // Default, will be overridden
        }
    }

    pub fn set_max_context(&mut self, max_context: usize) {
        self.max_context = max_context;
    }

    pub fn get_context_usage(&self) -> (usize, usize, f32) {
        // Returns (used, max, percentage)
        let percentage = (self.estimated_tokens as f32 / self.max_context as f32) * 100.0;
        (self.estimated_tokens, self.max_context, percentage)
    }

    pub fn add_user_message(&mut self, content: String) {
        self.estimated_tokens += estimate_tokens(&content);
        self.messages.push(Message {
            role: "user".to_string(),
            content,
        });
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.estimated_tokens += estimate_tokens(&content);
        self.messages.push(Message {
            role: "assistant".to_string(),
            content,
        });
    }

    pub fn add_tool_result(&mut self, tool_name: &str, result: &str) {
        // Add tool results as user messages since many models don't support "tool" role
        let content = format!("Tool execution result:\nTool: {}\nResult:\n{}", tool_name, result);
        self.estimated_tokens += estimate_tokens(&content);
        self.messages.push(Message {
            role: "user".to_string(),
            content,
        });
    }

    /// Check if compaction is needed (at 75% capacity)
    pub fn needs_compaction(&self) -> bool {
        self.estimated_tokens > (self.max_context * 3 / 4)
    }

    /// Compact the conversation by summarizing older messages
    /// Returns true if compaction occurred, false otherwise
    pub async fn compact_if_needed(&mut self, client: &super::client::LlamaClient) -> Result<bool> {
        if !self.needs_compaction() {
            return Ok(false);
        }

        // Keep system prompt (index 0) and last 10 messages
        // Summarize everything in between
        if self.messages.len() <= 11 {
            // Not enough to compact
            return Ok(false);
        }

        let system_msg = self.messages[0].clone();
        let messages_to_compact: Vec<_> = self.messages.iter()
            .skip(1)
            .take(self.messages.len() - 11)
            .cloned()
            .collect();
        let recent_messages: Vec<_> = self.messages.iter()
            .skip(self.messages.len() - 10)
            .cloned()
            .collect();

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
        let response = client.chat_completion(vec![
            Message {
                role: "user".to_string(),
                content: summary_prompt,
            }
        ], None).await?;

        let summary_response = response.choices[0].message.content.clone()
            .unwrap_or_default();

        // Rebuild conversation with summary
        let summary_msg = Message {
            role: "assistant".to_string(),
            content: format!("[Conversation summary of {} messages]\n\n{}",
                messages_to_compact.len(), summary_response),
        };

        // Recalculate tokens
        self.estimated_tokens = estimate_tokens(&system_msg.content);
        self.estimated_tokens += estimate_tokens(&summary_msg.content);
        for msg in &recent_messages {
            self.estimated_tokens += estimate_tokens(&msg.content);
        }

        // Rebuild messages
        self.messages = vec![system_msg, summary_msg];
        self.messages.extend(recent_messages);

        Ok(true)
    }

    #[allow(dead_code)]
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    #[allow(dead_code)]
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
- analyze_image: Analyze images (PNG, JPG, GIF, BMP, WebP) - describe contents, read text, analyze UI

CRITICAL: All user requests are WORKSPACE-RELATIVE by default.
- When user says "put it in /docs/", they mean "./docs/" (relative to current workspace)
- When user says "create /src/file.rs", they mean "./src/file.rs" (in the current directory)
- Only use absolute paths starting with / if the user EXPLICITLY mentions system directories like /usr/, /etc/, /home/username/
- Always interpret paths as relative to the current working directory unless clearly absolute
- If user says "document this in /docs/api/", create "./docs/api/" in the workspace

Examples:
- User: "put docs in /docs/" → Create "./docs/" (workspace-relative)
- User: "save to /home/user/backup/" → Use "/home/user/backup/" (absolute, as stated)
- User: "create /api/handlers.rs" → Create "./api/handlers.rs" (workspace-relative)

When helping with code:
1. Always read existing files before modifying them
2. Provide clear explanations for your changes
3. Use bash_exec to run tests or check compilation
4. Be precise and avoid breaking existing functionality
5. When writing code, include proper error handling and documentation
6. REMEMBER: Paths are workspace-relative unless explicitly absolute system paths

When the user asks you to modify code:
1. First read the file to understand the current state
2. Make the changes carefully
3. Write the updated file
4. Optionally run tests to verify the changes

You should be proactive in using tools to help solve problems. Don't just suggest changes - actually make them using the available tools.
"#;

/// Estimate token count (rough approximation: 1 token ≈ 4 characters)
fn estimate_tokens(text: &str) -> usize {
    // More accurate estimation considering:
    // - ~4 chars per token on average
    // - Extra tokens for formatting, role markers, etc.
    (text.len() / 4) + 10
}
