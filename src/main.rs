mod client;
mod config;

use anyhow::{bail, Result};
use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use std::process::Command;

use client::{Client, GrafanaClient};
use config::Config;

#[derive(Parser)]
#[command(name = "groundcover-cli")]
#[command(about = "Groundcover CLI - query logs, traces, and metrics")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure API key
    Config {
        /// Set API key directly
        #[arg(long)]
        api_key: Option<String>,
        /// Fetch API key from groundcover CLI
        #[arg(long)]
        fetch: bool,
    },
    /// Query logs from ClickHouse
    Logs {
        /// Time range (e.g., 15m, 1h, 24h)
        #[arg(long, short, default_value = "15m")]
        since: String,
        /// Filter by service/workload name
        #[arg(long, short = 'w')]
        workload: Option<String>,
        /// Filter by namespace
        #[arg(long)]
        namespace: Option<String>,
        /// Filter by log level (info, warn, error)
        #[arg(long, short)]
        level: Option<String>,
        /// Search text pattern
        #[arg(long, short = 'g')]
        grep: Option<String>,
        /// Limit number of results
        #[arg(long, short = 'n', default_value = "100")]
        limit: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Query traces from ClickHouse
    Traces {
        /// Time range (e.g., 15m, 1h, 24h)
        #[arg(long, short, default_value = "15m")]
        since: String,
        /// Filter by service name
        #[arg(long, short = 'w')]
        workload: Option<String>,
        /// Filter by operation/endpoint
        #[arg(long, short)]
        operation: Option<String>,
        /// Filter by minimum duration in ms
        #[arg(long)]
        min_duration: Option<u64>,
        /// Filter by status (ok, error)
        #[arg(long)]
        status: Option<String>,
        /// Limit number of results
        #[arg(long, short = 'n', default_value = "50")]
        limit: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Query Kubernetes events from ClickHouse
    Events {
        /// Time range (e.g., 15m, 1h, 24h)
        #[arg(long, short, default_value = "1h")]
        since: String,
        /// Filter by namespace
        #[arg(long)]
        namespace: Option<String>,
        /// Filter by event type (Normal, Warning)
        #[arg(long, short = 't')]
        event_type: Option<String>,
        /// Filter by reason
        #[arg(long)]
        reason: Option<String>,
        /// Limit number of results
        #[arg(long, short = 'n', default_value = "100")]
        limit: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Query metrics from VictoriaMetrics using PromQL
    Metrics {
        /// PromQL query
        query: String,
        /// Time range (e.g., 15m, 1h, 24h)
        #[arg(long, short, default_value = "1h")]
        since: String,
        /// Step interval (e.g., 15s, 1m, 5m)
        #[arg(long, default_value = "60s")]
        step: String,
        /// Output as JSON (default is table)
        #[arg(long)]
        json: bool,
    },
    /// Execute raw ClickHouse SQL query
    SqlClickhouse {
        /// SQL query to execute
        query: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List available tables in ClickHouse
    Tables,
    /// List API endpoints with metrics
    Api {
        /// Filter by namespace
        #[arg(long)]
        namespace: Option<String>,
        /// Filter by server/workload name
        #[arg(long, short = 'w')]
        workload: Option<String>,
        /// Filter by endpoint (e.g., /api/users)
        #[arg(long, short = 'e')]
        endpoint: Option<String>,
        /// Only show APIs with errors
        #[arg(long)]
        errors: bool,
        /// Limit number of results
        #[arg(long, short = 'n', default_value = "50")]
        limit: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List workloads with metrics
    Workloads {
        /// Filter by namespace
        #[arg(long)]
        namespace: Option<String>,
        /// Filter by workload name
        #[arg(long, short = 'w')]
        workload: Option<String>,
        /// Filter by kind (Deployment, StatefulSet, DaemonSet, etc.)
        #[arg(long, short)]
        kind: Option<String>,
        /// Only show workloads with errors (error_rate > 0)
        #[arg(long)]
        errors: bool,
        /// Only show not ready workloads
        #[arg(long)]
        not_ready: bool,
        /// Limit number of results
        #[arg(long, short = 'n', default_value = "50")]
        limit: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Query alerts from ClickHouse
    Alerts {
        /// Time range (e.g., 15m, 1h, 24h)
        #[arg(long, short, default_value = "1h")]
        since: String,
        /// Filter by state (Normal, Pending, Alerting)
        #[arg(long)]
        state: Option<String>,
        /// Filter by severity (S1, S2, S3, S4, S5)
        #[arg(long)]
        severity: Option<String>,
        /// Filter by namespace
        #[arg(long)]
        namespace: Option<String>,
        /// Filter by workload
        #[arg(long, short = 'w')]
        workload: Option<String>,
        /// Filter by monitor name
        #[arg(long, short = 'm')]
        monitor: Option<String>,
        /// Limit number of results
        #[arg(long, short = 'n', default_value = "50")]
        limit: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Query detected issues from traces
    Issues {
        /// Time range (e.g., 15m, 1h, 24h)
        #[arg(long, short, default_value = "1h")]
        since: String,
        /// Filter by namespace
        #[arg(long)]
        namespace: Option<String>,
        /// Filter by workload
        #[arg(long, short = 'w')]
        workload: Option<String>,
        /// Filter by issue description
        #[arg(long, short = 'g')]
        grep: Option<String>,
        /// Filter by return code
        #[arg(long)]
        code: Option<String>,
        /// Limit number of results
        #[arg(long, short = 'n', default_value = "50")]
        limit: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Grafana API commands
    Grafana {
        #[command(subcommand)]
        command: GrafanaCommands,
    },
}

#[derive(Subcommand)]
enum GrafanaCommands {
    /// List all datasources
    Datasources,
    /// Get datasource by UID
    Datasource {
        /// Datasource UID
        uid: String,
    },
    /// List dashboards
    Dashboards,
    /// Search dashboards
    Search {
        /// Search query
        query: String,
    },
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.ends_with('m') {
        let mins: i64 = s.trim_end_matches('m').parse()?;
        Ok(Duration::minutes(mins))
    } else if s.ends_with('h') {
        let hours: i64 = s.trim_end_matches('h').parse()?;
        Ok(Duration::hours(hours))
    } else if s.ends_with('d') {
        let days: i64 = s.trim_end_matches('d').parse()?;
        Ok(Duration::days(days))
    } else if s.ends_with('s') {
        let secs: i64 = s.trim_end_matches('s').parse()?;
        Ok(Duration::seconds(secs))
    } else {
        bail!("Invalid duration format: {}. Use format like 15m, 1h, 24h, 7d", s);
    }
}

fn fetch_grafana_token_from_groundcover() -> Result<String> {
    let output = Command::new("groundcover")
        .args(["auth", "generate-service-account-token"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to get Grafana token from groundcover CLI: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)?;
    // Token starts with "glsa_" and is on the last line
    let token = stdout
        .lines()
        .rev()
        .find(|line| line.trim().starts_with("glsa_"))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not parse Grafana token from groundcover output: {}", stdout))?;

    Ok(token)
}

fn fetch_api_key_from_groundcover() -> Result<String> {
    let output = Command::new("groundcover")
        .args(["auth", "get-datasources-api-key"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to get API key from groundcover CLI: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)?;
    // The API key is on the last non-empty line, contains only alphanumeric chars
    let key = stdout
        .lines()
        .rev()
        .find(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not parse API key from groundcover output: {}", stdout))?;

    Ok(key)
}

async fn get_client(config: &Config) -> Result<Client> {
    let api_key = config
        .get_api_key()
        .ok_or_else(|| anyhow::anyhow!("No API key configured. Run: groundcover-cli config --fetch"))?;
    Client::new(api_key.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    match cli.command {
        Commands::Config { api_key, fetch } => {
            if fetch {
                eprintln!("Fetching API key from groundcover CLI...");
                let key = fetch_api_key_from_groundcover()?;
                config.api_key = Some(key);

                eprintln!("Fetching Grafana token from groundcover CLI...");
                let token = fetch_grafana_token_from_groundcover()?;
                config.grafana_token = Some(token);

                config.save()?;
                println!("API key and Grafana token saved.");
            } else if let Some(key) = api_key {
                config.api_key = Some(key);
                config.save()?;
                println!("API key saved.");
            } else {
                println!("Current configuration:");
                println!(
                    "  api_key: {}",
                    config
                        .api_key
                        .as_ref()
                        .map(|k| format!("{}...", &k[..12.min(k.len())]))
                        .unwrap_or_else(|| "(not set)".to_string())
                );
                println!(
                    "  grafana_token: {}",
                    config
                        .grafana_token
                        .as_ref()
                        .map(|k| format!("{}...", &k[..20.min(k.len())]))
                        .unwrap_or_else(|| "(not set)".to_string())
                );
            }
        }

        Commands::Logs {
            since,
            workload,
            namespace,
            level,
            grep,
            limit,
            json,
        } => {
            let client = get_client(&config).await?;
            let duration = parse_duration(&since)?;

            let mut conditions = vec![format!(
                "timestamp > now() - INTERVAL '{}' SECOND",
                duration.num_seconds()
            )];

            if let Some(w) = workload {
                conditions.push(format!("workload LIKE '%{}%'", w));
            }
            if let Some(ns) = namespace {
                conditions.push(format!("namespace = '{}'", ns));
            }
            if let Some(l) = level {
                conditions.push(format!("level = '{}'", l.to_uppercase()));
            }
            if let Some(g) = grep {
                conditions.push(format!("body LIKE '%{}%'", g));
            }

            let sql = format!(
                "SELECT timestamp, namespace, workload, level, body FROM logs WHERE {} ORDER BY timestamp DESC LIMIT {}",
                conditions.join(" AND "),
                limit
            );

            if json {
                let result = client.query_clickhouse_json(&sql).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = client.query_clickhouse(&format!("{} FORMAT TabSeparated", sql)).await?;
                for line in result.lines() {
                    println!("{}", line);
                }
            }
        }

        Commands::Traces {
            since,
            workload,
            operation,
            min_duration,
            status,
            limit,
            json,
        } => {
            let client = get_client(&config).await?;
            let duration = parse_duration(&since)?;

            let mut conditions = vec![format!(
                "start_timestamp > now() - INTERVAL '{}' SECOND",
                duration.num_seconds()
            )];

            if let Some(w) = workload {
                conditions.push(format!("service_name LIKE '%{}%'", w));
            }
            if let Some(op) = operation {
                conditions.push(format!("operation LIKE '%{}%'", op));
            }
            if let Some(min_dur) = min_duration {
                conditions.push(format!("duration_ms >= {}", min_dur));
            }
            if let Some(s) = status {
                if s == "error" {
                    conditions.push("status_code != 0".to_string());
                } else if s == "ok" {
                    conditions.push("status_code = 0".to_string());
                }
            }

            let sql = format!(
                "SELECT start_timestamp, service_name, operation, duration_ms, status_code FROM traces WHERE {} ORDER BY start_timestamp DESC LIMIT {}",
                conditions.join(" AND "),
                limit
            );

            if json {
                let result = client.query_clickhouse_json(&sql).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = client.query_clickhouse(&format!("{} FORMAT TabSeparated", sql)).await?;
                for line in result.lines() {
                    println!("{}", line);
                }
            }
        }

        Commands::Events {
            since,
            namespace,
            event_type,
            reason,
            limit,
            json,
        } => {
            let client = get_client(&config).await?;
            let duration = parse_duration(&since)?;

            let mut conditions = vec![format!(
                "timestamp > now() - INTERVAL '{}' SECOND",
                duration.num_seconds()
            )];

            if let Some(ns) = namespace {
                conditions.push(format!("namespace = '{}'", ns));
            }
            if let Some(t) = event_type {
                conditions.push(format!("type = '{}'", t));
            }
            if let Some(r) = reason {
                conditions.push(format!("reason LIKE '%{}%'", r));
            }

            let sql = format!(
                "SELECT timestamp, namespace, type, reason, message FROM k8s_events WHERE {} ORDER BY timestamp DESC LIMIT {}",
                conditions.join(" AND "),
                limit
            );

            if json {
                let result = client.query_clickhouse_json(&sql).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = client.query_clickhouse(&format!("{} FORMAT TabSeparated", sql)).await?;
                for line in result.lines() {
                    println!("{}", line);
                }
            }
        }

        Commands::Metrics { query, since, step, json: json_output } => {
            let client = get_client(&config).await?;
            let duration = parse_duration(&since)?;
            let now = Utc::now();
            let start = now - duration;

            let result = client
                .query_metrics(&query, start.timestamp(), now.timestamp(), Some(&step))
                .await?;

            if json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                // Print metrics in a readable format
                if let Some(data) = result.get("data").and_then(|d| d.get("result")) {
                    if let Some(arr) = data.as_array() {
                        for series in arr {
                            if let Some(metric) = series.get("metric") {
                                println!("Metric: {}", metric);
                            }
                            if let Some(values) = series.get("values").and_then(|v| v.as_array()) {
                                for val in values.iter().take(10) {
                                    if let Some(arr) = val.as_array() {
                                        if arr.len() >= 2 {
                                            let ts = arr[0].as_f64().unwrap_or(0.0);
                                            let v = arr[1].as_str().unwrap_or("?");
                                            println!("  {}: {}", ts as i64, v);
                                        }
                                    }
                                }
                                if values.len() > 10 {
                                    println!("  ... and {} more values", values.len() - 10);
                                }
                            }
                        }
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
        }

        Commands::SqlClickhouse { query, json } => {
            let client = get_client(&config).await?;
            if json {
                let result = client.query_clickhouse_json(&query).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = client.query_clickhouse(&query).await?;
                println!("{}", result);
            }
        }

        Commands::Tables => {
            let client = get_client(&config).await?;
            let result = client
                .query_clickhouse("SHOW TABLES FORMAT TabSeparated")
                .await?;
            println!("Available tables:");
            for line in result.lines() {
                println!("  {}", line);
            }
        }

        Commands::Api {
            namespace,
            workload,
            endpoint,
            errors,
            limit,
            json,
        } => {
            let client = get_client(&config).await?;

            let mut conditions: Vec<String> = vec![];

            if let Some(ns) = namespace {
                conditions.push(format!("server_namespace = '{}'", ns));
            }
            if let Some(w) = workload {
                conditions.push(format!("server LIKE '%{}%'", w));
            }
            if let Some(ep) = endpoint {
                conditions.push(format!("span_name LIKE '%{}%'", ep));
            }
            if errors {
                conditions.push("error_rate > 0".to_string());
            }

            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", conditions.join(" AND "))
            };

            let sql = format!(
                "SELECT server_namespace, server, span_name, \
                 round(rps, 2) as rps, round(error_rate * 100, 2) as error_pct, \
                 round(p50, 1) as p50_ms, round(p99, 1) as p99_ms \
                 FROM apm_measurements_resource_refreshable_one_hour \
                 {} \
                 ORDER BY rps DESC NULLS LAST \
                 LIMIT {}",
                where_clause,
                limit
            );

            if json {
                let result = client.query_clickhouse_json(&sql).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = client.query_clickhouse(&format!("{} FORMAT TabSeparated", sql)).await?;
                println!("{:<20} {:<25} {:<50} {:<10} {:<8} {:<8} {}",
                    "NAMESPACE", "SERVICE", "ENDPOINT", "RPS", "ERR%", "P50ms", "P99ms");
                println!("{}", "-".repeat(130));
                for line in result.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 7 {
                        let null_to_dash = |s: &str| if s == "\\N" { "-".to_string() } else { s.to_string() };
                        println!("{:<20} {:<25} {:<50} {:<10} {:<8} {:<8} {}",
                            &parts[0][..20.min(parts[0].len())],
                            &parts[1][..25.min(parts[1].len())],
                            &parts[2][..50.min(parts[2].len())],
                            null_to_dash(parts[3]),
                            null_to_dash(parts[4]),
                            null_to_dash(parts[5]),
                            null_to_dash(parts[6])
                        );
                    }
                }
            }
        }

        Commands::Workloads {
            namespace,
            workload,
            kind,
            errors,
            not_ready,
            limit,
            json,
        } => {
            let client = get_client(&config).await?;

            let mut conditions: Vec<String> = vec![];

            if let Some(ns) = namespace {
                conditions.push(format!("namespace = '{}'", ns));
            }
            if let Some(w) = workload {
                conditions.push(format!("workload LIKE '%{}%'", w));
            }
            if let Some(k) = kind {
                conditions.push(format!("kind = '{}'", k));
            }
            if errors {
                conditions.push("error_rate > 0".to_string());
            }
            if not_ready {
                conditions.push("ready = false".to_string());
            }

            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", conditions.join(" AND "))
            };

            let sql = format!(
                "SELECT namespace, workload, kind, ready, pods_count, \
                 round(rps, 2) as rps, round(error_rate * 100, 2) as error_pct, \
                 round(p50, 1) as p50_ms, round(p99, 1) as p99_ms \
                 FROM workloads_refreshable \
                 {} \
                 ORDER BY rps DESC NULLS LAST \
                 LIMIT {}",
                where_clause,
                limit
            );

            if json {
                let result = client.query_clickhouse_json(&sql).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = client.query_clickhouse(&format!("{} FORMAT TabSeparated", sql)).await?;
                println!("{:<20} {:<30} {:<12} {:<6} {:<6} {:<8} {:<8} {:<8} {}",
                    "NAMESPACE", "WORKLOAD", "KIND", "READY", "PODS", "RPS", "ERR%", "P50ms", "P99ms");
                println!("{}", "-".repeat(120));
                for line in result.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 9 {
                        let null_to_dash = |s: &str| if s == "\\N" { "-".to_string() } else { s.to_string() };
                        println!("{:<20} {:<30} {:<12} {:<6} {:<6} {:<8} {:<8} {:<8} {}",
                            parts[0],
                            &parts[1][..30.min(parts[1].len())],
                            parts[2],
                            if parts[3] == "true" { "yes" } else { "no" },
                            null_to_dash(parts[4]),
                            null_to_dash(parts[5]),
                            null_to_dash(parts[6]),
                            null_to_dash(parts[7]),
                            null_to_dash(parts[8])
                        );
                    }
                }
            }
        }

        Commands::Alerts {
            since,
            state,
            severity,
            namespace,
            workload,
            monitor,
            limit,
            json,
        } => {
            let client = get_client(&config).await?;
            let duration = parse_duration(&since)?;

            let mut conditions = vec![format!(
                "timestamp > now() - INTERVAL '{}' SECOND",
                duration.num_seconds()
            )];

            if let Some(s) = state {
                conditions.push(format!("state = '{}'", s));
            }
            if let Some(sev) = severity {
                conditions.push(format!("severity = '{}'", sev));
            }
            if let Some(ns) = namespace {
                conditions.push(format!("namespace = '{}'", ns));
            }
            if let Some(w) = workload {
                conditions.push(format!("workload LIKE '%{}%'", w));
            }
            if let Some(m) = monitor {
                conditions.push(format!("monitor_name LIKE '%{}%'", m));
            }

            let sql = format!(
                "SELECT timestamp, monitor_name, state, severity, namespace, workload \
                 FROM monitor_state \
                 WHERE {} \
                 ORDER BY timestamp DESC \
                 LIMIT {}",
                conditions.join(" AND "),
                limit
            );

            if json {
                let result = client.query_clickhouse_json(&sql).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = client.query_clickhouse(&format!("{} FORMAT TabSeparated", sql)).await?;
                println!("{:<24} {:<40} {:<10} {:<6} {:<20} {}",
                    "TIMESTAMP", "MONITOR", "STATE", "SEV", "NAMESPACE", "WORKLOAD");
                println!("{}", "-".repeat(120));
                for line in result.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 6 {
                        println!("{:<24} {:<40} {:<10} {:<6} {:<20} {}",
                            parts[0],
                            &parts[1][..40.min(parts[1].len())],
                            parts[2],
                            parts[3],
                            parts[4],
                            parts[5]
                        );
                    }
                }
            }
        }

        Commands::Issues {
            since,
            namespace,
            workload,
            grep,
            code,
            limit,
            json,
        } => {
            let client = get_client(&config).await?;
            let duration = parse_duration(&since)?;

            let mut conditions = vec![format!(
                "last_seen > now() - INTERVAL '{}' SECOND",
                duration.num_seconds()
            )];

            if let Some(ns) = namespace {
                conditions.push(format!("namespace = '{}'", ns));
            }
            if let Some(w) = workload {
                conditions.push(format!("workload LIKE '%{}%'", w));
            }
            if let Some(g) = grep {
                conditions.push(format!("issue_description LIKE '%{}%'", g));
            }
            if let Some(c) = code {
                conditions.push(format!("return_code = '{}'", c));
            }

            let sql = format!(
                "SELECT last_seen, namespace, workload, issue_description, return_code, \
                 sum(incident_count) as total_count \
                 FROM traces_issues_list_one_minute_view \
                 WHERE {} \
                 GROUP BY last_seen, namespace, workload, issue_description, return_code \
                 ORDER BY last_seen DESC \
                 LIMIT {}",
                conditions.join(" AND "),
                limit
            );

            if json {
                let result = client.query_clickhouse_json(&sql).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let result = client.query_clickhouse(&format!("{} FORMAT TabSeparated", sql)).await?;
                println!("{:<24} {:<20} {:<25} {:<30} {:<8} {}",
                    "LAST_SEEN", "NAMESPACE", "WORKLOAD", "ISSUE", "CODE", "COUNT");
                println!("{}", "-".repeat(120));
                for line in result.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 6 {
                        let null_to_dash = |s: &str| if s == "\\N" { "-".to_string() } else { s.to_string() };
                        println!("{:<24} {:<20} {:<25} {:<30} {:<8} {}",
                            parts[0],
                            &parts[1][..20.min(parts[1].len())],
                            &parts[2][..25.min(parts[2].len())],
                            &parts[3][..30.min(parts[3].len())],
                            null_to_dash(parts[4]),
                            null_to_dash(parts[5])
                        );
                    }
                }
            }
        }

        Commands::Grafana { command } => {
            let token = config
                .get_grafana_token()
                .ok_or_else(|| anyhow::anyhow!("No Grafana token configured. Run: groundcover-cli config --fetch"))?;
            let client = GrafanaClient::new(token.to_string())?;

            match command {
                GrafanaCommands::Datasources => {
                    let result = client.list_datasources().await?;
                    if let Some(arr) = result.as_array() {
                        println!("{:<40} {:<12} {}", "UID", "TYPE", "NAME");
                        println!("{}", "-".repeat(80));
                        for ds in arr {
                            println!(
                                "{:<40} {:<12} {}",
                                ds["uid"].as_str().unwrap_or("?"),
                                ds["type"].as_str().unwrap_or("?"),
                                ds["name"].as_str().unwrap_or("?")
                            );
                        }
                    }
                }
                GrafanaCommands::Datasource { uid } => {
                    let result = client.get_datasource(&uid).await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                GrafanaCommands::Dashboards => {
                    let result = client.list_dashboards().await?;
                    if let Some(arr) = result.as_array() {
                        println!("{:<40} {}", "UID", "TITLE");
                        println!("{}", "-".repeat(80));
                        for db in arr {
                            println!(
                                "{:<40} {}",
                                db["uid"].as_str().unwrap_or("?"),
                                db["title"].as_str().unwrap_or("?")
                            );
                        }
                    }
                }
                GrafanaCommands::Search { query } => {
                    let result = client.search_dashboards(&query).await?;
                    if let Some(arr) = result.as_array() {
                        println!("{:<40} {}", "UID", "TITLE");
                        println!("{}", "-".repeat(80));
                        for db in arr {
                            println!(
                                "{:<40} {}",
                                db["uid"].as_str().unwrap_or("?"),
                                db["title"].as_str().unwrap_or("?")
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
