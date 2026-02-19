# Real Gas Price Fetching - Optimal Design

## Executive Summary

This document outlines the optimal design for implementing real-time gas price fetching across multiple blockchain networks (Ethereum, Bitcoin, Solana, etc.) based on academic research and industry best practices.

## Research Findings

### Key Academic Insights

1. **Gaussian Process Models** ([Bayesian Gas Price Prediction](https://ui.adsabs.harvard.edu/abs/arXiv:2305.00337))
   - Gaussian process models provide superior estimation during volatile periods
   - Accounts for time correlations between blocks
   - Outperforms traditional oracles (GasStation-Express, Geth) when transaction volumes fluctuate

2. **EIP-1559 Optimization** ([Optimal Dynamic Fees](https://ar5iv.labs.arxiv.org/html/2309.12735))
   - Linear-quadratic framework for optimal pricing
   - Trade-off between adjusting to persistent demand vs. robustness to noise
   - Kalman filter for belief updates on demand and price sensitivity
   - Regularization term controls price fluctuations

3. **Multi-Resource Pricing**
   - Cross-effects between resources (complementarity/substitutability)
   - Eigenresource decomposition for independent pricing
   - Model Predictive Control (MPC) for stochastic environments

### Industry Best Practices

1. **Blocknative Approach**
   - Real-time mempool monitoring
   - Machine learning pipelines for base fee prediction
   - Next-block and next-10-second predictions

2. **Multi-Chain Support**
   - EVM chains: `fee = gasUsed × (baseFee + priorityFee)`
   - UTXO chains: `fee = feeRate × txSize`
   - Solana: Fixed fee structure with priority fees
   - Different transaction types have different gas requirements

## Proposed Architecture

### 1. Multi-Tier Caching Strategy

```
┌─────────────────────────────────────────────────────────┐
│                   Gas Price Cache                        │
├─────────────────────────────────────────────────────────┤
│  Tier 1: Exact Cache (10s TTL)                          │
│  - Key: chain:txType:exact                              │
│  - Use: Immediate requests                              │
├─────────────────────────────────────────────────────────┤
│  Tier 2: Bucketed Cache (60s TTL)                       │
│  - Key: chain:txType:bucketed                           │
│  - Use: Reduce RPC load                                 │
├─────────────────────────────────────────────────────────┤
│  Tier 3: Predictive Cache (5min TTL)                    │
│  - Key: chain:predicted                                 │
│  - Use: Gaussian process predictions                    │
└─────────────────────────────────────────────────────────┘
```

### 2. Gas Estimation by Chain Type

#### A. EVM Chains (Ethereum, Base, Arbitrum, Optimism, Polygon)

**Post-EIP-1559 Formula:**
```
totalFee = gasLimit × (baseFee + priorityFee)
```

**Components:**
- `baseFee`: Fetched from latest block via RPC
- `priorityFee`: Estimated based on recent block tips
- `gasLimit`: Estimated per transaction type

**Transaction Type Gas Limits:**
```rust
pub enum EvmTxType {
    NativeTransfer,      // 21,000 gas
    Erc20Transfer,       // 65,000 gas (avg)
    Erc20Approve,        // 45,000 gas
    SwapExactTokens,     // 150,000 - 300,000 gas
    ComplexContract,     // Use eth_estimateGas
}
```

**Optimization: Exponential Moving Average (EMA)**
```rust
// Smooth base fee predictions
baseFeeEMA = α × currentBaseFee + (1 - α) × previousEMA
// α = 0.125 (1/8) as per EIP-1559
```

#### B. Bitcoin & UTXO Chains

**Formula:**
```
fee = feeRate (sat/vByte) × txSize (vBytes)
```

**Transaction Size Estimation:**
```rust
pub fn estimate_btc_tx_size(inputs: usize, outputs: usize, tx_type: BtcTxType) -> usize {
    match tx_type {
        BtcTxType::P2PKH => {
            // Legacy: 148 bytes per input + 34 bytes per output + 10 bytes overhead
            inputs * 148 + outputs * 34 + 10
        }
        BtcTxType::P2WPKH => {
            // SegWit: 68 vBytes per input + 31 vBytes per output + 10.5 overhead
            (inputs * 68 + outputs * 31 + 11) as usize
        }
        BtcTxType::P2TR => {
            // Taproot: 57.5 vBytes per input + 43 vBytes per output
            (inputs * 58 + outputs * 43 + 11) as usize
        }
    }
}
```

**Fee Rate Sources:**
- RPC: `estimatesmartfee` with confirmation targets (1, 3, 6 blocks)
- Mempool analysis: Parse pending transactions for fee distribution

#### C. Solana

**Formula:**
```
fee = baseFee (5000 lamports) + priorityFee (optional)
```

**Characteristics:**
- Fixed base fee: 5,000 lamports per signature
- Priority fees: Market-driven, fetch from recent blocks
- Compute units: Different operations consume different units

### 3. Kalman Filter for Demand Estimation

**State Space Model:**
```rust
pub struct GasDemandState {
    // Hidden state: [demand, price_sensitivity]
    demand: f64,
    price_sensitivity: f64,
    
    // Covariance matrix
    covariance: Matrix2<f64>,
    
    // Process noise
    process_noise: f64,
    
    // Measurement noise
    measurement_noise: f64,
}

impl GasDemandState {
    pub fn predict(&mut self, current_price: f64) -> f64 {
        // Kalman prediction step
        let predicted_demand = self.demand * ALPHA + (1.0 - ALPHA) * MEAN_DEMAND;
        
        // Update covariance
        self.covariance = ALPHA * self.covariance * ALPHA + self.process_noise;
        
        predicted_demand
    }
    
    pub fn update(&mut self, observed_usage: f64, current_price: f64) {
        // Kalman update step
        let innovation = observed_usage - (self.demand + self.price_sensitivity * current_price);
        let innovation_covariance = self.covariance + self.measurement_noise;
        
        // Kalman gain
        let kalman_gain = self.covariance / innovation_covariance;
        
        // Update state
        self.demand += kalman_gain * innovation;
        self.covariance *= (1.0 - kalman_gain);
    }
}
```

### 4. Probabilistic Early Recomputation (PER)

**Prevent Cache Stampedes:**
```rust
pub fn should_refresh_gas_price(
    cache_age: Duration,
    ttl: Duration,
    beta: f64, // 1.5 for financial data
) -> bool {
    let delta = ttl.as_secs_f64() - cache_age.as_secs_f64();
    let xfetch_threshold = delta * beta * (-cache_age.as_secs_f64() / delta).exp();
    
    // Random value between 0 and delta
    let random_value = rand::random::<f64>() * delta;
    
    random_value < xfetch_threshold
}
```

### 5. RPC Client with Fallback

**Architecture:**
```rust
pub struct MultiRpcClient {
    primary: Vec<RpcEndpoint>,
    fallback: Vec<RpcEndpoint>,
    health_checker: HealthChecker,
}

impl MultiRpcClient {
    pub async fn fetch_gas_price(&self, chain: Chain) -> Result<GasPrice> {
        // Try primary endpoints with circuit breaker
        for endpoint in &self.primary {
            if self.health_checker.is_healthy(endpoint) {
                match self.try_fetch(endpoint, chain).await {
                    Ok(price) => return Ok(price),
                    Err(e) => {
                        self.health_checker.record_failure(endpoint);
                        continue;
                    }
                }
            }
        }
        
        // Fallback to secondary endpoints
        for endpoint in &self.fallback {
            if let Ok(price) = self.try_fetch(endpoint, chain).await {
                return Ok(price);
            }
        }
        
        Err(Error::AllRpcsFailed)
    }
}
```

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1)

1. **RPC Client Manager**
   - Multi-endpoint support with health checking
   - Circuit breaker pattern
   - Automatic failover
   - Connection pooling

2. **Gas Price Cache**
   - Redis-based multi-tier caching
   - PER algorithm implementation
   - Cache key bucketing

### Phase 2: Chain-Specific Implementations (Week 2)

1. **EVM Chains**
   - `eth_gasPrice` for legacy
   - `eth_feeHistory` for EIP-1559
   - `eth_estimateGas` for complex transactions
   - Base fee + priority fee calculation

2. **Bitcoin**
   - `estimatesmartfee` RPC call
   - Transaction size estimation
   - Fee rate bucketing (slow/medium/fast)

3. **Solana**
   - `getRecentPrioritizationFees` RPC
   - Compute unit price estimation
   - Fixed base fee handling

### Phase 3: Predictive Models (Week 3)

1. **Kalman Filter**
   - Demand state tracking
   - Price sensitivity estimation
   - Prediction smoothing

2. **Gaussian Process (Optional)**
   - For high-volatility periods
   - Time-series correlation
   - Confidence intervals

### Phase 4: Integration & Testing (Week 4)

1. **Swap Module Integration**
   - Replace hardcoded gas estimates
   - Dynamic commission calculation
   - Real-time cost updates

2. **Monitoring**
   - RPC health metrics
   - Cache hit rates
   - Prediction accuracy
   - Gas price volatility alerts

## Data Structures

### Gas Price Response

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPrice {
    pub chain: Chain,
    pub timestamp: i64,
    
    // EVM-specific
    pub base_fee: Option<u64>,        // Wei
    pub priority_fee: Option<u64>,    // Wei
    pub max_fee: Option<u64>,         // Wei
    
    // Bitcoin-specific
    pub fee_rate: Option<f64>,        // sat/vByte
    
    // Solana-specific
    pub lamports_per_signature: Option<u64>,
    pub compute_unit_price: Option<u64>,
    
    // Common
    pub confidence: f64,              // 0.0 - 1.0
    pub source: GasPriceSource,
    pub cache_age: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GasPriceSource {
    RpcDirect,
    CacheExact,
    CacheBucketed,
    Predicted,
    Fallback,
}
```

### Transaction Type Gas Limits

```rust
#[derive(Debug, Clone)]
pub struct GasLimitEstimate {
    pub tx_type: TransactionType,
    pub base_limit: u64,
    pub buffer_percent: f64,  // Safety margin (e.g., 20%)
}

pub fn get_gas_limit(tx_type: TransactionType, chain: Chain) -> u64 {
    let estimate = match (chain, tx_type) {
        (Chain::Ethereum, TransactionType::NativeTransfer) => 
            GasLimitEstimate { base_limit: 21_000, buffer_percent: 0.0 },
        (Chain::Ethereum, TransactionType::Erc20Transfer) => 
            GasLimitEstimate { base_limit: 65_000, buffer_percent: 0.1 },
        (Chain::Ethereum, TransactionType::Erc20Approve) => 
            GasLimitEstimate { base_limit: 45_000, buffer_percent: 0.1 },
        // ... more mappings
    };
    
    (estimate.base_limit as f64 * (1.0 + estimate.buffer_percent)) as u64
}
```

## Performance Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Cache Hit Rate | >85% | Reduce RPC load |
| P95 Latency | <100ms | User experience |
| RPC Failure Tolerance | 100% | Fallback coverage |
| Prediction Accuracy | >90% | Within 10% of actual |
| Gas Estimate Error | <20% | Safety margin |

## Cost Optimization

### RPC Call Reduction

**Before:**
- Every swap estimate: 1 RPC call
- 1000 estimates/hour = 1000 RPC calls

**After:**
- Tier 1 cache (10s): 85% hit rate
- Tier 2 cache (60s): 10% hit rate
- RPC calls: 5% = 50 calls/hour
- **95% reduction in RPC costs**

### Bucketing Strategy

```rust
pub fn bucket_gas_price(price: u64, bucket_size: u64) -> u64 {
    (price / bucket_size) * bucket_size
}

// Example: bucket_size = 1 gwei
// 23.7 gwei -> 23 gwei
// 23.9 gwei -> 23 gwei
// 24.1 gwei -> 24 gwei
```

## Error Handling

### Fallback Hierarchy

```
1. Primary RPC (fastest, most reliable)
   ↓ (on failure)
2. Secondary RPC (backup provider)
   ↓ (on failure)
3. Cached value (stale but valid)
   ↓ (on cache miss)
4. Historical average (last 24h)
   ↓ (on no data)
5. Conservative default (high but safe)
```

### Circuit Breaker

```rust
pub struct CircuitBreaker {
    failure_threshold: usize,    // 5 failures
    timeout: Duration,           // 60 seconds
    half_open_requests: usize,   // 1 test request
}

// States: Closed -> Open -> Half-Open -> Closed
```

## Security Considerations

1. **RPC Endpoint Validation**
   - HTTPS only
   - Certificate pinning for critical endpoints
   - Rate limiting per endpoint

2. **Data Validation**
   - Sanity checks on gas prices (min/max bounds)
   - Outlier detection (>3 standard deviations)
   - Cross-validation between multiple sources

3. **Cache Poisoning Prevention**
   - TTL enforcement
   - Source verification
   - Anomaly detection

## Monitoring & Alerts

### Metrics to Track

```rust
pub struct GasPriceMetrics {
    // Performance
    pub rpc_latency_p50: Duration,
    pub rpc_latency_p95: Duration,
    pub rpc_latency_p99: Duration,
    
    // Reliability
    pub rpc_success_rate: f64,
    pub cache_hit_rate: f64,
    pub fallback_usage_rate: f64,
    
    // Accuracy
    pub prediction_error_mean: f64,
    pub prediction_error_std: f64,
    
    // Cost
    pub rpc_calls_per_hour: u64,
    pub cache_memory_usage: u64,
}
```

### Alert Conditions

- RPC success rate < 95%
- Cache hit rate < 80%
- Prediction error > 25%
- Gas price spike > 200% in 5 minutes
- All RPCs failing for > 30 seconds

## Testing Strategy

### Unit Tests
- Gas limit estimation per transaction type
- Cache bucketing logic
- PER algorithm correctness
- Kalman filter state updates

### Integration Tests
- RPC client with mock endpoints
- Cache tier fallthrough
- Multi-chain gas fetching
- Error handling and fallbacks

### Load Tests
- 1000 concurrent gas price requests
- RPC endpoint failure scenarios
- Cache stampede prevention
- Memory usage under load

## References

1. Chuang & Lee (2023). "A Practical and Economical Bayesian Approach to Gas Price Prediction" - [arXiv:2305.00337](https://ui.adsabs.harvard.edu/abs/arXiv:2305.00337)

2. Optimal Dynamic Fees for Blockchain Resources (2023) - [arXiv:2309.12735](https://ar5iv.labs.arxiv.org/html/2309.12735)

3. Ethereum EIP-1559 Specification - [eips.ethereum.org](https://eips.ethereum.org/EIPS/eip-1559)

4. Blocknative Gas Platform Documentation

5. Tatum Fee Estimation API - Multi-chain gas estimation patterns

## Integration with Existing Codebase

### Current Architecture

The codebase already has:
- ✅ RPC configuration (`src/config/rpc_config.rs`) - 20+ chains configured
- ✅ Basic RPC client (`src/services/wallet/rpc.rs`) - `get_gas_price()` exists
- ✅ Redis caching (`src/services/redis_cache.rs`) - Ready to use
- ✅ Pricing engine (`src/services/pricing/engine.rs`) - Accepts `gas_cost_native`
- ✅ Public RPC endpoints configured (no API keys needed)

### What Needs to be Built

1. **New Service Module** (`src/services/gas_price/`)
   - `mod.rs` - Public API
   - `client.rs` - Multi-RPC client with fallback
   - `cache.rs` - Multi-tier caching with PER
   - `estimator.rs` - Gas limit estimation per tx type
   - `types.rs` - Data structures

2. **Update Existing Code**
   - `src/modules/swap/crud.rs:105` - Replace hardcoded gas values
   - Add error handling with fallback to conservative defaults

### Integration Point

```rust
// Current (src/modules/swap/crud.rs:105)
async fn get_gas_cost_for_network(&self, network: &str) -> f64 {
    match network.to_lowercase().as_str() {
        "ethereum" | "erc20" => 0.002, // HARDCODED
        ...
    }
}

// After Implementation
async fn get_gas_cost_for_network(&self, network: &str) -> f64 {
    // Try to fetch real gas price
    if let Some(gas_service) = &self.gas_price_service {
        if let Ok(gas_price) = gas_service.get_gas_cost(network).await {
            return gas_price;
        }
    }
    
    // Fallback to conservative defaults
    match network.to_lowercase().as_str() {
        "ethereum" | "erc20" => 0.002,
        ...
    }
}
```

## Next Steps

1. ✅ Review design document - DONE
2. ✅ Analyze API requirements - DONE (see GAS_PRICE_API_REQUIREMENTS.md)
3. **Decision Point:** Public endpoints (free) or Premium (Alchemy)?
4. Create implementation spec in `.kiro/specs/`
5. Implement Phase 1 (Core Infrastructure)
6. Add comprehensive tests
7. Deploy to staging environment
8. Monitor metrics and iterate

**Recommended:** Start with public endpoints (no API keys needed) and implement aggressive caching for 85%+ cache hit rate.

---

**Document Status:** Draft for Review  
**Last Updated:** 2026-02-18  
**Author:** Kiro AI Assistant
