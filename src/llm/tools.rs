use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub output: String,
}

pub fn get_available_tools() -> Vec<serde_json::Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file (creates or overwrites)",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "The content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files in a directory",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The directory path to list (default: current directory)"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "bash_exec",
                "description": "Execute a bash command and return the output",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The bash command to execute"
                        }
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_files",
                "description": "Search for a pattern in files using grep",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "The pattern to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "The path to search in (default: current directory)"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        }),
    ]
}

pub async fn execute_tool(
    name: &str,
    arguments: serde_json::Value,
    approval_system: Option<&super::approval::ApprovalSystem>,
) -> Result<String> {
    match name {
        "read_file" => {
            let path = arguments["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

            eprintln!("📖 Reading file: {}", path);

            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read file: {}", path))?;

            Ok(content)
        }
        "write_file" => {
            let path = arguments["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;
            let content = arguments["content"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

            eprintln!("✏️  Writing file: {}", path);

            // Check approval
            if let Some(approval) = approval_system {
                if !approval.should_approve_write(path)? {
                    return Ok(format!("Write to {} was denied by user", path));
                }
            }

            // Create parent directories if they don't exist
            if let Some(parent) = std::path::Path::new(path).parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create parent directories for: {}", path))?;
            }

            fs::write(path, content)
                .with_context(|| format!("Failed to write file: {}", path))?;

            eprintln!("✅ Wrote {} bytes to {}", content.len(), path);
            Ok(format!("Successfully wrote {} bytes to {}", content.len(), path))
        }
        "list_files" => {
            let path = arguments["path"]
                .as_str()
                .unwrap_or(".");

            eprintln!("📁 Listing directory: {}", path);

            let entries = fs::read_dir(path)
                .with_context(|| format!("Failed to read directory: {}", path))?;

            let mut files = vec![];
            for entry in entries {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                let file_type = if entry.path().is_dir() { "/" } else { "" };
                files.push(format!("{}{}", name, file_type));
            }

            eprintln!("✅ Found {} items in {}", files.len(), path);
            Ok(files.join("\n"))
        }
        "bash_exec" => {
            let command = arguments["command"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

            eprintln!("⚡ Executing bash command: {}", command);

            // Check approval
            if let Some(approval) = approval_system {
                if !approval.should_approve_bash(command)? {
                    return Ok(format!("Command '{}' was denied by user", command));
                }
            }

            let output = Command::new("bash")
                .arg("-c")
                .arg(command)
                .output()
                .with_context(|| format!("Failed to execute command: {}", command))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let exit_code = output.status.code().unwrap_or(-1);
            if exit_code == 0 {
                eprintln!("✅ Command completed successfully");
            } else {
                eprintln!("⚠️  Command exited with code {}", exit_code);
            }

            Ok(format!(
                "Exit code: {}\n\nStdout:\n{}\n\nStderr:\n{}",
                exit_code,
                stdout,
                stderr
            ))
        }
        "search_files" => {
            let pattern = arguments["pattern"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?;
            let path = arguments["path"]
                .as_str()
                .unwrap_or(".");

            eprintln!("🔍 Searching for '{}' in {}", pattern, path);

            let output = Command::new("grep")
                .arg("-r")
                .arg("-n")
                .arg(pattern)
                .arg(path)
                .output()
                .with_context(|| format!("Failed to search for pattern: {}", pattern))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let line_count = stdout.lines().count();

            if line_count > 0 {
                eprintln!("✅ Found {} matches", line_count);
            } else {
                eprintln!("ℹ️  No matches found");
            }

            Ok(stdout.to_string())
        }
        _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
    }
}
