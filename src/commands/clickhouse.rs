use anyhow::{Result, bail};
use chrono::Utc;

use crate::client::Client;
use crate::helpers::{parse_duration, print_query};

pub struct LogsArgs {
    pub since: String,
    pub workload: Option<String>,
    pub namespace: Option<String>,
    pub level: Option<String>,
    pub grep: Option<String>,
    pub limit: u32,
    pub json: bool,
}

pub struct TracesArgs {
    pub since: String,
    pub workload: Option<String>,
    pub operation: Option<String>,
    pub min_duration: Option<u64>,
    pub status: Option<String>,
    pub limit: u32,
    pub json: bool,
}

pub struct EventsArgs {
    pub since: String,
    pub namespace: Option<String>,
    pub event_type: Option<String>,
    pub reason: Option<String>,
    pub limit: u32,
    pub json: bool,
}

pub struct MetricsArgs {
    pub query: String,
    pub since: String,
    pub step: String,
    pub json: bool,
}

pub async fn run_logs(client: &Client, args: LogsArgs) -> Result<()> {
    let duration = parse_duration(&args.since)?;
    if args.grep.is_some() && duration.num_hours() > 24 {
        bail!(
            "--since {} too large for -g (max 24h): ClickHouse times out on body LIKE scans beyond ~36h. Narrow the window or add -w/--workload to use an indexed filter.",
            args.since
        );
    }
    let mut conditions = vec![format!(
        "timestamp > now() - INTERVAL '{}' SECOND",
        duration.num_seconds()
    )];
    if let Some(w) = args.workload {
        conditions.push(format!("workload LIKE '%{w}%'"));
    }
    if let Some(ns) = args.namespace {
        conditions.push(format!("namespace = '{ns}'"));
    }
    if let Some(l) = args.level {
        conditions.push(format!("level = '{}'", l.to_uppercase()));
    }
    if let Some(g) = args.grep {
        conditions.push(format!("body LIKE '%{g}%'"));
    }
    let sql = format!(
        "SELECT timestamp, namespace, workload, level, body \
         FROM logs WHERE {} ORDER BY timestamp DESC LIMIT {}",
        conditions.join(" AND "),
        args.limit
    );
    print_query(client, &sql, args.json).await
}

pub async fn run_traces(client: &Client, args: TracesArgs) -> Result<()> {
    let duration = parse_duration(&args.since)?;
    let mut conditions = vec![format!(
        "start_timestamp > now() - INTERVAL '{}' SECOND",
        duration.num_seconds()
    )];
    if let Some(w) = args.workload {
        conditions.push(format!("service_name LIKE '%{w}%'"));
    }
    if let Some(op) = args.operation {
        conditions.push(format!("operation LIKE '%{op}%'"));
    }
    if let Some(min_dur) = args.min_duration {
        conditions.push(format!("duration_ms >= {min_dur}"));
    }
    if let Some(s) = args.status {
        if s == "error" {
            conditions.push("status_code != 0".to_string());
        } else if s == "ok" {
            conditions.push("status_code = 0".to_string());
        }
    }
    let sql = format!(
        "SELECT start_timestamp, service_name, operation, duration_ms, status_code \
         FROM traces WHERE {} ORDER BY start_timestamp DESC LIMIT {}",
        conditions.join(" AND "),
        args.limit
    );
    print_query(client, &sql, args.json).await
}

pub async fn run_events(client: &Client, args: EventsArgs) -> Result<()> {
    let duration = parse_duration(&args.since)?;
    let mut conditions = vec![
        format!(
            "timestamp > now() - INTERVAL '{}' SECOND",
            duration.num_seconds()
        ),
        "length(k8s_reason) > 0".to_string(),
    ];
    if let Some(ns) = args.namespace {
        conditions.push(format!("entity_namespace = '{ns}'"));
    }
    if let Some(t) = args.event_type {
        conditions.push(format!("type = '{t}'"));
    }
    if let Some(r) = args.reason {
        conditions.push(format!("k8s_reason LIKE '%{r}%'"));
    }
    let sql = format!(
        "SELECT timestamp, entity_namespace, type, k8s_reason, k8s_message \
         FROM events WHERE {} ORDER BY timestamp DESC LIMIT {}",
        conditions.join(" AND "),
        args.limit
    );
    print_query(client, &sql, args.json).await
}

pub async fn run_metrics(client: &Client, args: MetricsArgs) -> Result<()> {
    let duration = parse_duration(&args.since)?;
    let now = Utc::now();
    let start = now - duration;
    let result = client
        .query_metrics(
            &args.query,
            start.timestamp(),
            now.timestamp(),
            Some(&args.step),
        )
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_metrics_table(&result);
    }
    Ok(())
}

fn print_metrics_table(result: &serde_json::Value) {
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

pub async fn run_sql_clickhouse(client: &Client, query: String, json: bool) -> Result<()> {
    if json {
        let result = client.query_clickhouse_json(&query).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let result = client.query_clickhouse(&query).await?;
        println!("{result}");
    }
    Ok(())
}

pub async fn run_tables(client: &Client) -> Result<()> {
    let result = client
        .query_clickhouse("SHOW TABLES FORMAT TabSeparated")
        .await?;
    println!("Available tables:");
    for line in result.lines() {
        println!("  {line}");
    }
    Ok(())
}
