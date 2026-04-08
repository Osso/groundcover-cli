use anyhow::Result;

use crate::client::Client;
use crate::helpers::null_to_dash;

pub struct ApiArgs {
    pub namespace: Option<String>,
    pub workload: Option<String>,
    pub endpoint: Option<String>,
    pub errors: bool,
    pub limit: u32,
    pub json: bool,
}

pub struct WorkloadsArgs {
    pub namespace: Option<String>,
    pub workload: Option<String>,
    pub kind: Option<String>,
    pub errors: bool,
    pub not_ready: bool,
    pub limit: u32,
    pub json: bool,
}

pub async fn run_api(client: &Client, args: ApiArgs) -> Result<()> {
    let mut conditions: Vec<String> = vec![];
    if let Some(ns) = args.namespace {
        conditions.push(format!("server_namespace = '{ns}'"));
    }
    if let Some(w) = args.workload {
        conditions.push(format!("server LIKE '%{w}%'"));
    }
    if let Some(ep) = args.endpoint {
        conditions.push(format!("span_name LIKE '%{ep}%'"));
    }
    if args.errors {
        conditions.push("error_rate > 0".to_string());
    }
    let where_clause = build_where(&conditions);
    let sql = format!(
        "SELECT server_namespace, server, span_name, \
         round(rps, 2) as rps, round(error_rate * 100, 2) as error_pct, \
         round(p50, 1) as p50_ms, round(p99, 1) as p99_ms \
         FROM apm_measurements_resource_refreshable_one_hour \
         {where_clause} \
         ORDER BY rps DESC NULLS LAST \
         LIMIT {}",
        args.limit
    );
    if args.json {
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

pub async fn run_workloads(client: &Client, args: WorkloadsArgs) -> Result<()> {
    let mut conditions: Vec<String> = vec![];
    if let Some(ns) = args.namespace {
        conditions.push(format!("namespace = '{ns}'"));
    }
    if let Some(w) = args.workload {
        conditions.push(format!("workload LIKE '%{w}%'"));
    }
    if let Some(k) = args.kind {
        conditions.push(format!("kind = '{k}'"));
    }
    if args.errors {
        conditions.push("error_rate > 0".to_string());
    }
    if args.not_ready {
        conditions.push("ready = false".to_string());
    }
    let where_clause = build_where(&conditions);
    let sql = format!(
        "SELECT namespace, workload, kind, ready, pods_count, \
         round(rps, 2) as rps, round(error_rate * 100, 2) as error_pct, \
         round(p50, 1) as p50_ms, round(p99, 1) as p99_ms \
         FROM workloads_refreshable \
         {where_clause} \
         ORDER BY rps DESC NULLS LAST \
         LIMIT {}",
        args.limit
    );
    if args.json {
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

fn build_where(conditions: &[String]) -> String {
    if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    }
}
