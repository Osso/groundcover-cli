use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde_json::Value;

const DATASOURCE_BASE_URL: &str = "https://ds.groundcover.com";

pub struct Client {
    http: HttpClient,
    api_key: String,
}

impl Client {
    pub fn new(api_key: String) -> Result<Self> {
        let http = HttpClient::builder()
            .build()
            .context("Failed to create HTTP client")?;
        Ok(Self { http, api_key })
    }

    /// Query ClickHouse (logs, traces, events)
    pub async fn query_clickhouse(&self, sql: &str) -> Result<String> {
        let response = self
            .http
            .post(DATASOURCE_BASE_URL)
            .header("X-ClickHouse-Key", &self.api_key)
            .body(sql.to_string())
            .send()
            .await
            .context("Failed to send ClickHouse query")?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("ClickHouse query failed ({}): {}", status, text);
        }

        Ok(text)
    }

    /// Query ClickHouse and parse as JSON
    pub async fn query_clickhouse_json(&self, sql: &str) -> Result<Value> {
        // Append FORMAT JSON if not already present
        let sql = if sql.to_uppercase().contains("FORMAT ") {
            sql.to_string()
        } else {
            format!("{} FORMAT JSON", sql)
        };

        let text = self.query_clickhouse(&sql).await?;
        let value: Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse ClickHouse response as JSON: {}", text))?;
        Ok(value)
    }

    /// Query VictoriaMetrics (metrics) using PromQL
    pub async fn query_metrics(
        &self,
        query: &str,
        start: i64,
        end: i64,
        step: Option<&str>,
    ) -> Result<Value> {
        let step = step.unwrap_or("60s");
        let url = format!("{}/datasources/prometheus/api/v1/query_range", DATASOURCE_BASE_URL);

        let response = self
            .http
            .get(&url)
            .header("apikey", &self.api_key)
            .query(&[
                ("query", query),
                ("start", &start.to_string()),
                ("end", &end.to_string()),
                ("step", step),
            ])
            .send()
            .await
            .context("Failed to send metrics query")?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("Metrics query failed ({}): {}", status, text);
        }

        let value: Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse metrics response: {}", text))?;
        Ok(value)
    }

    /// Query VictoriaMetrics instant query
    #[allow(dead_code)]
    pub async fn query_metrics_instant(&self, query: &str) -> Result<Value> {
        let url = format!("{}/datasources/prometheus/api/v1/query", DATASOURCE_BASE_URL);

        let response = self
            .http
            .get(&url)
            .header("apikey", &self.api_key)
            .query(&[("query", query)])
            .send()
            .await
            .context("Failed to send metrics query")?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            anyhow::bail!("Metrics query failed ({}): {}", status, text);
        }

        let value: Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse metrics response: {}", text))?;
        Ok(value)
    }
}
