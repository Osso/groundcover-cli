mod client;
mod config;

use anyhow::{Result, bail};
use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use serde_json::Value;
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
    /// List alert rules (provisioning API)
    AlertRules {
        /// Filter by title (case-insensitive substring match)
        #[arg(long, short = 'f')]
        filter: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List alert rules with state and query expressions (Ruler API)
    Alerts {
        /// Filter by title (case-insensitive substring match)
        #[arg(long, short = 'f')]
        filter: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

// === Helpers ===

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.ends_with('m') {
        Ok(Duration::minutes(s.trim_end_matches('m').parse()?))
    } else if s.ends_with('h') {
        Ok(Duration::hours(s.trim_end_matches('h').parse()?))
    } else if s.ends_with('d') {
        Ok(Duration::days(s.trim_end_matches('d').parse()?))
    } else if s.ends_with('s') {
        Ok(Duration::seconds(s.trim_end_matches('s').parse()?))
    } else {
        bail!("Invalid duration format: {s}. Use format like 15m, 1h, 24h, 7d")
    }
}

fn null_to_dash(s: &str) -> &str {
    if s == "\\N" { "-" } else { s }
}

fn fetch_groundcover_output(args: &[&str]) -> Result<String> {
    let output = Command::new("groundcover").args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("groundcover {} failed: {}", args.join(" "), stderr);
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn fetch_api_key() -> Result<String> {
    let stdout = fetch_groundcover_output(&["auth", "get-datasources-api-key"])?;
    stdout
        .lines()
        .rev()
        .find(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .map(|s| s.trim().to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not parse API key from groundcover output: {stdout}"))
}

fn fetch_grafana_token() -> Result<String> {
    let stdout = fetch_groundcover_output(&["auth", "generate-service-account-token"])?;
    stdout
        .lines()
        .rev()
        .find(|line| line.trim().starts_with("glsa_"))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| {
            anyhow::anyhow!("Could not parse Grafana token from groundcover output: {stdout}")
        })
}

fn build_client(config: &Config) -> Result<Client> {
    let api_key = config.api_key().ok_or_else(|| {
        anyhow::anyhow!("No API key configured. Run: groundcover-cli config --fetch")
    })?;
    Client::new(api_key.to_string())
}

async fn print_query(client: &Client, sql: &str, json: bool) -> Result<()> {
    if json {
        let result = client.query_clickhouse_json(sql).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let result = client
            .query_clickhouse(&format!("{sql} FORMAT TabSeparated"))
            .await?;
        for line in result.lines() {
            println!("{}", line);
        }
    }
    Ok(())
}

fn filter_by_title<'a>(items: &'a [Value], filter: Option<&str>) -> Vec<&'a Value> {
    match filter {
        Some(f) => {
            let f_lower = f.to_lowercase();
            items
                .iter()
                .filter(|item| {
                    item["title"]
                        .as_str()
                        .is_some_and(|t| t.to_lowercase().contains(&f_lower))
                })
                .collect()
        }
        None => items.iter().collect(),
    }
}

// === Command Handlers ===

fn run_config(config: &mut Config, api_key: Option<String>, fetch: bool) -> Result<()> {
    if fetch {
        eprintln!("Fetching API key from groundcover CLI...");
        config.api_key = Some(fetch_api_key()?);
        eprintln!("Fetching Grafana token from groundcover CLI...");
        config.grafana_token = Some(fetch_grafana_token()?);
        config.save()?;
        println!("API key and Grafana token saved.");
    } else if let Some(key) = api_key {
        config.api_key = Some(key);
        config.save()?;
        println!("API key saved.");
    } else {
        let mask = |k: &str, n: usize| format!("{}...", &k[..n.min(k.len())]);
        println!("Current configuration:");
        println!(
            "  api_key: {}",
            config
                .api_key
                .as_deref()
                .map(|k| mask(k, 12))
                .unwrap_or_else(|| "(not set)".to_string())
        );
        println!(
            "  grafana_token: {}",
            config
                .grafana_token
                .as_deref()
                .map(|k| mask(k, 20))
                .unwrap_or_else(|| "(not set)".to_string())
        );
    }
    Ok(())
}

async fn run_logs(
    client: &Client,
    since: String,
    workload: Option<String>,
    namespace: Option<String>,
    level: Option<String>,
    grep: Option<String>,
    limit: u32,
    json: bool,
) -> Result<()> {
    let duration = parse_duration(&since)?;
    let mut conditions = vec![format!(
        "timestamp > now() - INTERVAL '{}' SECOND",
        duration.num_seconds()
    )];
    if let Some(w) = workload {
        conditions.push(format!("workload LIKE '%{w}%'"));
    }
    if let Some(ns) = namespace {
        conditions.push(format!("namespace = '{ns}'"));
    }
    if let Some(l) = level {
        conditions.push(format!("level = '{}'", l.to_uppercase()));
    }
    if let Some(g) = grep {
        conditions.push(format!("body LIKE '%{g}%'"));
    }
    let sql = format!(
        "SELECT timestamp, namespace, workload, level, body \
         FROM logs WHERE {} ORDER BY timestamp DESC LIMIT {limit}",
        conditions.join(" AND ")
    );
    print_query(client, &sql, json).await
}

async fn run_traces(
    client: &Client,
    since: String,
    workload: Option<String>,
    operation: Option<String>,
    min_duration: Option<u64>,
    status: Option<String>,
    limit: u32,
    json: bool,
) -> Result<()> {
    let duration = parse_duration(&since)?;
    let mut conditions = vec![format!(
        "start_timestamp > now() - INTERVAL '{}' SECOND",
        duration.num_seconds()
    )];
    if let Some(w) = workload {
        conditions.push(format!("service_name LIKE '%{w}%'"));
    }
    if let Some(op) = operation {
        conditions.push(format!("operation LIKE '%{op}%'"));
    }
    if let Some(min_dur) = min_duration {
        conditions.push(format!("duration_ms >= {min_dur}"));
    }
    if let Some(s) = status {
        if s == "error" {
            conditions.push("status_code != 0".to_string());
        } else if s == "ok" {
            conditions.push("status_code = 0".to_string());
        }
    }
    let sql = format!(
        "SELECT start_timestamp, service_name, operation, duration_ms, status_code \
         FROM traces WHERE {} ORDER BY start_timestamp DESC LIMIT {limit}",
        conditions.join(" AND ")
    );
    print_query(client, &sql, json).await
}

async fn run_events(
    client: &Client,
    since: String,
    namespace: Option<String>,
    event_type: Option<String>,
    reason: Option<String>,
    limit: u32,
    json: bool,
) -> Result<()> {
    let duration = parse_duration(&since)?;
    let mut conditions = vec![
        format!(
            "timestamp > now() - INTERVAL '{}' SECOND",
            duration.num_seconds()
        ),
        "length(k8s_reason) > 0".to_string(),
    ];
    if let Some(ns) = namespace {
        conditions.push(format!("entity_namespace = '{ns}'"));
    }
    if let Some(t) = event_type {
        conditions.push(format!("type = '{t}'"));
    }
    if let Some(r) = reason {
        conditions.push(format!("k8s_reason LIKE '%{r}%'"));
    }
    let sql = format!(
        "SELECT timestamp, entity_namespace, type, k8s_reason, k8s_message \
         FROM events WHERE {} ORDER BY timestamp DESC LIMIT {limit}",
        conditions.join(" AND ")
    );
    print_query(client, &sql, json).await
}

async fn run_metrics(
    client: &Client,
    query: String,
    since: String,
    step: String,
    json: bool,
) -> Result<()> {
    let duration = parse_duration(&since)?;
    let now = Utc::now();
    let start = now - duration;
    let result = client
        .query_metrics(&query, start.timestamp(), now.timestamp(), Some(&step))
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_metrics_table(&result);
    }
    Ok(())
}

fn print_metrics_table(result: &Value) {
    let Some(data) = result
        .get("data")
        .and_then(|d| d.get("result"))
        .and_then(|r| r.as_array())
    else {
        if let Ok(pretty) = serde_json::to_string_pretty(result) {
            println!("{pretty}");
        }
        return;
    };
    for series in data {
        if let Some(metric) = series.get("metric") {
            println!("Metric: {metric}");
        }
        let Some(values) = series.get("values").and_then(|v| v.as_array()) else {
            continue;
        };
        for val in values.iter().take(10) {
            let Some(arr) = val.as_array().filter(|a| a.len() >= 2) else {
                continue;
            };
            let ts = arr[0].as_f64().unwrap_or(0.0);
            let v = arr[1].as_str().unwrap_or("?");
            println!("  {}: {v}", ts as i64);
        }
        if values.len() > 10 {
            println!("  ... and {} more values", values.len() - 10);
        }
    }
}

async fn run_sql_clickhouse(client: &Client, query: String, json: bool) -> Result<()> {
    if json {
        let result = client.query_clickhouse_json(&query).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let result = client.query_clickhouse(&query).await?;
        println!("{result}");
    }
    Ok(())
}

async fn run_tables(client: &Client) -> Result<()> {
    let result = client
        .query_clickhouse("SHOW TABLES FORMAT TabSeparated")
        .await?;
    println!("Available tables:");
    for line in result.lines() {
        println!("  {line}");
    }
    Ok(())
}

async fn run_api(
    client: &Client,
    namespace: Option<String>,
    workload: Option<String>,
    endpoint: Option<String>,
    errors: bool,
    limit: u32,
    json: bool,
) -> Result<()> {
    let mut conditions: Vec<String> = vec![];
    if let Some(ns) = namespace {
        conditions.push(format!("server_namespace = '{ns}'"));
    }
    if let Some(w) = workload {
        conditions.push(format!("server LIKE '%{w}%'"));
    }
    if let Some(ep) = endpoint {
        conditions.push(format!("span_name LIKE '%{ep}%'"));
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
         {where_clause} \
         ORDER BY rps DESC NULLS LAST \
         LIMIT {limit}"
    );
    if json {
        let result = client.query_clickhouse_json(&sql).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let result = client
            .query_clickhouse(&format!("{sql} FORMAT TabSeparated"))
            .await?;
        println!(
            "{:<20} {:<25} {:<50} {:<10} {:<8} {:<8} {}",
            "NAMESPACE", "SERVICE", "ENDPOINT", "RPS", "ERR%", "P50ms", "P99ms"
        );
        println!("{}", "-".repeat(130));
        for line in result.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 7 {
                println!(
                    "{:<20} {:<25} {:<50} {:<10} {:<8} {:<8} {}",
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
    Ok(())
}

async fn run_workloads(
    client: &Client,
    namespace: Option<String>,
    workload: Option<String>,
    kind: Option<String>,
    errors: bool,
    not_ready: bool,
    limit: u32,
    json: bool,
) -> Result<()> {
    let mut conditions: Vec<String> = vec![];
    if let Some(ns) = namespace {
        conditions.push(format!("namespace = '{ns}'"));
    }
    if let Some(w) = workload {
        conditions.push(format!("workload LIKE '%{w}%'"));
    }
    if let Some(k) = kind {
        conditions.push(format!("kind = '{k}'"));
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
         {where_clause} \
         ORDER BY rps DESC NULLS LAST \
         LIMIT {limit}"
    );
    if json {
        let result = client.query_clickhouse_json(&sql).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let result = client
            .query_clickhouse(&format!("{sql} FORMAT TabSeparated"))
            .await?;
        println!(
            "{:<20} {:<30} {:<12} {:<6} {:<6} {:<8} {:<8} {:<8} {}",
            "NAMESPACE", "WORKLOAD", "KIND", "READY", "PODS", "RPS", "ERR%", "P50ms", "P99ms"
        );
        println!("{}", "-".repeat(120));
        for line in result.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 9 {
                println!(
                    "{:<20} {:<30} {:<12} {:<6} {:<6} {:<8} {:<8} {:<8} {}",
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
    Ok(())
}

async fn run_alerts(
    client: &Client,
    since: String,
    state: Option<String>,
    severity: Option<String>,
    namespace: Option<String>,
    workload: Option<String>,
    monitor: Option<String>,
    limit: u32,
    json: bool,
) -> Result<()> {
    let duration = parse_duration(&since)?;
    let mut conditions = vec![format!(
        "timestamp > now() - INTERVAL '{}' SECOND",
        duration.num_seconds()
    )];
    if let Some(s) = state {
        conditions.push(format!("state = '{s}'"));
    }
    if let Some(sev) = severity {
        conditions.push(format!("severity = '{sev}'"));
    }
    if let Some(ns) = namespace {
        conditions.push(format!("namespace = '{ns}'"));
    }
    if let Some(w) = workload {
        conditions.push(format!("workload LIKE '%{w}%'"));
    }
    if let Some(m) = monitor {
        conditions.push(format!("monitor_name LIKE '%{m}%'"));
    }
    let sql = format!(
        "SELECT timestamp, monitor_name, state, severity, namespace, workload \
         FROM monitor_state \
         WHERE {} \
         ORDER BY timestamp DESC \
         LIMIT {limit}",
        conditions.join(" AND ")
    );
    if json {
        let result = client.query_clickhouse_json(&sql).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let result = client
            .query_clickhouse(&format!("{sql} FORMAT TabSeparated"))
            .await?;
        println!(
            "{:<24} {:<40} {:<10} {:<6} {:<20} {}",
            "TIMESTAMP", "MONITOR", "STATE", "SEV", "NAMESPACE", "WORKLOAD"
        );
        println!("{}", "-".repeat(120));
        for line in result.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 6 {
                println!(
                    "{:<24} {:<40} {:<10} {:<6} {:<20} {}",
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
    Ok(())
}

async fn run_issues(
    client: &Client,
    since: String,
    namespace: Option<String>,
    workload: Option<String>,
    grep: Option<String>,
    code: Option<String>,
    limit: u32,
    json: bool,
) -> Result<()> {
    let duration = parse_duration(&since)?;
    let mut conditions = vec![format!(
        "last_seen > now() - INTERVAL '{}' SECOND",
        duration.num_seconds()
    )];
    if let Some(ns) = namespace {
        conditions.push(format!("namespace = '{ns}'"));
    }
    if let Some(w) = workload {
        conditions.push(format!("workload LIKE '%{w}%'"));
    }
    if let Some(g) = grep {
        conditions.push(format!("issue_description LIKE '%{g}%'"));
    }
    if let Some(c) = code {
        conditions.push(format!("return_code = '{c}'"));
    }
    let sql = format!(
        "SELECT last_seen, namespace, workload, issue_description, return_code, \
         sum(incident_count) as total_count \
         FROM traces_issues_list_one_minute_view \
         WHERE {} \
         GROUP BY last_seen, namespace, workload, issue_description, return_code \
         ORDER BY last_seen DESC \
         LIMIT {limit}",
        conditions.join(" AND ")
    );
    if json {
        let result = client.query_clickhouse_json(&sql).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let result = client
            .query_clickhouse(&format!("{sql} FORMAT TabSeparated"))
            .await?;
        println!(
            "{:<24} {:<20} {:<25} {:<30} {:<8} {}",
            "LAST_SEEN", "NAMESPACE", "WORKLOAD", "ISSUE", "CODE", "COUNT"
        );
        println!("{}", "-".repeat(120));
        for line in result.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 6 {
                println!(
                    "{:<24} {:<20} {:<25} {:<30} {:<8} {}",
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
    Ok(())
}

// === Grafana Handlers ===

async fn run_grafana(client: &GrafanaClient, command: GrafanaCommands) -> Result<()> {
    match command {
        GrafanaCommands::Datasources => {
            let result = client.list_datasources().await?;
            let Some(arr) = result.as_array() else {
                return Ok(());
            };
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
        GrafanaCommands::Datasource { uid } => {
            let result = client.get_datasource(&uid).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        GrafanaCommands::Dashboards => {
            print_uid_title_table(&client.list_dashboards().await?);
        }
        GrafanaCommands::Search { query } => {
            print_uid_title_table(&client.search_dashboards(&query).await?);
        }
        GrafanaCommands::AlertRules { filter, json } => {
            run_grafana_alert_rules(client, filter, json).await?;
        }
        GrafanaCommands::Alerts { filter, json } => {
            run_grafana_alerts(client, filter, json).await?;
        }
    }
    Ok(())
}

fn print_uid_title_table(result: &Value) {
    let Some(arr) = result.as_array() else {
        return;
    };
    println!("{:<40} {}", "UID", "TITLE");
    println!("{}", "-".repeat(80));
    for item in arr {
        println!(
            "{:<40} {}",
            item["uid"].as_str().unwrap_or("?"),
            item["title"].as_str().unwrap_or("?")
        );
    }
}

async fn run_grafana_alert_rules(
    client: &GrafanaClient,
    filter: Option<String>,
    json: bool,
) -> Result<()> {
    let result = client.list_alert_rules().await?;
    let Some(arr) = result.as_array() else {
        return Ok(());
    };
    let filtered = filter_by_title(arr, filter.as_deref());
    if json {
        println!("{}", serde_json::to_string_pretty(&filtered)?);
    } else {
        println!("{:<40} {:<20} {}", "UID", "FOLDER", "TITLE");
        println!("{}", "-".repeat(100));
        for rule in &filtered {
            println!(
                "{:<40} {:<20} {}",
                rule["uid"].as_str().unwrap_or("?"),
                rule["folderUID"].as_str().unwrap_or("?"),
                rule["title"].as_str().unwrap_or("?")
            );
        }
    }
    Ok(())
}

async fn run_grafana_alerts(
    client: &GrafanaClient,
    filter: Option<String>,
    json: bool,
) -> Result<()> {
    let result = client.get_ruler_rules().await?;
    let alerts = parse_ruler_alerts(&result);
    let filtered = filter_by_title(&alerts, filter.as_deref());
    if json {
        println!("{}", serde_json::to_string_pretty(&filtered)?);
    } else {
        println!(
            "{:<10} {:<25} {:<40} {}",
            "STATE", "FOLDER", "TITLE", "EXPRESSION"
        );
        println!("{}", "-".repeat(120));
        for alert in &filtered {
            let title = alert["title"].as_str().unwrap_or("?");
            let title_display = if title.len() > 38 {
                format!("{}...", &title[..35])
            } else {
                title.to_string()
            };
            let expr = alert["expr"].as_str().unwrap_or("");
            let expr_display = if expr.len() > 45 {
                format!("{}...", &expr[..42])
            } else {
                expr.to_string()
            };
            let folder = alert["folder"].as_str().unwrap_or("?");
            println!(
                "{:<10} {:<25} {:<40} {}",
                alert["state"].as_str().unwrap_or("?"),
                &folder[..25.min(folder.len())],
                title_display,
                expr_display
            );
        }
    }
    Ok(())
}

fn parse_ruler_alerts(result: &Value) -> Vec<Value> {
    let Some(obj) = result.as_object() else {
        return Vec::new();
    };
    let mut alerts = Vec::new();
    for (folder_name, groups) in obj {
        let Some(groups_arr) = groups.as_array() else {
            continue;
        };
        for group in groups_arr {
            let Some(rules) = group.get("rules").and_then(|r| r.as_array()) else {
                continue;
            };
            for rule in rules {
                let grafana_alert = rule.get("grafana_alert");
                let title = grafana_alert
                    .and_then(|ga| ga.get("title"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("?");
                let state = rule.get("state").and_then(|s| s.as_str()).unwrap_or("?");
                let expr = grafana_alert
                    .and_then(|ga| ga.get("data"))
                    .and_then(|d| d.as_array())
                    .and_then(|arr| {
                        arr.iter().find_map(|item| {
                            item.get("model")
                                .and_then(|m| m.get("expr"))
                                .and_then(|e| e.as_str())
                                .filter(|s| !s.is_empty())
                        })
                    })
                    .unwrap_or("");
                alerts.push(serde_json::json!({
                    "title": title,
                    "state": state,
                    "folder": folder_name,
                    "expr": expr,
                }));
            }
        }
    }
    alerts
}

// === Main ===

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    match cli.command {
        Commands::Config { api_key, fetch } => run_config(&mut config, api_key, fetch),
        Commands::Logs {
            since,
            workload,
            namespace,
            level,
            grep,
            limit,
            json,
        } => {
            run_logs(
                &build_client(&config)?,
                since,
                workload,
                namespace,
                level,
                grep,
                limit,
                json,
            )
            .await
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
            run_traces(
                &build_client(&config)?,
                since,
                workload,
                operation,
                min_duration,
                status,
                limit,
                json,
            )
            .await
        }
        Commands::Events {
            since,
            namespace,
            event_type,
            reason,
            limit,
            json,
        } => {
            run_events(
                &build_client(&config)?,
                since,
                namespace,
                event_type,
                reason,
                limit,
                json,
            )
            .await
        }
        Commands::Metrics {
            query,
            since,
            step,
            json,
        } => run_metrics(&build_client(&config)?, query, since, step, json).await,
        Commands::SqlClickhouse { query, json } => {
            run_sql_clickhouse(&build_client(&config)?, query, json).await
        }
        Commands::Tables => run_tables(&build_client(&config)?).await,
        Commands::Api {
            namespace,
            workload,
            endpoint,
            errors,
            limit,
            json,
        } => {
            run_api(
                &build_client(&config)?,
                namespace,
                workload,
                endpoint,
                errors,
                limit,
                json,
            )
            .await
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
            run_workloads(
                &build_client(&config)?,
                namespace,
                workload,
                kind,
                errors,
                not_ready,
                limit,
                json,
            )
            .await
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
            run_alerts(
                &build_client(&config)?,
                since,
                state,
                severity,
                namespace,
                workload,
                monitor,
                limit,
                json,
            )
            .await
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
            run_issues(
                &build_client(&config)?,
                since,
                namespace,
                workload,
                grep,
                code,
                limit,
                json,
            )
            .await
        }
        Commands::Grafana { command } => {
            let token = config.grafana_token().ok_or_else(|| {
                anyhow::anyhow!("No Grafana token configured. Run: groundcover-cli config --fetch")
            })?;
            let client = GrafanaClient::new(token.to_string())?;
            run_grafana(&client, command).await
        }
    }
}
