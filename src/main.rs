#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod client;
#[cfg_attr(coverage_nightly, coverage(off))]
mod commands;
mod config;
mod helpers;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use client::GrafanaClient;
use commands::{alerts, api, clickhouse, grafana};
use config::Config;
use helpers::{build_client, fetch_api_key, fetch_grafana_token};

#[derive(Parser)]
#[command(name = "groundcover-cli")]
#[command(about = "Groundcover CLI - query logs, traces, and metrics")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// === Args Structs ===

#[derive(Args)]
struct LogsArgs {
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
}

#[derive(Args)]
struct TracesArgs {
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
}

#[derive(Args)]
struct EventsArgs {
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
}

#[derive(Args)]
struct MetricsArgs {
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
}

#[derive(Args)]
struct ApiArgs {
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
}

#[derive(Args)]
struct WorkloadsArgs {
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
}

#[derive(Args)]
struct AlertsArgs {
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
}

#[derive(Args)]
struct IssuesArgs {
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
}

// === Commands ===

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
    Logs(LogsArgs),
    /// Query traces from ClickHouse
    Traces(TracesArgs),
    /// Query Kubernetes events from ClickHouse
    Events(EventsArgs),
    /// Query metrics from VictoriaMetrics using PromQL
    Metrics(MetricsArgs),
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
    Api(ApiArgs),
    /// List workloads with metrics
    Workloads(WorkloadsArgs),
    /// Query alerts from ClickHouse
    Alerts(AlertsArgs),
    /// Query detected issues from traces
    Issues(IssuesArgs),
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

// === Config Handler ===

#[cfg_attr(coverage_nightly, coverage(off))]
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

// === Main ===

#[cfg_attr(coverage_nightly, coverage(off))]
async fn run_data_command(config: &Config, command: Commands) -> Result<()> {
    let client = build_client(config)?;
    match command {
        Commands::Logs(args) => clickhouse::run_logs(&client, args.into()).await,
        Commands::Traces(args) => clickhouse::run_traces(&client, args.into()).await,
        Commands::Events(args) => clickhouse::run_events(&client, args.into()).await,
        Commands::Metrics(args) => clickhouse::run_metrics(&client, args.into()).await,
        Commands::SqlClickhouse { query, json } => {
            clickhouse::run_sql_clickhouse(&client, query, json).await
        }
        Commands::Tables => clickhouse::run_tables(&client).await,
        Commands::Api(args) => api::run_api(&client, args.into()).await,
        Commands::Workloads(args) => api::run_workloads(&client, args.into()).await,
        Commands::Alerts(args) => alerts::run_alerts(&client, args.into()).await,
        Commands::Issues(args) => alerts::run_issues(&client, args.into()).await,
        Commands::Config { .. } | Commands::Grafana { .. } => unreachable!(),
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn run_grafana_command(config: &Config, command: GrafanaCommands) -> Result<()> {
    let token = config.grafana_token().ok_or_else(|| {
        anyhow::anyhow!("No Grafana token configured. Run: groundcover-cli config --fetch")
    })?;
    let client = GrafanaClient::new(token.to_string())?;
    grafana::run_grafana(&client, map_grafana_command(command)).await
}

#[tokio::main]
#[cfg_attr(coverage_nightly, coverage(off))]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    match cli.command {
        Commands::Config { api_key, fetch } => run_config(&mut config, api_key, fetch),
        Commands::Grafana { command } => run_grafana_command(&config, command).await,
        command => run_data_command(&config, command).await,
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn map_grafana_command(command: GrafanaCommands) -> grafana::GrafanaCommands {
    match command {
        GrafanaCommands::Datasources => grafana::GrafanaCommands::Datasources,
        GrafanaCommands::Datasource { uid } => grafana::GrafanaCommands::Datasource { uid },
        GrafanaCommands::Dashboards => grafana::GrafanaCommands::Dashboards,
        GrafanaCommands::Search { query } => grafana::GrafanaCommands::Search { query },
        GrafanaCommands::AlertRules { filter, json } => {
            grafana::GrafanaCommands::AlertRules { filter, json }
        }
        GrafanaCommands::Alerts { filter, json } => {
            grafana::GrafanaCommands::Alerts { filter, json }
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl From<LogsArgs> for clickhouse::LogsArgs {
    fn from(args: LogsArgs) -> Self {
        Self {
            since: args.since,
            workload: args.workload,
            namespace: args.namespace,
            level: args.level,
            grep: args.grep,
            limit: args.limit,
            json: args.json,
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl From<TracesArgs> for clickhouse::TracesArgs {
    fn from(args: TracesArgs) -> Self {
        Self {
            since: args.since,
            workload: args.workload,
            operation: args.operation,
            min_duration: args.min_duration,
            status: args.status,
            limit: args.limit,
            json: args.json,
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl From<EventsArgs> for clickhouse::EventsArgs {
    fn from(args: EventsArgs) -> Self {
        Self {
            since: args.since,
            namespace: args.namespace,
            event_type: args.event_type,
            reason: args.reason,
            limit: args.limit,
            json: args.json,
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl From<MetricsArgs> for clickhouse::MetricsArgs {
    fn from(args: MetricsArgs) -> Self {
        Self {
            query: args.query,
            since: args.since,
            step: args.step,
            json: args.json,
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl From<ApiArgs> for api::ApiArgs {
    fn from(args: ApiArgs) -> Self {
        Self {
            namespace: args.namespace,
            workload: args.workload,
            endpoint: args.endpoint,
            errors: args.errors,
            limit: args.limit,
            json: args.json,
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl From<WorkloadsArgs> for api::WorkloadsArgs {
    fn from(args: WorkloadsArgs) -> Self {
        Self {
            namespace: args.namespace,
            workload: args.workload,
            kind: args.kind,
            errors: args.errors,
            not_ready: args.not_ready,
            limit: args.limit,
            json: args.json,
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl From<AlertsArgs> for alerts::AlertsArgs {
    fn from(args: AlertsArgs) -> Self {
        Self {
            since: args.since,
            state: args.state,
            severity: args.severity,
            namespace: args.namespace,
            workload: args.workload,
            monitor: args.monitor,
            limit: args.limit,
            json: args.json,
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl From<IssuesArgs> for alerts::IssuesArgs {
    fn from(args: IssuesArgs) -> Self {
        Self {
            since: args.since,
            namespace: args.namespace,
            workload: args.workload,
            grep: args.grep,
            code: args.code,
            limit: args.limit,
            json: args.json,
        }
    }
}
