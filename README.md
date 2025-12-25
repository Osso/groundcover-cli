# groundcover-cli

CLI for querying [Groundcover](https://groundcover.com) observability data - logs, traces, metrics, issues, and alerts.

## Installation

```bash
cargo install --path .
```

Or build manually:

```bash
cargo build --release
# Binary at target/release/groundcover-cli
```

## Setup

First, authenticate with the Groundcover CLI (`groundcover auth login`), then fetch credentials:

```bash
groundcover-cli config --fetch
```

This stores the API key and Grafana token in `~/.config/groundcover/config.json`.

## Usage

### Logs

```bash
groundcover-cli logs                          # Recent logs (last 15m)
groundcover-cli logs -n 50                    # Limit to 50 results
groundcover-cli logs -s 1h                    # Last hour
groundcover-cli logs -w api                   # Filter by workload
groundcover-cli logs --namespace default      # Filter by namespace
groundcover-cli logs -g "error"               # Search text
groundcover-cli logs -l error                 # Filter by level
groundcover-cli logs --json                   # Output as JSON
```

### Issues

Query trace-detected issues (errors, exceptions, failed requests):

```bash
groundcover-cli issues                        # Recent issues
groundcover-cli issues -w api                 # Filter by workload
groundcover-cli issues --code 502             # Filter by return code
groundcover-cli issues -g "timeout"           # Search issue description
```

### Alerts

```bash
groundcover-cli alerts                        # Recent alerts
groundcover-cli alerts --state Alerting       # Only alerting
groundcover-cli alerts --severity S1          # Filter by severity
groundcover-cli alerts -m "cpu"               # Filter by monitor name
```

### API Endpoints

List API endpoints with metrics (RPS, error rate, latency):

```bash
groundcover-cli api                           # Top endpoints by RPS
groundcover-cli api -e "/v1/releases"         # Filter by endpoint path
groundcover-cli api -w nginx                  # Filter by service
groundcover-cli api --errors                  # Only with errors
```

### Workloads

```bash
groundcover-cli workloads                     # All workloads with metrics
groundcover-cli workloads --namespace default # Filter by namespace
groundcover-cli workloads --errors            # Only with errors
groundcover-cli workloads --not-ready         # Only not ready
```

### Traces

```bash
groundcover-cli traces -s 1h                  # Traces from last hour
groundcover-cli traces -w api                 # Filter by workload
groundcover-cli traces --errors               # Only error traces
```

### Events

Kubernetes events:

```bash
groundcover-cli events                        # Recent events
groundcover-cli events -t Warning             # Only warnings
groundcover-cli events --reason BackOff       # Filter by reason
```

### Metrics

Query VictoriaMetrics using PromQL:

```bash
groundcover-cli metrics "up" -s 1h
groundcover-cli metrics "container_cpu_usage_seconds_total" -s 30m --step 5m
```

### Raw ClickHouse SQL

```bash
groundcover-cli sql-clickhouse "SELECT * FROM logs LIMIT 5"
groundcover-cli tables                        # List available tables
```

### Grafana

```bash
groundcover-cli grafana datasources           # List datasources
groundcover-cli grafana dashboards            # List dashboards
groundcover-cli grafana search "cpu"          # Search dashboards
```

## Common Options

| Option | Description |
|--------|-------------|
| `-s, --since` | Time range (e.g., 15m, 1h, 24h) |
| `-n, --limit` | Maximum results |
| `-w, --workload` | Filter by workload name |
| `--namespace` | Filter by Kubernetes namespace |
| `--json` | Output as JSON |

## Data Sources

- **ClickHouse**: Logs, traces, events, issues, alerts
- **VictoriaMetrics**: Metrics (PromQL)
- **Grafana API**: Datasources, dashboards

## License

MIT
