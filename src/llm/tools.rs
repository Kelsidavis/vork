use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct Tool {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
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
        json!({
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "Search the web for information using DuckDuckGo. Returns summarized results with titles, URLs, and snippets.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        },
                        "max_results": {
                            "type": "number",
                            "description": "Maximum number of results to return (default: 5)"
                        }
                    },
                    "required": ["query"]
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
        "web_search" => {
            let query = arguments["query"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;
            let max_results = arguments["max_results"]
                .as_u64()
                .unwrap_or(5) as usize;

            eprintln!("🌐 Searching web for: {}", query);

            // Use DuckDuckGo HTML search (no API key needed)
            let search_url = format!(
                "https://html.duckduckgo.com/html/?q={}",
                urlencoding::encode(query)
            );

            let client = reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
                .build()?;

            let response = client
                .get(&search_url)
                .send()
                .await
                .context("Failed to fetch search results")?;

            let html = response.text().await?;

            // Parse results from HTML (simple parsing)
            let mut results = Vec::new();
            let lines: Vec<&str> = html.lines().collect();

            for i in 0..lines.len() {
                if lines[i].contains("result__a") && results.len() < max_results {
                    // Extract title
                    if let Some(title_start) = lines[i].find(">") {
                        if let Some(title_end) = lines[i][title_start..].find("</a>") {
                            let title = &lines[i][title_start + 1..title_start + title_end];
                            let title = html_escape::decode_html_entities(title);

                            // Extract URL
                            if let Some(url_start) = lines[i].find("href=\"") {
                                if let Some(url_end) = lines[i][url_start + 6..].find("\"") {
                                    let url = &lines[i][url_start + 6..url_start + 6 + url_end];

                                    // Find snippet in next few lines
                                    let mut snippet = String::new();
                                    for j in i+1..std::cmp::min(i+10, lines.len()) {
                                        if lines[j].contains("result__snippet") {
                                            if let Some(snip_start) = lines[j].find(">") {
                                                if let Some(snip_end) = lines[j][snip_start..].find("</") {
                                                    snippet = lines[j][snip_start + 1..snip_start + snip_end].to_string();
                                                    snippet = html_escape::decode_html_entities(&snippet).to_string();
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    results.push(format!(
                                        "Title: {}\nURL: {}\nSnippet: {}\n",
                                        title, url, snippet
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            if results.is_empty() {
                eprintln!("ℹ️  No search results found");
                Ok("No results found".to_string())
            } else {
                eprintln!("✅ Found {} search results", results.len());
                Ok(results.join("\n---\n\n"))
            }
        }
        _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
    }
}
