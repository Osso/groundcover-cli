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
    let duration = parse_duration(&args.since)?;
    let mut conditions = vec![format!(
        "timestamp > now() - INTERVAL '{}' SECOND",
        duration.num_seconds()
    )];
    if let Some(s) = args.state {
        conditions.push(format!("state = '{s}'"));
    }
    if let Some(sev) = args.severity {
        conditions.push(format!("severity = '{sev}'"));
    }
    if let Some(ns) = args.namespace {
        conditions.push(format!("namespace = '{ns}'"));
    }
    if let Some(w) = args.workload {
        conditions.push(format!("workload LIKE '%{w}%'"));
    }
    if let Some(m) = args.monitor {
        conditions.push(format!("monitor_name LIKE '%{m}%'"));
    }
    let sql = format!(
        "SELECT timestamp, monitor_name, state, severity, namespace, workload \
         FROM monitor_state \
         WHERE {} \
         ORDER BY timestamp DESC \
         LIMIT {}",
        conditions.join(" AND "),
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

pub async fn run_issues(client: &Client, args: IssuesArgs) -> Result<()> {
    let duration = parse_duration(&args.since)?;
    let mut conditions = vec![format!(
        "last_seen > now() - INTERVAL '{}' SECOND",
        duration.num_seconds()
    )];
    if let Some(ns) = args.namespace {
        conditions.push(format!("namespace = '{ns}'"));
    }
    if let Some(w) = args.workload {
        conditions.push(format!("workload LIKE '%{w}%'"));
    }
    if let Some(g) = args.grep {
        conditions.push(format!("issue_description LIKE '%{g}%'"));
    }
    if let Some(c) = args.code {
        conditions.push(format!("return_code = '{c}'"));
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
