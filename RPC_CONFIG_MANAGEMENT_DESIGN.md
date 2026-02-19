# RPC Configuration Management - Design & Implementation Guide

## Research Summary

Based on industry best practices for resilient API integration, circuit breaker patterns, and load balancing algorithms, here's the optimal mathematical and architectural approach for RPC configuration management.

## Mathematical Approaches

### 1. Health Score Calculation

**Composite Health Score Formula**:
```
Health_Score = w1 * Availability_Score 
             + w2 * Latency_Score 
             + w3 * Success_Rate_Score
             + w4 * Block_Height_Score

Where:
- w1, w2, w3, w4 are weights (sum to 1.0)
- Recommended: w1=0.3, w2=0.3, w3=0.3, w4=0.1
```

**Component Scores** (normalized to 0-1):

```rust
// Availability Score (exponential moving average)
Availability_Score = EMA(is_responsive)
EMA_new = α * current_value + (1 - α) * EMA_old
α = 0.2  // Smoothing factor

// Latency Score (inverse normalized)
Latency_Score = 1 - min(latency_ms / max_acceptable_latency, 1.0)
max_acceptable_latency = 5000ms  // 5 seconds

// Success Rate Score (last N requests)
Success_Rate_Score = successful_requests / total_requests
window_size = 100  // Last 100 requests

// Block Height Score (freshness)
Block_Height_Score = 1 - min((current_time - last_block_time) / max_staleness, 1.0)
max_staleness = 60000ms  // 1 minute
```

### 2. Weighted Round Robin Selection

**Weight Calculation**:
```
Weight = floor(Health_Score * 100)

Example:
- RPC A: Health_Score = 0.95 → Weight = 95
- RPC B: Health_Score = 0.80 → Weight = 80
- RPC C: Health_Score = 0.60 → Weight = 60
```

**Selection Algorithm**:
```rust
// Smooth Weighted Round Robin (SWRR)
current_weight[i] += effective_weight[i]
selected = index_of_max(current_weight)
current_weight[selected] -= sum(effective_weight)
```

### 3. Circuit Breaker State Machine

**States**:
- **CLOSED**: Normal operation (0-20% failure rate)
- **HALF_OPEN**: Testing recovery (after timeout)
- **OPEN**: Blocking requests (>20% failure rate)

**Transition Formula**:
```
Failure_Rate = failures / (successes + failures)

CLOSED → OPEN:
  if Failure_Rate > threshold (0.2) AND failures > min_requests (5)

OPEN → HALF_OPEN:
  if time_since_open > timeout (30s)

HALF_OPEN → CLOSED:
  if consecutive_successes >= required (3)

HALF_OPEN → OPEN:
  if any_failure
```

### 4. Exponential Backoff with Jitter

**Formula**:
```
delay = min(base_delay * 2^attempt, max_delay) + jitter

Where:
- base_delay = 100ms
- max_delay = 30000ms (30 seconds)
- jitter = random(0, delay * 0.1)  // ±10% randomization

Example progression:
- Attempt 1: 100ms + jitter
- Attempt 2: 200ms + jitter
- Attempt 3: 400ms + jitter
- Attempt 4: 800ms + jitter
- Attempt 5: 1600ms + jitter
- ...
- Attempt 10+: 30000ms + jitter (capped)
```

**Jitter prevents thundering herd**:
```rust
fn calculate_backoff(attempt: u32) -> Duration {
    let base_ms = 100;
    let max_ms = 30_000;
    let delay_ms = std::cmp::min(base_ms * 2_u64.pow(attempt), max_ms);
    
    // Add ±10% jitter
    let jitter = rand::random::<f64>() * 0.2 - 0.1;  // -0.1 to +0.1
    let final_delay = (delay_ms as f64 * (1.0 + jitter)) as u64;
    
    Duration::from_millis(final_delay)
}
```

### 5. Adaptive Timeout Calculation

**Dynamic Timeout Formula**:
```
timeout = max(
    min_timeout,
    min(
        p95_latency * 1.5,  // 1.5x the 95th percentile
        max_timeout
    )
)

Where:
- min_timeout = 1000ms
- max_timeout = 30000ms
- p95_latency = 95th percentile of recent requests
```

**P95 Calculation** (using histogram):
```rust
// Maintain latency histogram
let mut histogram = vec![0; 100];  // 100 buckets, each 100ms
histogram[latency_ms / 100] += 1;

// Calculate P95
let total = histogram.iter().sum();
let p95_index = (total as f64 * 0.95) as usize;
let mut cumulative = 0;
for (i, &count) in histogram.iter().enumerate() {
    cumulative += count;
    if cumulative >= p95_index {
        return (i * 100) as u64;  // P95 latency in ms
    }
}
```

## Architecture Design

### 1. RPC Configuration Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub chain: String,              // "ethereum", "bitcoin", "solana"
    pub endpoints: Vec<RpcEndpoint>,
    pub strategy: LoadBalancingStrategy,
    pub health_check_interval: Duration,
    pub circuit_breaker_config: CircuitBreakerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEndpoint {
    pub url: String,
    pub priority: u8,               // 1 (primary) to 10 (backup)
    pub weight: u32,                // For weighted round robin
    pub max_requests_per_second: Option<u32>,
    pub timeout_ms: u64,
    pub auth: Option<RpcAuth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcAuth {
    ApiKey(String),
    Bearer(String),
    Basic { username: String, password: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    WeightedRoundRobin,
    LeastLatency,
    HealthScoreBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: f64,     // 0.2 = 20% failure rate
    pub min_requests: u32,          // Minimum requests before opening
    pub timeout_seconds: u64,       // Time before attempting recovery
    pub half_open_max_requests: u32, // Max requests in half-open state
}
```

### 2. Health Tracking Structure

```rust
#[derive(Debug, Clone)]
pub struct EndpointHealth {
    pub url: String,
    pub state: CircuitState,
    pub health_score: f64,          // 0.0 to 1.0
    
    // Metrics (sliding window)
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub latencies: VecDeque<u64>,   // Last N latencies
    
    // Availability tracking
    pub last_success: Option<Instant>,
    pub last_failure: Option<Instant>,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    
    // Block height tracking (for blockchain RPCs)
    pub last_block_height: Option<u64>,
    pub last_block_time: Option<Instant>,
    
    // Circuit breaker
    pub circuit_opened_at: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    HalfOpen,
    Open,
}
```

### 3. RPC Manager Interface

```rust
pub struct RpcManager {
    configs: HashMap<String, RpcConfig>,
    health_tracker: Arc<RwLock<HashMap<String, EndpointHealth>>>,
    client: reqwest::Client,
    redis: Option<RedisService>,
}

impl RpcManager {
    /// Select best endpoint based on health scores and strategy
    pub async fn select_endpoint(&self, chain: &str) -> Result<String, RpcError>;
    
    /// Execute RPC call with automatic failover
    pub async fn call<T>(&self, chain: &str, method: &str, params: Value) -> Result<T, RpcError>;
    
    /// Update health metrics after request
    pub async fn record_result(&self, url: &str, latency: Duration, success: bool);
    
    /// Background health check loop
    pub async fn health_check_loop(&self);
    
    /// Get current health status for all endpoints
    pub async fn get_health_status(&self, chain: &str) -> Vec<EndpointHealthStatus>;
}
```

## Implementation Strategy

### Phase 1: Configuration Loader

```rust
// Load from config file
pub fn load_rpc_config(path: &str) -> Result<HashMap<String, RpcConfig>, Error> {
    let content = std::fs::read_to_string(path)?;
    let configs: HashMap<String, RpcConfig> = serde_json::from_str(&content)?;
    Ok(configs)
}

// Example config.json
{
  "ethereum": {
    "chain": "ethereum",
    "endpoints": [
      {
        "url": "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY",
        "priority": 1,
        "weight": 100,
        "timeout_ms": 5000,
        "auth": { "ApiKey": "YOUR_KEY" }
      },
      {
        "url": "https://mainnet.infura.io/v3/YOUR_KEY",
        "priority": 2,
        "weight": 80,
        "timeout_ms": 5000,
        "auth": { "ApiKey": "YOUR_KEY" }
      },
      {
        "url": "https://rpc.ankr.com/eth",
        "priority": 3,
        "weight": 60,
        "timeout_ms": 10000,
        "auth": null
      }
    ],
    "strategy": "HealthScoreBased",
    "health_check_interval": 30,
    "circuit_breaker_config": {
      "failure_threshold": 0.2,
      "min_requests": 5,
      "timeout_seconds": 30,
      "half_open_max_requests": 3
    }
  }
}
```

### Phase 2: Health Checking

```rust
impl RpcManager {
    async fn check_endpoint_health(&self, url: &str) -> HealthCheckResult {
        let start = Instant::now();
        
        // Simple health check: eth_blockNumber or equivalent
        let result = self.client
            .post(url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1
            }))
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        
        let latency = start.elapsed();
        
        match result {
            Ok(response) if response.status().is_success() => {
                // Extract block height if available
                if let Ok(body) = response.json::<Value>().await {
                    let block_height = body["result"]
                        .as_str()
                        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());
                    
                    HealthCheckResult {
                        success: true,
                        latency,
                        block_height,
                    }
                } else {
                    HealthCheckResult {
                        success: false,
                        latency,
                        block_height: None,
                    }
                }
            }
            _ => HealthCheckResult {
                success: false,
                latency,
                block_height: None,
            }
        }
    }
    
    pub async fn health_check_loop(&self) {
        loop {
            for (chain, config) in &self.configs {
                for endpoint in &config.endpoints {
                    let result = self.check_endpoint_health(&endpoint.url).await;
                    self.update_health_metrics(&endpoint.url, result).await;
                }
            }
            
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}
```

### Phase 3: Endpoint Selection

```rust
impl RpcManager {
    pub async fn select_endpoint(&self, chain: &str) -> Result<String, RpcError> {
        let config = self.configs.get(chain)
            .ok_or(RpcError::ChainNotConfigured)?;
        
        let health = self.health_tracker.read().await;
        
        // Filter to only healthy endpoints (circuit not open)
        let mut candidates: Vec<_> = config.endpoints.iter()
            .filter(|ep| {
                health.get(&ep.url)
                    .map(|h| h.state != CircuitState::Open)
                    .unwrap_or(true)
            })
            .collect();
        
        if candidates.is_empty() {
            return Err(RpcError::NoHealthyEndpoints);
        }
        
        // Sort by priority first
        candidates.sort_by_key(|ep| ep.priority);
        
        // Apply strategy
        match config.strategy {
            LoadBalancingStrategy::HealthScoreBased => {
                // Select endpoint with highest health score
                candidates.sort_by(|a, b| {
                    let score_a = health.get(&a.url).map(|h| h.health_score).unwrap_or(0.0);
                    let score_b = health.get(&b.url).map(|h| h.health_score).unwrap_or(0.0);
                    score_b.partial_cmp(&score_a).unwrap()
                });
                Ok(candidates[0].url.clone())
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                // Implement SWRR algorithm
                self.weighted_round_robin_select(candidates, &health).await
            }
            LoadBalancingStrategy::LeastLatency => {
                // Select endpoint with lowest P95 latency
                candidates.sort_by(|a, b| {
                    let lat_a = health.get(&a.url).and_then(|h| h.calculate_p95()).unwrap_or(u64::MAX);
                    let lat_b = health.get(&b.url).and_then(|h| h.calculate_p95()).unwrap_or(u64::MAX);
                    lat_a.cmp(&lat_b)
                });
                Ok(candidates[0].url.clone())
            }
            LoadBalancingStrategy::RoundRobin => {
                // Simple round robin
                let index = self.get_next_index(chain).await % candidates.len();
                Ok(candidates[index].url.clone())
            }
        }
    }
}
```

### Phase 4: Automatic Failover

```rust
impl RpcManager {
    pub async fn call<T: DeserializeOwned>(
        &self,
        chain: &str,
        method: &str,
        params: Value,
    ) -> Result<T, RpcError> {
        let config = self.configs.get(chain)
            .ok_or(RpcError::ChainNotConfigured)?;
        
        let max_attempts = config.endpoints.len().min(3);  // Try up to 3 endpoints
        
        for attempt in 0..max_attempts {
            // Select endpoint
            let url = self.select_endpoint(chain).await?;
            
            // Execute request with timeout
            let start = Instant::now();
            let result = self.execute_rpc_call(&url, method, params.clone()).await;
            let latency = start.elapsed();
            
            match result {
                Ok(response) => {
                    // Record success
                    self.record_result(&url, latency, true).await;
                    return Ok(response);
                }
                Err(e) => {
                    // Record failure
                    self.record_result(&url, latency, false).await;
                    
                    // Check if we should retry
                    if attempt < max_attempts - 1 {
                        // Apply exponential backoff
                        let backoff = calculate_backoff(attempt);
                        tokio::time::sleep(backoff).await;
                        continue;
                    }
                    
                    return Err(e);
                }
            }
        }
        
        Err(RpcError::AllEndpointsFailed)
    }
}
```

## Configuration File Format

### rpc_config.json
```json
{
  "ethereum": {
    "chain": "ethereum",
    "endpoints": [
      {
        "url": "${ETHEREUM_RPC_PRIMARY}",
        "priority": 1,
        "weight": 100,
        "max_requests_per_second": 100,
        "timeout_ms": 5000,
        "auth": { "ApiKey": "${ALCHEMY_API_KEY}" }
      },
      {
        "url": "${ETHEREUM_RPC_SECONDARY}",
        "priority": 2,
        "weight": 80,
        "timeout_ms": 5000,
        "auth": { "ApiKey": "${INFURA_API_KEY}" }
      },
      {
        "url": "https://rpc.ankr.com/eth",
        "priority": 3,
        "weight": 60,
        "timeout_ms": 10000,
        "auth": null
      }
    ],
    "strategy": "HealthScoreBased",
    "health_check_interval": 30,
    "circuit_breaker_config": {
      "failure_threshold": 0.2,
      "min_requests": 5,
      "timeout_seconds": 30,
      "half_open_max_requests": 3
    }
  },
  "bitcoin": {
    "chain": "bitcoin",
    "endpoints": [
      {
        "url": "${BITCOIN_RPC_PRIMARY}",
        "priority": 1,
        "weight": 100,
        "timeout_ms": 10000,
        "auth": { "Basic": { "username": "${BTC_RPC_USER}", "password": "${BTC_RPC_PASS}" } }
      },
      {
        "url": "https://blockstream.info/api",
        "priority": 2,
        "weight": 60,
        "timeout_ms": 15000,
        "auth": null
      }
    ],
    "strategy": "WeightedRoundRobin",
    "health_check_interval": 60,
    "circuit_breaker_config": {
      "failure_threshold": 0.3,
      "min_requests": 3,
      "timeout_seconds": 60,
      "half_open_max_requests": 2
    }
  }
}
```

## Monitoring & Metrics

### Key Metrics to Track

1. **Per-Endpoint Metrics**:
   - Request count (total, success, failure)
   - Latency (min, max, avg, p50, p95, p99)
   - Circuit breaker state changes
   - Health score over time
   - Block height lag

2. **Per-Chain Metrics**:
   - Total requests
   - Failover count
   - Average failover time
   - Endpoint distribution (which endpoints are used most)

3. **System Metrics**:
   - Total RPC calls
   - Cache hit rate (if caching responses)
   - Cost per chain (if tracking API costs)

### Prometheus Metrics Example

```rust
use prometheus::{Counter, Histogram, Gauge, Registry};

pub struct RpcMetrics {
    pub requests_total: Counter,
    pub requests_duration: Histogram,
    pub endpoint_health_score: Gauge,
    pub circuit_breaker_state: Gauge,
    pub failover_count: Counter,
}
```

## Testing Strategy

### Unit Tests
- Health score calculation
- Circuit breaker state transitions
- Weighted round robin selection
- Exponential backoff calculation

### Integration Tests
- Endpoint failover scenarios
- Circuit breaker behavior under load
- Health check accuracy
- Configuration loading

### Chaos Testing
- Simulate endpoint failures
- Network latency injection
- Partial outages
- Recovery scenarios

## Implementation Checklist

- [ ] Create RPC configuration structure
- [ ] Implement configuration loader with env var substitution
- [ ] Build health tracking system
- [ ] Implement circuit breaker pattern
- [ ] Add exponential backoff with jitter
- [ ] Implement load balancing strategies
- [ ] Create automatic failover logic
- [ ] Add background health check loop
- [ ] Implement metrics collection
- [ ] Create admin API for health status
- [ ] Write comprehensive tests
- [ ] Add monitoring dashboards
- [ ] Document configuration format
- [ ] Create migration guide from env vars

## References

Content rephrased for compliance with licensing restrictions:

- Circuit breaker and retry patterns from [resilient API integration guide](https://newsdatahub.com/learning-center/article/resilient-api-integration-cheat-sheet)
- Load balancing algorithms from [Traefik blog](https://traefik.io/blog/choose-the-right-load-balancing-strategy)
- Health check patterns from [blockchain node architecture guides](https://www.chainscore.finance/)
- Exponential backoff with jitter from distributed systems best practices
- Weighted round robin implementation from load balancing research

## Next Steps

1. Review existing `.env.example` RPC URLs
2. Design configuration file structure
3. Implement basic RPC manager with failover
4. Add health checking
5. Implement circuit breaker
6. Add metrics and monitoring
7. Create admin endpoints
8. Write comprehensive tests
9. Document usage and migration
