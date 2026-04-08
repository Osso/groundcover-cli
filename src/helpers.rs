use anyhow::{Result, bail};
use chrono::Duration;
use serde_json::Value;
use std::process::Command;

use crate::client::Client;
use crate::config::Config;

pub fn parse_duration(s: &str) -> Result<Duration> {
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

pub fn null_to_dash(s: &str) -> &str {
    if s == "\\N" { "-" } else { s }
}

pub fn fetch_groundcover_output(args: &[&str]) -> Result<String> {
    let output = Command::new("groundcover").args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("groundcover {} failed: {}", args.join(" "), stderr);
    }
    Ok(String::from_utf8(output.stdout)?)
}

pub fn fetch_api_key() -> Result<String> {
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

pub fn fetch_grafana_token() -> Result<String> {
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

pub fn build_client(config: &Config) -> Result<Client> {
    let api_key = config.api_key().ok_or_else(|| {
        anyhow::anyhow!("No API key configured. Run: groundcover-cli config --fetch")
    })?;
    Client::new(api_key.to_string())
}

pub async fn print_query(client: &Client, sql: &str, json: bool) -> Result<()> {
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

pub fn filter_by_title<'a>(items: &'a [Value], filter: Option<&str>) -> Vec<&'a Value> {
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
