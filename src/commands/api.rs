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
    let sql = build_api_sql(&args);
    if args.json {
        print_clickhouse_json(client, &sql).await?;
    } else {
        let rows = query_tab_separated(client, &sql).await?;
        print_api_rows(&rows);
    }
    Ok(())
}

pub async fn run_workloads(client: &Client, args: WorkloadsArgs) -> Result<()> {
    let sql = build_workloads_sql(&args);
    if args.json {
        print_clickhouse_json(client, &sql).await?;
    } else {
        let rows = query_tab_separated(client, &sql).await?;
        print_workload_rows(&rows);
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

fn build_api_sql(args: &ApiArgs) -> String {
    let mut conditions = Vec::new();
    push_optional_eq(
        &mut conditions,
        "server_namespace",
        args.namespace.as_deref(),
    );
    push_optional_like(&mut conditions, "server", args.workload.as_deref());
    push_optional_like(&mut conditions, "span_name", args.endpoint.as_deref());
    if args.errors {
        conditions.push("error_rate > 0".to_string());
    }
    let where_clause = build_where(&conditions);
    format!(
        "SELECT server_namespace, server, span_name, \
         round(rps, 2) as rps, round(error_rate * 100, 2) as error_pct, \
         round(p50, 1) as p50_ms, round(p99, 1) as p99_ms \
         FROM apm_measurements_resource_refreshable_one_hour \
         {where_clause} \
         ORDER BY rps DESC NULLS LAST \
         LIMIT {}",
        args.limit
    )
}

fn build_workloads_sql(args: &WorkloadsArgs) -> String {
    let mut conditions = Vec::new();
    push_optional_eq(&mut conditions, "namespace", args.namespace.as_deref());
    push_optional_like(&mut conditions, "workload", args.workload.as_deref());
    push_optional_eq(&mut conditions, "kind", args.kind.as_deref());
    if args.errors {
        conditions.push("error_rate > 0".to_string());
    }
    if args.not_ready {
        conditions.push("ready = false".to_string());
    }
    let where_clause = build_where(&conditions);
    format!(
        "SELECT namespace, workload, kind, ready, pods_count, \
         round(rps, 2) as rps, round(error_rate * 100, 2) as error_pct, \
         round(p50, 1) as p50_ms, round(p99, 1) as p99_ms \
         FROM workloads_refreshable \
         {where_clause} \
         ORDER BY rps DESC NULLS LAST \
         LIMIT {}",
        args.limit
    )
}

fn push_optional_eq(conditions: &mut Vec<String>, field: &str, value: Option<&str>) {
    if let Some(value) = value {
        conditions.push(format!("{field} = '{value}'"));
    }
}

fn push_optional_like(conditions: &mut Vec<String>, field: &str, value: Option<&str>) {
    if let Some(value) = value {
        conditions.push(format!("{field} LIKE '%{value}%'"));
    }
}

async fn print_clickhouse_json(client: &Client, sql: &str) -> Result<()> {
    let result = client.query_clickhouse_json(sql).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn query_tab_separated(client: &Client, sql: &str) -> Result<String> {
    client
        .query_clickhouse(&format!("{sql} FORMAT TabSeparated"))
        .await
}

fn print_api_rows(rows: &str) {
    println!(
        "{:<20} {:<25} {:<50} {:<10} {:<8} {:<8} {}",
        "NAMESPACE", "SERVICE", "ENDPOINT", "RPS", "ERR%", "P50ms", "P99ms"
    );
    println!("{}", "-".repeat(130));
    for line in rows.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 7 {
            continue;
        }
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

fn print_workload_rows(rows: &str) {
    println!(
        "{:<20} {:<30} {:<12} {:<6} {:<6} {:<8} {:<8} {:<8} {}",
        "NAMESPACE", "WORKLOAD", "KIND", "READY", "PODS", "RPS", "ERR%", "P50ms", "P99ms"
    );
    println!("{}", "-".repeat(120));
    for line in rows.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 9 {
            continue;
        }
        println!(
            "{:<20} {:<30} {:<12} {:<6} {:<6} {:<8} {:<8} {:<8} {}",
            parts[0],
            &parts[1][..30.min(parts[1].len())],
            parts[2],
            yes_or_no(parts[3]),
            null_to_dash(parts[4]),
            null_to_dash(parts[5]),
            null_to_dash(parts[6]),
            null_to_dash(parts[7]),
            null_to_dash(parts[8])
        );
    }
}

fn yes_or_no(value: &str) -> &str {
    if value == "true" { "yes" } else { "no" }
}
