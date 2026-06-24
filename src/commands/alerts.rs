use anyhow::Result;

use crate::client::Client;
use crate::helpers::{null_to_dash, parse_duration};

pub struct AlertsArgs {
    pub since: String,
    pub state: Option<String>,
    pub severity: Option<String>,
    pub namespace: Option<String>,
    pub workload: Option<String>,
    pub monitor: Option<String>,
    pub limit: u32,
    pub json: bool,
}

pub struct IssuesArgs {
    pub since: String,
    pub namespace: Option<String>,
    pub workload: Option<String>,
    pub grep: Option<String>,
    pub code: Option<String>,
    pub limit: u32,
    pub json: bool,
}

pub async fn run_alerts(client: &Client, args: AlertsArgs) -> Result<()> {
    let sql = build_alerts_sql(&args)?;
    if args.json {
        print_clickhouse_json(client, &sql).await?;
    } else {
        let rows = query_tab_separated(client, &sql).await?;
        print_alert_rows(&rows);
    }
    Ok(())
}

pub async fn run_issues(client: &Client, args: IssuesArgs) -> Result<()> {
    let sql = build_issues_sql(&args)?;
    if args.json {
        print_clickhouse_json(client, &sql).await?;
    } else {
        let rows = query_tab_separated(client, &sql).await?;
        print_issue_rows(&rows);
    }
    Ok(())
}

fn build_alerts_sql(args: &AlertsArgs) -> Result<String> {
    let mut conditions = build_time_conditions("timestamp", &args.since)?;
    push_optional_eq(&mut conditions, "state", args.state.as_deref());
    push_optional_eq(&mut conditions, "severity", args.severity.as_deref());
    push_optional_eq(&mut conditions, "namespace", args.namespace.as_deref());
    push_optional_like(&mut conditions, "workload", args.workload.as_deref());
    push_optional_like(&mut conditions, "monitor_name", args.monitor.as_deref());
    Ok(format!(
        "SELECT timestamp, monitor_name, state, severity, namespace, workload \
         FROM monitor_state \
         WHERE {} \
         ORDER BY timestamp DESC \
         LIMIT {}",
        conditions.join(" AND "),
        args.limit
    ))
}

fn build_issues_sql(args: &IssuesArgs) -> Result<String> {
    let mut conditions = build_time_conditions("last_seen", &args.since)?;
    push_optional_eq(&mut conditions, "namespace", args.namespace.as_deref());
    push_optional_like(&mut conditions, "workload", args.workload.as_deref());
    push_optional_like(&mut conditions, "issue_description", args.grep.as_deref());
    push_optional_eq(&mut conditions, "return_code", args.code.as_deref());
    Ok(format!(
        "SELECT last_seen, namespace, workload, issue_description, return_code, \
         sum(incident_count) as total_count \
         FROM traces_issues_list_one_minute_view \
         WHERE {} \
         GROUP BY last_seen, namespace, workload, issue_description, return_code \
         ORDER BY last_seen DESC \
         LIMIT {}",
        conditions.join(" AND "),
        args.limit
    ))
}

fn build_time_conditions(field: &str, since: &str) -> Result<Vec<String>> {
    let duration = parse_duration(since)?;
    let condition = format!(
        "{field} > now() - INTERVAL '{}' SECOND",
        duration.num_seconds()
    );
    Ok(vec![condition])
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

fn print_alert_rows(rows: &str) {
    println!(
        "{:<24} {:<40} {:<10} {:<6} {:<20} {}",
        "TIMESTAMP", "MONITOR", "STATE", "SEV", "NAMESPACE", "WORKLOAD"
    );
    println!("{}", "-".repeat(120));
    for line in rows.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 6 {
            continue;
        }
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

fn print_issue_rows(rows: &str) {
    println!(
        "{:<24} {:<20} {:<25} {:<30} {:<8} {}",
        "LAST_SEEN", "NAMESPACE", "WORKLOAD", "ISSUE", "CODE", "COUNT"
    );
    println!("{}", "-".repeat(120));
    for line in rows.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 6 {
            continue;
        }
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
