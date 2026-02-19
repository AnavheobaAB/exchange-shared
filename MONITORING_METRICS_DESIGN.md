# Monitoring & Metrics System Design

## Executive Summary

This document outlines a comprehensive observability strategy for the exchange platform using Prometheus metrics, following industry best practices including the RED method (Rate, Errors, Duration) for services and proper SLI/SLO/Error Budget management.

## 1. Monitoring Philosophy

### RED Method for Services
Track three critical metrics for every service endpoint:
- **Rate**: Requests per second (throughput)
- **Errors**: Failed requests percentage
- **Duration**: Request latency distribution (P50, P95, P99)

### USE Method for Resources
Track resource utilization:
- **Utilization**: Percentage of resource capacity used
- **Saturation**: Queue depth or wait time
- **Errors**: Resource-level errors

## 2. Metric Types & Mathematical Foundations

### 2.1 Counters
**Definition**: Monotonically increasing value that only goes up (or resets to zero).

**Use Cases**:
- Total requests processed
- Total errors encountered
- Total bytes transferred
- Total swaps completed

**Mathematical Properties**:
```
Rate calculation: rate(counter[5m])
Increase over time: increase(counter[1h])
```

**Best Practices**:
- Always use `_total` suffix (e.g., `http_requests_total`)
- Track totals, not current values
- Use `rate()` or `irate()` for per-second rates

### 2.2 Gauges
**Definition**: Value that can go up or down arbitrarily.

**Use Cases**:
- Current active connections
- Memory usage
- Queue depth
- Cache size
- RPC endpoint health scores

**Mathematical Properties**:
```
Current value: gauge_metric
Average over time: avg_over_time(gauge[5m])
Max/Min: max_over_time(gauge[5m])
```

**Best Practices**:
- Use for snapshots of current state
- Avoid high-cardinality labels
- Consider using histograms for distributions

### 2.3 Histograms
**Definition**: Samples observations and counts them in configurable buckets.

**Use Cases**:
- Request duration/latency
- Response sizes
- Database query times
- Payout processing time

**Mathematical Properties**:
```
Percentile calculation: histogram_quantile(0.95, rate(metric_bucket[5m]))
Average: rate(metric_sum[5m]) / rate(metric_count[5m])
```

**Bucket Design**:
```rust
// Exponential buckets for latency (milliseconds)
[10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000]

// Linear buckets for response sizes (bytes)
[100, 500, 1000, 5000, 10000, 50000, 100000, 500000, 1000000]
```

**Advantages**:
- Server-side aggregation across instances
- Flexible percentile calculation
- Lower client-side CPU overhead

**Disadvantages**:
- Approximate percentiles (bucket-based)
- Fixed bucket boundaries
- Higher storage cost

### 2.4 Summaries
**Definition**: Similar to histograms but calculates quantiles client-side.

**Use Cases**:
- When exact percentiles are required
- Single-instance metrics
- Low-volume, high-precision measurements

**Mathematical Properties**:
```
Pre-calculated quantiles: metric{quantile="0.95"}
Count: metric_count
Sum: metric_sum
```

**Advantages**:
- Exact percentiles
- Lower storage cost
- No bucket configuration needed

**Disadvantages**:
- Cannot aggregate across instances
- Higher client-side CPU overhead
- Fixed quantiles at instrumentation time

**Recommendation**: Use histograms for most cases; summaries only when exact percentiles are critical and aggregation isn't needed.

## 3. Cardinality Management

### 3.1 Cardinality Explosion Problem

**Definition**: Total unique time series = Product of all label value combinations

**Example**:
```
metric{method="GET", endpoint="/api/swap", status="200", user_id="12345"}
```

If you have:
- 10 methods
- 50 endpoints
- 10 status codes
- 100,000 users

Total series = 10 × 50 × 10 × 100,000 = 500,000,000 time series! 💥

### 3.2 Cardinality Best Practices

**Rule 1: Keep cardinality below 10 per label**
```rust
// ❌ BAD: Unbounded cardinality
http_requests_total{user_id="12345"}  // Millions of users

// ✅ GOOD: Bounded cardinality
http_requests_total{user_tier="premium"}  // 3-5 tiers
```

**Rule 2: Avoid high-cardinality labels**
```rust
// ❌ BAD
- user_id, session_id, request_id
- email, ip_address
- timestamp, uuid

// ✅ GOOD
- method (GET, POST, PUT, DELETE)
- status_code (200, 400, 500)
- endpoint (/api/swap, /api/estimate)
- chain (ethereum, bitcoin, solana)
```

**Rule 3: Use aggregation for high-cardinality data**
```rust
// Instead of per-user metrics, aggregate by tier
swap_count_total{user_tier="premium"} 150
swap_count_total{user_tier="standard"} 1200
swap_count_total{user_tier="free"} 5000
```

**Rule 4: Total cardinality target**
- < 10,000 series: Excellent
- 10,000 - 100,000: Good
- 100,000 - 1,000,000: Acceptable with tuning
- > 1,000,000: Investigate optimization

### 3.3 Label Naming Conventions

```rust
// Format: <namespace>_<subsystem>_<name>_<unit>_<suffix>

// ✅ GOOD
http_request_duration_seconds
swap_processing_time_milliseconds
cache_hit_ratio
rpc_health_score

// ❌ BAD
RequestDuration  // Use snake_case
request_time     // Missing unit
swap_count       // Missing _total suffix for counter
```

## 4. Service Level Indicators (SLIs)

### 4.1 SLI Definition

**SLI**: Quantitative measure of service performance from user perspective.

**Key SLIs for Exchange Platform**:

#### Availability SLI
```
Availability = (Successful Requests / Total Requests) × 100%

SLI = (http_requests_total{status!~"5.."} / http_requests_total) × 100
```

#### Latency SLI
```
Latency SLI = Percentage of requests faster than threshold

P95 < 500ms: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

#### Correctness SLI
```
Correctness = (Successful Swaps / Total Swaps) × 100%

SLI = (swap_completed_total / swap_initiated_total) × 100
```

### 4.2 Service Level Objectives (SLOs)

**SLO**: Target value or range for an SLI.

**Recommended SLOs**:

```yaml
Availability:
  - API Endpoints: 99.9% (43.2 min downtime/month)
  - Swap Processing: 99.5% (3.6 hours downtime/month)
  - RPC Endpoints: 99.0% (7.2 hours downtime/month)

Latency:
  - P95 API Response: < 500ms
  - P99 API Response: < 2000ms
  - P95 Swap Processing: < 30 seconds
  - P99 Swap Processing: < 120 seconds

Correctness:
  - Swap Success Rate: > 95%
  - Payout Success Rate: > 98%
  - Price Accuracy: ±2% of market rate
```

### 4.3 Error Budget Calculation

**Error Budget**: Amount of unreliability you can afford while meeting SLO.

**Formula**:
```
Error Budget = 100% - SLO%

For 99.9% SLO:
Error Budget = 100% - 99.9% = 0.1%
```

**Time-based Error Budget**:
```
Monthly Error Budget (99.9% SLO):
= 30 days × 24 hours × 60 minutes × 0.1%
= 43,200 minutes × 0.001
= 43.2 minutes of downtime per month
```

**Request-based Error Budget**:
```
If you serve 10M requests/month with 99.9% SLO:
Error Budget = 10,000,000 × 0.001 = 10,000 failed requests allowed
```

**Error Budget Consumption Rate**:
```
Burn Rate = (Actual Error Rate) / (Error Budget Rate)

If SLO is 99.9% (0.1% error budget):
- Current error rate: 0.5%
- Burn Rate = 0.5% / 0.1% = 5x
- At this rate, budget exhausted in: 30 days / 5 = 6 days
```

**Alerting Thresholds**:
```
Fast Burn (1 hour window):
  - Burn rate > 14.4x → Alert immediately
  - Consumes 2% of monthly budget per hour

Slow Burn (6 hour window):
  - Burn rate > 6x → Warning
  - Consumes 2% of monthly budget per 6 hours
```

## 5. Metrics Specification for Exchange Platform

### 5.1 HTTP Request Metrics

```rust
// Counter: Total HTTP requests
http_requests_total{
    method="GET|POST|PUT|DELETE",
    endpoint="/api/swap|/api/estimate|/api/pairs",
    status="200|400|500",
}

// Histogram: Request duration
http_request_duration_seconds{
    method="GET|POST|PUT|DELETE",
    endpoint="/api/swap|/api/estimate|/api/pairs",
}
// Buckets: [0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]

// Histogram: Response size
http_response_size_bytes{
    method="GET|POST|PUT|DELETE",
    endpoint="/api/swap|/api/estimate|/api/pairs",
}
// Buckets: [100, 500, 1000, 5000, 10000, 50000, 100000, 500000, 1000000]
```

### 5.2 Swap Metrics

```rust
// Counter: Swap lifecycle events
swap_initiated_total{
    base_currency="BTC|ETH|USDT",
    quote_currency="BTC|ETH|USDT",
    provider="trocador|other",
}

swap_completed_total{
    base_currency="BTC|ETH|USDT",
    quote_currency="BTC|ETH|USDT",
    provider="trocador|other",
}

swap_failed_total{
    base_currency="BTC|ETH|USDT",
    quote_currency="BTC|ETH|USDT",
    provider="trocador|other",
    reason="timeout|rpc_error|insufficient_funds|validation_error",
}

// Histogram: Swap processing time
swap_processing_duration_seconds{
    base_currency="BTC|ETH|USDT",
    quote_currency="BTC|ETH|USDT",
    provider="trocador|other",
}
// Buckets: [1, 5, 10, 30, 60, 120, 300, 600, 1800, 3600]

// Histogram: Swap amounts (in USD equivalent)
swap_amount_usd{
    base_currency="BTC|ETH|USDT",
    quote_currency="BTC|ETH|USDT",
}
// Buckets: [10, 50, 100, 500, 1000, 5000, 10000, 50000, 100000]

// Gauge: Active swaps
swap_active_count{
    status="pending|processing|awaiting_payout",
}
```

### 5.3 Payout Metrics

```rust
// Counter: Payout events
payout_initiated_total{
    chain="ethereum|bitcoin|solana|polygon",
    currency="BTC|ETH|USDT|SOL",
}

payout_completed_total{
    chain="ethereum|bitcoin|solana|polygon",
    currency="BTC|ETH|USDT|SOL",
}

payout_failed_total{
    chain="ethereum|bitcoin|solana|polygon",
    currency="BTC|ETH|USDT|SOL",
    reason="insufficient_gas|rpc_error|invalid_address|timeout",
}

// Histogram: Payout processing time
payout_duration_seconds{
    chain="ethereum|bitcoin|solana|polygon",
    currency="BTC|ETH|USDT|SOL",
}
// Buckets: [1, 5, 10, 30, 60, 120, 300, 600]

// Histogram: Gas costs (in native currency)
payout_gas_cost{
    chain="ethereum|bitcoin|solana|polygon",
}
// Buckets: [0.0001, 0.001, 0.01, 0.1, 1, 10, 100]
```

### 5.4 RPC Health Metrics

```rust
// Gauge: RPC endpoint health score (0.0 to 1.0)
rpc_endpoint_health_score{
    chain="ethereum|bitcoin|solana|polygon",
    endpoint="primary|backup1|backup2",
}

// Counter: RPC requests
rpc_requests_total{
    chain="ethereum|bitcoin|solana|polygon",
    endpoint="primary|backup1|backup2",
    method="eth_blockNumber|eth_sendRawTransaction|getblockcount",
    status="success|failure",
}

// Histogram: RPC latency
rpc_request_duration_seconds{
    chain="ethereum|bitcoin|solana|polygon",
    endpoint="primary|backup1|backup2",
    method="eth_blockNumber|eth_sendRawTransaction|getblockcount",
}
// Buckets: [0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]

// Gauge: Circuit breaker state (0=closed, 1=half-open, 2=open)
rpc_circuit_breaker_state{
    chain="ethereum|bitcoin|solana|polygon",
    endpoint="primary|backup1|backup2",
}

// Gauge: Block height lag
rpc_block_height_lag{
    chain="ethereum|bitcoin|solana|polygon",
    endpoint="primary|backup1|backup2",
}
```

### 5.5 Cache Metrics

```rust
// Counter: Cache operations
cache_operations_total{
    cache="redis|memory",
    operation="get|set|delete",
    result="hit|miss|error",
}

// Gauge: Cache hit ratio (calculated)
cache_hit_ratio{
    cache="redis|memory",
    key_pattern="rate:*|estimate:*|gas:*",
}

// Gauge: Cache size
cache_size_bytes{
    cache="redis|memory",
}

// Gauge: Cache entry count
cache_entries_total{
    cache="redis|memory",
}

// Histogram: Cache operation latency
cache_operation_duration_seconds{
    cache="redis|memory",
    operation="get|set|delete",
}
// Buckets: [0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
```

### 5.6 Database Metrics

```rust
// Counter: Database queries
db_queries_total{
    operation="select|insert|update|delete",
    table="swaps|currencies|trading_pairs|users",
    status="success|error",
}

// Histogram: Query duration
db_query_duration_seconds{
    operation="select|insert|update|delete",
    table="swaps|currencies|trading_pairs|users",
}
// Buckets: [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]

// Gauge: Connection pool stats
db_connections_active
db_connections_idle
db_connections_max
```

### 5.7 Business Metrics

```rust
// Counter: Revenue tracking
revenue_total_usd{
    currency="BTC|ETH|USDT",
    fee_type="commission|gas_markup",
}

// Gauge: Total Value Locked (TVL)
tvl_usd{
    currency="BTC|ETH|USDT",
}

// Counter: User activity
user_swaps_total{
    user_tier="free|standard|premium",
}

// Histogram: Commission earned per swap
commission_per_swap_usd{
    currency_pair="BTC_ETH|ETH_USDT",
}
// Buckets: [0.1, 0.5, 1, 5, 10, 50, 100, 500]
```

## 6. Implementation Architecture

### 6.1 Rust Crates

```toml
[dependencies]
# Prometheus metrics
prometheus = "0.13"
# Or use metrics ecosystem
metrics = "0.23"
metrics-exporter-prometheus = "0.15"

# Axum integration
axum-prometheus = "0.7"
# Or manual middleware
tower-http = { version = "0.6", features = ["trace"] }
```

### 6.2 Metrics Registry Pattern

```rust
use prometheus::{Registry, Counter, Histogram, Gauge, HistogramOpts, Opts};
use std::sync::Arc;

pub struct MetricsRegistry {
    registry: Registry,
    
    // HTTP metrics
    pub http_requests_total: Counter,
    pub http_request_duration: Histogram,
    
    // Swap metrics
    pub swap_initiated_total: Counter,
    pub swap_completed_total: Counter,
    pub swap_failed_total: Counter,
    pub swap_processing_duration: Histogram,
    
    // RPC metrics
    pub rpc_health_score: Gauge,
    pub rpc_requests_total: Counter,
    pub rpc_request_duration: Histogram,
    
    // Cache metrics
    pub cache_hit_ratio: Gauge,
    pub cache_operations_total: Counter,
}

impl MetricsRegistry {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let registry = Registry::new();
        
        // Initialize metrics with proper naming and labels
        let http_requests_total = Counter::with_opts(
            Opts::new("http_requests_total", "Total HTTP requests")
                .namespace("exchange")
        )?;
        registry.register(Box::new(http_requests_total.clone()))?;
        
        // ... initialize other metrics
        
        Ok(Self {
            registry,
            http_requests_total,
            // ... other metrics
        })
    }
    
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}
```

### 6.3 Middleware Integration

```rust
use axum::{
    Router,
    routing::get,
    middleware,
};
use tower_http::trace::TraceLayer;

async fn metrics_handler(
    State(metrics): State<Arc<MetricsRegistry>>,
) -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = metrics.registry().gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

pub fn create_router(metrics: Arc<MetricsRegistry>) -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .layer(middleware::from_fn(metrics_middleware))
        .with_state(metrics)
}
```

## 7. Grafana Dashboard Design

### 7.1 Dashboard Structure

**Overview Dashboard**:
- Service health (RED metrics)
- Error budget consumption
- Active swaps gauge
- Revenue metrics

**Swap Performance Dashboard**:
- Swap success rate by currency pair
- Processing time percentiles (P50, P95, P99)
- Failure reasons breakdown
- Volume by provider

**RPC Health Dashboard**:
- Endpoint health scores
- Circuit breaker states
- Latency heatmap
- Failover events

**Cache Performance Dashboard**:
- Hit ratio by cache type
- Operation latency
- Memory usage
- Eviction rate

### 7.2 Key PromQL Queries

```promql
# Availability SLI (last 30 days)
sum(rate(http_requests_total{status!~"5.."}[30d])) 
/ 
sum(rate(http_requests_total[30d])) * 100

# P95 Latency
histogram_quantile(0.95, 
  rate(http_request_duration_seconds_bucket[5m])
)

# Error Budget Remaining (99.9% SLO)
(1 - (
  sum(rate(http_requests_total{status=~"5.."}[30d])) 
  / 
  sum(rate(http_requests_total[30d]))
)) / 0.001 * 100

# Swap Success Rate
sum(rate(swap_completed_total[5m])) 
/ 
sum(rate(swap_initiated_total[5m])) * 100

# Cache Hit Ratio
sum(rate(cache_operations_total{result="hit"}[5m])) 
/ 
sum(rate(cache_operations_total{operation="get"}[5m])) * 100

# RPC Failover Rate
sum(rate(rpc_requests_total{endpoint!="primary"}[5m])) 
/ 
sum(rate(rpc_requests_total[5m])) * 100
```

## 8. Alerting Rules

### 8.1 Critical Alerts

```yaml
groups:
  - name: critical
    interval: 30s
    rules:
      - alert: HighErrorRate
        expr: |
          sum(rate(http_requests_total{status=~"5.."}[5m])) 
          / 
          sum(rate(http_requests_total[5m])) > 0.05
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }}"
      
      - alert: ErrorBudgetBurnRateFast
        expr: |
          (
            sum(rate(http_requests_total{status=~"5.."}[1h])) 
            / 
            sum(rate(http_requests_total[1h]))
          ) / 0.001 > 14.4
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Error budget burning too fast"
          description: "At current rate, monthly budget exhausted in {{ $value | humanizeDuration }}"
      
      - alert: HighLatency
        expr: |
          histogram_quantile(0.95, 
            rate(http_request_duration_seconds_bucket[5m])
          ) > 2.0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "P95 latency above threshold"
          description: "P95 latency is {{ $value }}s"
      
      - alert: AllRPCEndpointsDown
        expr: |
          sum(rpc_circuit_breaker_state == 2) by (chain) 
          == 
          count(rpc_circuit_breaker_state) by (chain)
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "All RPC endpoints down for {{ $labels.chain }}"
```

### 8.2 Warning Alerts

```yaml
      - alert: LowCacheHitRatio
        expr: |
          sum(rate(cache_operations_total{result="hit"}[5m])) 
          / 
          sum(rate(cache_operations_total{operation="get"}[5m])) < 0.7
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Cache hit ratio below 70%"
      
      - alert: SlowSwapProcessing
        expr: |
          histogram_quantile(0.95, 
            rate(swap_processing_duration_seconds_bucket[5m])
          ) > 60
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Swap processing taking longer than expected"
```

## 9. Implementation Phases

### Phase 1: Core Metrics (Week 1)
- Set up Prometheus registry
- Implement HTTP request metrics
- Add /metrics endpoint
- Basic Grafana dashboard

### Phase 2: Business Metrics (Week 2)
- Swap lifecycle metrics
- Payout metrics
- Revenue tracking
- SLI/SLO dashboards

### Phase 3: Infrastructure Metrics (Week 3)
- RPC health metrics
- Cache metrics
- Database metrics
- Circuit breaker monitoring

### Phase 4: Alerting & SLOs (Week 4)
- Configure alerting rules
- Error budget tracking
- On-call runbooks
- Incident response procedures

## 10. Cost & Performance Considerations

### Storage Estimates
```
Metrics per second: ~500
Samples per day: 500 × 86,400 = 43,200,000
Storage per sample: ~2 bytes (compressed)
Daily storage: ~86 MB
Monthly storage: ~2.6 GB
Yearly storage: ~31 GB
```

### Query Performance
- Keep queries under 1000 series
- Use recording rules for expensive queries
- Set appropriate retention (30-90 days)
- Use downsampling for long-term storage

### Cardinality Budget
```
Target: < 100,000 active series
Current estimate:
- HTTP metrics: ~500 series
- Swap metrics: ~1,000 series
- RPC metrics: ~200 series
- Cache metrics: ~50 series
- DB metrics: ~100 series
Total: ~2,000 series ✅
```

## 11. References

- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [RED Method](https://grafana.com/blog/2018/08/02/the-red-method-how-to-instrument-your-services/)
- [SRE Book - SLIs, SLOs, SLAs](https://sre.google/sre-book/service-level-objectives/)
- [Histograms and Summaries](https://prometheus.io/docs/practices/histograms/)
- [Label Best Practices](https://prometheus.io/docs/practices/naming/)
