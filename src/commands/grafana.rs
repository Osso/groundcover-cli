use anyhow::Result;
use serde_json::Value;

use crate::client::GrafanaClient;
use crate::helpers::filter_by_title;

pub enum GrafanaCommands {
    Datasources,
    Datasource { uid: String },
    Dashboards,
    Search { query: String },
    AlertRules { filter: Option<String>, json: bool },
    Alerts { filter: Option<String>, json: bool },
}

pub async fn run_grafana(client: &GrafanaClient, command: GrafanaCommands) -> Result<()> {
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
