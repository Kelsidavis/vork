use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::process::Command;
use base64::{Engine as _, engine::general_purpose};

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
        json!({
            "type": "function",
            "function": {
                "name": "analyze_image",
                "description": "Analyze an image file and describe its contents. Supports common formats: PNG, JPG, JPEG, GIF, BMP, WebP. Returns base64-encoded image data for vision-capable models.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the image file to analyze"
                        },
                        "question": {
                            "type": "string",
                            "description": "Optional specific question about the image (e.g., 'What text is visible?', 'Describe the UI layout')"
                        }
                    },
                    "required": ["path"]
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

            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read file: {}", path))?;

            let line_count = content.lines().count();
            Ok(format!("📖 Read {} lines from {}\n\n{}", line_count, path, content))
        }
        "write_file" => {
            let path = arguments["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;
            let content = arguments["content"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

            // Check approval
            if let Some(approval) = approval_system {
                if !approval.should_approve_write(path)? {
                    return Ok(format!("❌ Write to {} was denied by user", path));
                }
            }

            // Create parent directories if they don't exist
            if let Some(parent) = std::path::Path::new(path).parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create parent directories for: {}", path))?;
            }

            fs::write(path, content)
                .with_context(|| format!("Failed to write file: {}", path))?;

            let line_count = content.lines().count();
            Ok(format!("✅ Wrote {} bytes ({} lines) to {}", content.len(), line_count, path))
        }
        "list_files" => {
            let path = arguments["path"]
                .as_str()
                .unwrap_or(".");

            let entries = fs::read_dir(path)
                .with_context(|| format!("Failed to read directory: {}", path))?;

            let mut files = vec![];
            for entry in entries {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                let file_type = if entry.path().is_dir() { "/" } else { "" };
                files.push(format!("{}{}", name, file_type));
            }

            Ok(format!("📁 Found {} items in {}:\n\n{}", files.len(), path, files.join("\n")))
        }
        "bash_exec" => {
            let command = arguments["command"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

            // Check approval
            if let Some(approval) = approval_system {
                if !approval.should_approve_bash(command)? {
                    return Ok(format!("❌ Command '{}' was denied by user", command));
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
            let status_icon = if exit_code == 0 { "✅" } else { "⚠️" };

            Ok(format!(
                "{} Executed: {}\nExit code: {}\n\nStdout:\n{}\n\nStderr:\n{}",
                status_icon,
                command,
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
                Ok(format!("🔍 Found {} matches for '{}' in {}:\n\n{}", line_count, pattern, path, stdout))
            } else {
                Ok(format!("ℹ️  No matches found for '{}' in {}", pattern, path))
            }
        }
        "web_search" => {
            let query = arguments["query"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;
            let max_results = arguments["max_results"]
                .as_u64()
                .unwrap_or(5) as usize;

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
                Ok(format!("ℹ️  No search results found for '{}'", query))
            } else {
                Ok(format!("🌐 Found {} search results for '{}':\n\n{}", results.len(), query, results.join("\n---\n\n")))
            }
        }
        "analyze_image" => {
            let path = arguments["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;
            let question = arguments["question"]
                .as_str()
                .map(|s| s.to_string());

            // Read image file
            let image_data = fs::read(path)
                .with_context(|| format!("Failed to read image file: {}", path))?;

            // Detect image format from extension
            let extension = std::path::Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let mime_type = match extension.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "bmp" => "image/bmp",
                "webp" => "image/webp",
                _ => "image/png", // default
            };

            // Encode to base64
            let base64_data = general_purpose::STANDARD.encode(&image_data);

            // Create data URL
            let data_url = format!("data:{};base64,{}", mime_type, base64_data);

            let size_kb = image_data.len() / 1024;
            let question_text = question.as_deref().unwrap_or("Please describe what you see in this image");

            // Return formatted response with image data and context
            Ok(format!(
                "🖼️  Loaded image: {} ({} KB, {})\n\nQuestion: {}\n\n[IMAGE_DATA: {}]\n\nNote: This image has been loaded and encoded. If your model supports vision, it will analyze the image based on the question.",
                path,
                size_kb,
                mime_type,
                question_text,
                data_url
            ))
        }
        _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
    }
}
