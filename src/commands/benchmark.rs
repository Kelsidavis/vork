use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
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

    // Test prompts - Complex real-world tasks with specified length for fair comparison
    let test_cases = vec![
        (
            "Code Generation",
            "Write a complete Rust function that implements a thread-safe LRU cache with generic key/value types. Include:\n\
            - Proper struct definition with HashMap and linked list\n\
            - Methods: new(), get(), put(), capacity()\n\
            - Thread safety using Arc and Mutex\n\
            - Comprehensive inline documentation\n\
            Target: 300-400 words including code and explanations.",
        ),
        (
            "Bug Analysis & Fix",
            "Analyze and fix this Rust code:\n\
            ```rust\n\
            use std::collections::HashMap;\n\
            fn process_data(data: Vec<String>) -> HashMap<String, usize> {\n\
                let mut map = HashMap::new();\n\
                for item in data {\n\
                    let count = map.get(&item).unwrap();\n\
                    map.insert(item, count + 1);\n\
                }\n\
                map\n\
            }\n\
            ```\n\
            Provide:\n\
            - Detailed explanation of all bugs (race conditions, panics, logic errors)\n\
            - Complete corrected version with proper error handling\n\
            - Best practices commentary\n\
            Target: 350-450 words.",
        ),
        (
            "System Design & Implementation",
            "Design and implement a CLI tool in Rust that monitors system resources (CPU, memory, disk) and logs to a file when thresholds are exceeded. Include:\n\
            - Architecture overview with component breakdown\n\
            - Key data structures (Config, ResourceSnapshot, Alert)\n\
            - Core function signatures with detailed logic\n\
            - Error handling strategy\n\
            - Performance considerations\n\
            Target: 400-500 words with code snippets and technical reasoning.",
        ),
        (
            "Algorithm Optimization",
            "Given this naive string matching implementation:\n\
            ```rust\n\
            fn find_pattern(text: &str, pattern: &str) -> Vec<usize> {\n\
                let mut positions = Vec::new();\n\
                for i in 0..text.len() {\n\
                    if text[i..].starts_with(pattern) {\n\
                        positions.push(i);\n\
                    }\n\
                }\n\
                positions\n\
            }\n\
            ```\n\
            Provide:\n\
            - Time complexity analysis of current implementation\n\
            - Optimized version using Boyer-Moore or KMP algorithm\n\
            - Performance comparison with Big-O notation\n\
            - Benchmarking strategy\n\
            Target: 350-450 words with detailed explanations.",
        ),
        (
            "Refactoring & Architecture",
            "Refactor this monolithic function into clean, testable components:\n\
            ```rust\n\
            fn handle_request(req: String) -> String {\n\
                let parts: Vec<&str> = req.split('|').collect();\n\
                let cmd = parts[0];\n\
                if cmd == \"get\" {\n\
                    let id = parts[1].parse::<u32>().unwrap();\n\
                    format!(\"Result: {}\", id * 2)\n\
                } else if cmd == \"set\" {\n\
                    let id = parts[1].parse::<u32>().unwrap();\n\
                    let val = parts[2];\n\
                    format!(\"Stored: {} = {}\", id, val)\n\
                } else {\n\
                    \"Error\".to_string()\n\
                }\n\
            }\n\
            ```\n\
            Provide:\n\
            - Command pattern implementation with enums\n\
            - Proper error handling with Result types\n\
            - Unit test examples\n\
            - SOLID principles explanation\n\
            Target: 400-500 words.",
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
                    let tokens_per_sec = if estimated_tokens > 0 {
                        estimated_tokens as f64 / duration.as_secs_f64()
                    } else {
                        0.0
                    };

                    // Latency score: ms per token (lower is better)
                    let latency_score = if estimated_tokens > 0 {
                        duration.as_millis() as f64 / estimated_tokens as f64
                    } else {
                        999999.0
                    };

                    println!("      {} {:.1} tok/s (~{} tokens, {:.1}s, {:.1}ms/tok)",
                        "✓".green(),
                        tokens_per_sec,
                        estimated_tokens,
                        duration.as_secs_f64(),
                        latency_score
                    );

                    preset_results.test_results.push(TestResult {
                        test_name: test_name.to_string(),
                        tokens_per_second: tokens_per_sec,
                        total_tokens: estimated_tokens,
                        duration,
                        response_preview: truncate_string(content, 100),
                        latency_score,
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
                        latency_score: 999999.0,
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

    // Save benchmark results for agent allocation
    save_benchmark_results(&results)?;

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
    latency_score: f64,  // Lower is better: duration_ms / tokens
}

fn print_summary(results: &[PresetBenchmark]) {
    println!();
    println!("{}", "=== BENCHMARK SUMMARY ===".green().bold());
    println!();

    // Table header
    println!("{:<25} {:<12} {:<15} {:<18} {:<15}",
        "Preset".cyan().bold(),
        "Context".cyan().bold(),
        "Avg Speed".cyan().bold(),
        "Latency".cyan().bold(),
        "Use Case".cyan().bold()
    );
    println!("{}", "─".repeat(95).cyan());

    for preset in results {
        let avg_speed: f64 = preset.test_results.iter()
            .map(|r| r.tokens_per_second)
            .sum::<f64>() / preset.test_results.len() as f64;

        let avg_latency: f64 = preset.test_results.iter()
            .filter(|r| r.latency_score < 999999.0)
            .map(|r| r.latency_score)
            .sum::<f64>() / preset.test_results.iter()
            .filter(|r| r.latency_score < 999999.0)
            .count() as f64;

        let use_case = match preset.name.as_str() {
            n if n.contains("fast") => "Speed priority",
            n if n.contains("balanced") => "Balanced",
            n if n.contains("extended") => "Max context",
            _ => "General purpose",
        };

        let ctx_display = format!("{}k", preset.context_size / 1024);

        println!("{:<25} {:<12} {:<15} {:<18} {:<15}",
            preset.name.green(),
            ctx_display.yellow(),
            format!("{:.1} tok/s", avg_speed).cyan(),
            format!("{:.1} ms/tok", avg_latency).magenta(),
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
                println!("     Speed: {:.1} tok/s ({:.1} ms/tok)",
                    test.tokens_per_second, test.latency_score);
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

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkResults {
    timestamp: String,
    fastest_preset: String,
    largest_context_preset: String,
    best_reasoning_preset: String,
    presets: Vec<PresetStats>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PresetStats {
    name: String,
    avg_tokens_per_second: f64,
    context_size: u32,
}

fn save_benchmark_results(results: &[PresetBenchmark]) -> Result<()> {
    use chrono::Local;

    // Find fastest preset (lowest latency = fastest per-token generation)
    let fastest = results.iter()
        .min_by(|a, b| {
            let avg_lat_a = a.test_results.iter()
                .filter(|r| r.latency_score < 999999.0)
                .map(|r| r.latency_score)
                .sum::<f64>() / a.test_results.iter().filter(|r| r.latency_score < 999999.0).count() as f64;
            let avg_lat_b = b.test_results.iter()
                .filter(|r| r.latency_score < 999999.0)
                .map(|r| r.latency_score)
                .sum::<f64>() / b.test_results.iter().filter(|r| r.latency_score < 999999.0).count() as f64;
            avg_lat_a.partial_cmp(&avg_lat_b).unwrap()
        })
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "qwen3-30b-fast".to_string());

    // Find largest context
    let largest_context = results.iter()
        .max_by_key(|r| r.context_size)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "qwen3-30b-extended".to_string());

    // Find best for reasoning (30B model or fallback to fastest)
    let best_reasoning = results.iter()
        .find(|r| r.name.contains("30b"))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| fastest.clone());

    let preset_stats: Vec<PresetStats> = results.iter()
        .map(|p| {
            let avg_speed = p.test_results.iter().map(|r| r.tokens_per_second).sum::<f64>()
                / p.test_results.len() as f64;
            PresetStats {
                name: p.name.clone(),
                avg_tokens_per_second: avg_speed,
                context_size: p.context_size,
            }
        })
        .collect();

    let benchmark_results = BenchmarkResults {
        timestamp: Local::now().to_rfc3339(),
        fastest_preset: fastest,
        largest_context_preset: largest_context,
        best_reasoning_preset: best_reasoning,
        presets: preset_stats,
    };

    // Save to config directory
    let config_dir = Config::config_dir()?;
    let benchmark_path = config_dir.join("benchmark_results.json");
    let json = serde_json::to_string_pretty(&benchmark_results)?;
    std::fs::write(&benchmark_path, json)?;

    println!();
    println!("{}", "💾 Benchmark results saved!".green().bold());
    println!("   Path: {}", benchmark_path.display().to_string().yellow());
    println!();
    println!("{}", "🤖 Updating agent preset assignments...".cyan().bold());

    // Update default agents with benchmark results
    update_agent_presets(&benchmark_results)?;

    Ok(())
}

fn update_agent_presets(results: &BenchmarkResults) -> Result<()> {
    use crate::agents::Agent;

    // Define agent-to-preset mapping based on benchmark results
    let agent_mappings = vec![
        // Fast, simple tasks - use fastest model
        ("default", results.fastest_preset.as_str()),
        ("code-editor", results.fastest_preset.as_str()),
        ("debugger", results.fastest_preset.as_str()),
        ("documenter", results.fastest_preset.as_str()),

        // Large context needs - use largest context even if slower
        ("reviewer", results.largest_context_preset.as_str()),
        ("code-auditor", results.largest_context_preset.as_str()),

        // Complex reasoning - use best reasoning (usually 30B)
        ("researcher", results.best_reasoning_preset.as_str()),
        ("reverse-engineer", results.best_reasoning_preset.as_str()),
        ("rust-expert", results.best_reasoning_preset.as_str()),
        ("security-auditor", results.best_reasoning_preset.as_str()),
        ("performance-optimizer", results.best_reasoning_preset.as_str()),
        ("test-writer", results.best_reasoning_preset.as_str()),
    ];

    let mut updated_count = 0;
    for (agent_name, preset_name) in agent_mappings {
        if let Ok(mut agent) = Agent::load(agent_name) {
            agent.preferred_preset = Some(preset_name.to_string());
            if agent.save().is_ok() {
                println!("   ✓ {} → {}", agent_name.green(), preset_name.yellow());
                updated_count += 1;
            }
        }
    }

    println!();
    println!("{} {} agents updated with optimal presets",
        "✓".green(), updated_count);

    Ok(())
}
