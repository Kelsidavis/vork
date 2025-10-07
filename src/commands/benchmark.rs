use anyhow::Result;
use colored::Colorize;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use crate::config::Config;
use crate::llm::LlamaClient;
use crate::llm::client::Message;

pub async fn execute() -> Result<()> {
    println!("{}", "=== Vork Model Benchmark ===" .green().bold());
    println!();

    // Get all available presets
    let config_dir = Config::config_dir()?;
    let presets_dir = config_dir.join("presets");

    let mut presets = vec![];
    if presets_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&presets_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.path().file_stem() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("toml")
                        && name != "README" {
                        presets.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    presets.sort();

    if presets.is_empty() {
        println!("{}", "No presets found in presets directory".yellow());
        return Ok(());
    }

    println!("{}", "Available presets:".cyan().bold());
    for (i, preset) in presets.iter().enumerate() {
        println!("  {}. {}", i + 1, preset.green());
    }
    println!();

    // Test prompts covering different use cases
    let test_cases = vec![
        (
            "Simple Code Generation",
            "Write a Python function to calculate fibonacci numbers recursively. Just code, no explanation.",
        ),
        (
            "Code Review",
            "Review this code for bugs:\n```python\ndef divide(a, b):\n    return a / b\n```",
        ),
        (
            "Complex Reasoning",
            "Explain the difference between async/await and threads in Rust. Give a concrete example of when to use each.",
        ),
    ];

    let mut results = Vec::new();

    for preset_name in &presets {
        println!("{}", format!("=== Testing: {} ===", preset_name).cyan().bold());

        // Load preset config
        let preset_path = presets_dir.join(format!("{}.toml", preset_name));
        let preset_content = std::fs::read_to_string(&preset_path)?;
        let preset_config: Config = toml::from_str(&preset_content)?;

        // Copy to main config
        let config_path = Config::config_path()?;
        std::fs::copy(&preset_path, &config_path)?;

        println!("{}", "  Restarting server with preset...".yellow());

        // Kill existing server
        let _ = std::process::Command::new("pkill")
            .arg("llama-server")
            .output();
        sleep(Duration::from_secs(2)).await;

        // Start new server
        crate::backends::llamacpp::LlamaCppBackend::start_server(8080)?;
        sleep(Duration::from_secs(5)).await; // Give more time for server to fully initialize

        // Create client
        let client = LlamaClient::new(
            preset_config.assistant.server_url.clone(),
            preset_config.assistant.model.clone(),
        );

        // Wait for server to be ready - try a simple test message
        println!("{}", "  Waiting for server to be ready...".yellow());
        let mut ready = false;
        for _ in 0..30 {
            let test_messages = vec![
                Message {
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                },
            ];
            if client.chat_completion(test_messages, None).await.is_ok() {
                ready = true;
                break;
            }
            sleep(Duration::from_millis(500)).await;
        }

        if !ready {
            println!("{}", "  ⚠️  Server not ready, skipping...".red());
            continue;
        }

        println!("{}", "  ✓ Server ready".green());
        println!();

        let mut preset_results = PresetBenchmark {
            name: preset_name.clone(),
            model: preset_config.assistant.model.clone(),
            context_size: preset_config.llamacpp.context_size,
            test_results: Vec::new(),
        };

        // Run each test case
        for (test_name, prompt) in &test_cases {
            println!("    Testing: {}", test_name.cyan());

            let messages = vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a helpful coding assistant. Be concise.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ];

            let start = Instant::now();

            match client.chat_completion(messages, None).await {
                Ok(response) => {
                    let duration = start.elapsed();

                    // Extract response content
                    let content = response.choices.first()
                        .and_then(|c| c.message.content.as_ref())
                        .map(|s| s.as_str())
                        .unwrap_or("");

                    // Estimate tokens (rough approximation: ~4 chars per token)
                    let estimated_tokens = content.len() / 4;
                    let tokens_per_sec = estimated_tokens as f64 / duration.as_secs_f64();

                    println!("      {} {:.1} tok/s (~{} tokens in {:.1}s)",
                        "✓".green(),
                        tokens_per_sec,
                        estimated_tokens,
                        duration.as_secs_f64()
                    );

                    preset_results.test_results.push(TestResult {
                        test_name: test_name.to_string(),
                        tokens_per_second: tokens_per_sec,
                        total_tokens: estimated_tokens,
                        duration,
                        response_preview: truncate_string(content, 100),
                    });
                }
                Err(e) => {
                    println!("      {} Failed: {}", "✗".red(), e);
                    preset_results.test_results.push(TestResult {
                        test_name: test_name.to_string(),
                        tokens_per_second: 0.0,
                        total_tokens: 0,
                        duration: Duration::from_secs(0),
                        response_preview: format!("Error: {}", e),
                    });
                }
            }

            // Small delay between tests
            sleep(Duration::from_millis(500)).await;
        }

        results.push(preset_results);
        println!();
    }

    // Print summary report
    print_summary(&results);

    Ok(())
}

struct PresetBenchmark {
    name: String,
    model: String,
    context_size: u32,
    test_results: Vec<TestResult>,
}

struct TestResult {
    test_name: String,
    tokens_per_second: f64,
    total_tokens: usize,
    duration: Duration,
    response_preview: String,
}

fn print_summary(results: &[PresetBenchmark]) {
    println!();
    println!("{}", "=== BENCHMARK SUMMARY ===".green().bold());
    println!();

    // Table header
    println!("{:<25} {:<12} {:<15} {:<15}",
        "Preset".cyan().bold(),
        "Context".cyan().bold(),
        "Avg Speed".cyan().bold(),
        "Use Case".cyan().bold()
    );
    println!("{}", "─".repeat(80).cyan());

    for preset in results {
        let avg_speed: f64 = preset.test_results.iter()
            .map(|r| r.tokens_per_second)
            .sum::<f64>() / preset.test_results.len() as f64;

        let use_case = match preset.name.as_str() {
            n if n.contains("instant") => "Fast responses, simple tasks",
            n if n.contains("large-context") => "Large files, code review",
            n if n.contains("30b") || n.contains("max") => "Complex reasoning, research",
            _ => "General purpose",
        };

        let ctx_display = format!("{}k", preset.context_size / 1024);

        println!("{:<25} {:<12} {:<15} {:<15}",
            preset.name.green(),
            ctx_display.yellow(),
            format!("{:.1} tok/s", avg_speed).cyan(),
            use_case
        );
    }

    println!();
    println!("{}", "=== DETAILED RESULTS ===".cyan().bold());
    println!();

    for preset in results {
        println!("{}", format!("📊 {}", preset.name).green().bold());
        println!("   Model: {}", preset.model.yellow());
        println!("   Context: {}k tokens", preset.context_size / 1024);
        println!();

        for test in &preset.test_results {
            if test.tokens_per_second > 0.0 {
                println!("   {} {}", "•".cyan(), test.test_name.bold());
                println!("     Speed: {:.1} tok/s", test.tokens_per_second);
                println!("     Time: {:.2}s for {} tokens",
                    test.duration.as_secs_f64(), test.total_tokens);
                println!("     Preview: {}",
                    test.response_preview.trim().replace('\n', " "));
            } else {
                println!("   {} {} - {}", "✗".red(), test.test_name, test.response_preview);
            }
            println!();
        }
    }

    println!("{}", "=== RECOMMENDATIONS ===".green().bold());
    println!();

    // Find fastest and best for different use cases
    if let Some(fastest) = results.iter().max_by(|a, b| {
        let avg_a = a.test_results.iter().map(|r| r.tokens_per_second).sum::<f64>() / a.test_results.len() as f64;
        let avg_b = b.test_results.iter().map(|r| r.tokens_per_second).sum::<f64>() / b.test_results.len() as f64;
        avg_a.partial_cmp(&avg_b).unwrap()
    }) {
        println!("⚡ {} for quick responses and simple tasks", fastest.name.green().bold());
    }

    if let Some(largest_ctx) = results.iter().max_by_key(|r| r.context_size) {
        println!("📄 {} for large files and code review ({}k context)",
            largest_ctx.name.green().bold(), largest_ctx.context_size / 1024);
    }

    if let Some(largest_model) = results.iter().max_by(|a, b| {
        let size_a = if a.name.contains("30") { 30 } else { 14 };
        let size_b = if b.name.contains("30") { 30 } else { 14 };
        size_a.cmp(&size_b)
    }) {
        if largest_model.name.contains("30") {
            println!("🧠 {} for complex reasoning and research",
                largest_model.name.green().bold());
        }
    }

    println!();
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
