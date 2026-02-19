# Gas Price Fetching - API Requirements & Integration Analysis

## Current State Analysis

### ✅ What's Already in Place

1. **RPC Configuration Infrastructure** (`src/config/rpc_config.rs`)
   - Multi-chain RPC endpoint configuration
   - Primary + fallback endpoint support
   - Protocol type definitions (EVM, Solana, Bitcoin, etc.)
   - Environment variable loading
   - **20+ chains pre-configured**

2. **RPC Client** (`src/services/wallet/rpc.rs`)
   - Basic HTTP RPC client with `eth_gasPrice` support
   - Already implements `get_gas_price()` method
   - 10-second timeout configured
   - Error handling in place

3. **Environment Variables** (`.env.example`)
   - All RPC URLs pre-configured with fallbacks
   - Public endpoints as defaults (no API key required)
   - Optional premium endpoints (Alchemy, Infura, QuickNode)

4. **Current Gas Estimation** (`src/modules/swap/crud.rs:105`)
   ```rust
   async fn get_gas_cost_for_network(&self, network: &str) -> f64 {
       // TODO: use RpcClient to fetch real gas prices
       match network.to_lowercase().as_str() {
           "ethereum" | "erc20" => 0.002, // HARDCODED
           "bitcoin" => 0.0001,           // HARDCODED
           "solana" | "sol" => 0.00001,   // HARDCODED
           ...
       }
   }
   ```

5. **Pricing Engine Integration** (`src/services/pricing/engine.rs`)
   - Already accepts `gas_cost_native` parameter
   - Uses gas cost for commission calculation
   - Gas floor protection implemented

### 🔑 API Keys Required

#### Option 1: Use Public Endpoints (NO API KEYS NEEDED) ✅ RECOMMENDED FOR MVP

**Pros:**
- Already configured in `.env.example`
- Free forever
- No registration required
- Sufficient for moderate traffic

**Cons:**
- Rate limited (varies by endpoint)
- May be slower than premium
- Less reliable during high traffic

**Public Endpoints Already Configured:**
```bash
# Ethereum
ETH_PRIMARY_RPC=https://eth.llamarpc.com
ETH_FALLBACK_1_RPC=https://eth-pokt.nodies.app
ETH_FALLBACK_2_RPC=https://rpc.ankr.com/eth

# Bitcoin
BITCOIN_BLOCK_EXPLORER=https://blockchair.com/api/v1
BITCOIN_FALLBACK=https://mempool.space/api

# Solana
SOLANA_PRIMARY_RPC=https://api.mainnet-beta.solana.com
SOLANA_FALLBACK_1_RPC=https://api.rpc.solana.com

# All other chains have public endpoints configured
```

#### Option 2: Use Premium Endpoints (OPTIONAL - FOR PRODUCTION SCALE)

**When to upgrade:**
- Traffic > 100,000 requests/day
- Need guaranteed uptime SLA
- Need faster response times (<50ms)
- Need advanced features (webhooks, historical data)

**Premium Providers:**

1. **Alchemy** (Recommended)
   - Website: https://www.alchemy.com
   - Free Tier: 2M requests/day
   - Supports: Ethereum, Polygon, Arbitrum, Optimism, Base, Fantom, Mantle, Blast
   - Cost: Free tier → $49/month (Growth) → Custom (Scale)
   - **ONE API KEY for all supported chains**

2. **Infura**
   - Website: https://infura.io
   - Free Tier: 3 requests/sec (259,200 req/day)
   - Supports: Ethereum, Polygon, Arbitrum, Optimism, Linea
   - Cost: Free tier → $50/month (Developer) → Custom (Team)
   - **ONE API KEY for all supported chains**

3. **QuickNode**
   - Website: https://www.quicknode.com
   - Free Tier: 750K requests/day
   - Supports: All major chains
   - Cost: Free tier → $49/month (Build) → Custom (Scale)
   - **Separate endpoint per chain**

4. **BlockFrost** (Cardano only)
   - Website: https://blockfrost.io
   - Free Tier: 50K requests/day
   - Supports: Cardano only
   - Cost: Free tier → $10/month (Starter) → Custom

**How to Configure Premium Endpoints:**

```bash
# Option A: Alchemy (Recommended - Single Key)
ALCHEMY_API_KEY=your_alchemy_key_here

# Then update in .env:
ETH_PRIMARY_RPC=https://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}
POLYGON_PRIMARY_RPC=https://polygon-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}
ARBITRUM_PRIMARY_RPC=https://arb-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}
# ... etc

# Option B: Infura (Single Key)
INFURA_API_KEY=your_infura_key_here

# Then update in .env:
ETH_PRIMARY_RPC=https://mainnet.infura.io/v3/${INFURA_API_KEY}
POLYGON_PRIMARY_RPC=https://polygon-mainnet.infura.io/v3/${INFURA_API_KEY}
# ... etc
```

### 📊 API Key Decision Matrix

| Scenario | Recommendation | API Keys Needed |
|----------|---------------|-----------------|
| Development/Testing | Public endpoints | **NONE** ✅ |
| MVP/Launch (<10K users) | Public endpoints | **NONE** ✅ |
| Growing (10K-100K users) | Alchemy Free Tier | **1 key** (Alchemy) |
| Production (>100K users) | Alchemy Paid + Fallbacks | **1-2 keys** (Alchemy + Infura) |
| Enterprise | Multi-provider setup | **3+ keys** (Alchemy + Infura + QuickNode) |

## Integration Flow Analysis

### Current Flow (Hardcoded Gas)

```
User Request
    ↓
SwapCrud::get_gas_cost_for_network()
    ↓
Return hardcoded value (0.002 ETH)
    ↓
PricingEngine::apply_optimal_markup(gas_cost_native)
    ↓
Calculate commission with gas floor
    ↓
Return rates to user
```

### Proposed Flow (Real-Time Gas)

```
User Request
    ↓
SwapCrud::get_gas_cost_for_network()
    ↓
Check Redis Cache (Tier 1: 10s TTL)
    ↓ (cache miss)
Check Redis Cache (Tier 2: 60s bucketed)
    ↓ (cache miss)
GasPriceService::fetch_gas_price(chain)
    ↓
RpcClient::get_gas_price() [Primary RPC]
    ↓ (on failure)
RpcClient::get_gas_price() [Fallback RPC]
    ↓
Calculate: gas_cost = gasPrice × gasLimit
    ↓
Cache result in Redis (PER algorithm)
    ↓
Return to SwapCrud
    ↓
PricingEngine::apply_optimal_markup(gas_cost_native)
    ↓
Calculate commission with gas floor
    ↓
Return rates to user
```

### Integration Points

1. **SwapCrud** (`src/modules/swap/crud.rs:105`)
   - Replace hardcoded values
   - Call new `GasPriceService`
   - Handle errors gracefully (fallback to conservative defaults)

2. **PricingEngine** (`src/services/pricing/engine.rs`)
   - No changes needed ✅
   - Already accepts `gas_cost_native` parameter
   - Already uses it for commission calculation

3. **New Service** (`src/services/gas_price/mod.rs`)
   - Create new module for gas price fetching
   - Implement multi-tier caching
   - Implement RPC client with fallbacks
   - Implement chain-specific logic

## Implementation Checklist

### Phase 1: Core Infrastructure (No API Keys Required)

- [ ] Create `src/services/gas_price/mod.rs`
- [ ] Create `src/services/gas_price/client.rs` (RPC client wrapper)
- [ ] Create `src/services/gas_price/cache.rs` (Multi-tier caching)
- [ ] Create `src/services/gas_price/estimator.rs` (Gas limit estimation)
- [ ] Implement EVM gas fetching (`eth_gasPrice`, `eth_feeHistory`)
- [ ] Implement Bitcoin fee estimation (`estimatesmartfee`)
- [ ] Implement Solana fee fetching (`getRecentPrioritizationFees`)
- [ ] Add PER (Probabilistic Early Recomputation) algorithm
- [ ] Add circuit breaker for RPC health

### Phase 2: Integration

- [ ] Update `SwapCrud::get_gas_cost_for_network()` to use new service
- [ ] Add fallback to hardcoded values on service failure
- [ ] Update tests to mock gas price service
- [ ] Add integration tests with real RPC endpoints

### Phase 3: Monitoring & Optimization

- [ ] Add metrics for cache hit rate
- [ ] Add metrics for RPC latency
- [ ] Add metrics for RPC failure rate
- [ ] Add alerts for gas price spikes
- [ ] Add Kalman filter for prediction (optional)

## Configuration Examples

### Minimal Setup (Public Endpoints Only)

```bash
# .env
DATABASE_URL=mysql://user:pass@localhost:3306/exchange_db
REDIS_URL=redis://localhost:6379
WALLET_MNEMONIC=your-mnemonic-here

# No RPC configuration needed - uses public endpoints from rpc_config.rs
```

### Production Setup (With Alchemy)

```bash
# .env
DATABASE_URL=mysql://user:pass@localhost:3306/exchange_db
REDIS_URL=redis://localhost:6379
WALLET_MNEMONIC=your-mnemonic-here

# Alchemy API Key (covers 10+ chains)
ALCHEMY_API_KEY=your_alchemy_key_here

# Override primary endpoints to use Alchemy
ETH_PRIMARY_RPC=https://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}
POLYGON_PRIMARY_RPC=https://polygon-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}
ARBITRUM_PRIMARY_RPC=https://arb-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}
OPTIMISM_PRIMARY_RPC=https://opt-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}
BASE_PRIMARY_RPC=https://base-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}

# Fallbacks still use public endpoints (already configured)
```

### Enterprise Setup (Multi-Provider)

```bash
# .env
DATABASE_URL=mysql://user:pass@localhost:3306/exchange_db
REDIS_URL=redis://localhost:6379
WALLET_MNEMONIC=your-mnemonic-here

# Primary: Alchemy
ALCHEMY_API_KEY=your_alchemy_key_here
ETH_PRIMARY_RPC=https://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}

# Fallback 1: Infura
INFURA_API_KEY=your_infura_key_here
ETH_FALLBACK_1_RPC=https://mainnet.infura.io/v3/${INFURA_API_KEY}

# Fallback 2: Public (already configured)
ETH_FALLBACK_2_RPC=https://eth.llamarpc.com

# RPC Configuration
RPC_TIMEOUT_MS=10000
RPC_RETRY_ATTEMPTS=3
RPC_MAX_CONCURRENT=50
RPC_CACHE_ENABLED=true
RPC_CACHE_TTL_SECONDS=60
```

## Cost Estimation

### Public Endpoints (Free)

- **Cost:** $0/month
- **Requests:** Varies by endpoint (typically 100-1000 req/min)
- **Suitable for:** Development, MVP, <10K users
- **Risk:** Rate limiting during traffic spikes

### Alchemy Free Tier

- **Cost:** $0/month
- **Requests:** 2M requests/day (≈1,388 req/min)
- **Suitable for:** Growing apps, 10K-100K users
- **Risk:** Need to upgrade if exceed limit

### Alchemy Growth Plan

- **Cost:** $49/month
- **Requests:** 10M requests/day (≈6,944 req/min)
- **Suitable for:** Production apps, 100K-500K users
- **Includes:** 99.9% uptime SLA, priority support

### Cost Optimization with Caching

**Without caching:**
- 1000 swap estimates/hour = 1000 RPC calls
- 24,000 RPC calls/day
- 720,000 RPC calls/month

**With 85% cache hit rate:**
- 1000 swap estimates/hour = 150 RPC calls (85% cached)
- 3,600 RPC calls/day
- 108,000 RPC calls/month
- **85% cost reduction**

**With 95% cache hit rate:**
- 1000 swap estimates/hour = 50 RPC calls (95% cached)
- 1,200 RPC calls/day
- 36,000 RPC calls/month
- **95% cost reduction**

## Recommended Approach

### For MVP/Initial Launch

1. **Use public endpoints** (no API keys)
2. **Implement aggressive caching** (85%+ hit rate)
3. **Set conservative fallback values** (if RPC fails)
4. **Monitor RPC usage** (prepare for upgrade)

### For Production

1. **Get Alchemy free tier** (1 API key)
2. **Keep public endpoints as fallbacks**
3. **Implement circuit breaker** (auto-failover)
4. **Monitor and alert** (gas spikes, RPC failures)

### For Scale

1. **Upgrade to Alchemy paid tier**
2. **Add Infura as secondary** (redundancy)
3. **Implement Kalman filter** (predictive caching)
4. **Add custom RPC nodes** (ultimate control)

## Security Considerations

1. **API Key Storage**
   - Store in `.env` file (never commit)
   - Use environment variables in production
   - Rotate keys periodically

2. **Rate Limiting**
   - Implement client-side rate limiting
   - Respect provider limits
   - Use exponential backoff on errors

3. **Data Validation**
   - Sanity check gas prices (min/max bounds)
   - Detect outliers (>3 std deviations)
   - Cross-validate between providers

4. **Fallback Strategy**
   - Always have fallback values
   - Never fail user requests due to RPC issues
   - Log failures for monitoring

## Next Steps

1. **Review this document** - Confirm approach
2. **Decide on API keys** - Public vs. Premium
3. **Create implementation spec** - Detailed technical spec
4. **Implement Phase 1** - Core infrastructure
5. **Test with public endpoints** - Validate functionality
6. **Deploy to staging** - Monitor performance
7. **Upgrade to premium if needed** - Based on metrics

---

**Recommendation:** Start with public endpoints (no API keys) and implement aggressive caching. This will get you to production quickly with zero cost. Monitor usage and upgrade to Alchemy free tier when you hit rate limits.

**Timeline:**
- Phase 1 (Core): 1 week
- Phase 2 (Integration): 3 days
- Phase 3 (Monitoring): 2 days
- **Total: ~2 weeks**

**Cost:**
- Development: $0 (public endpoints)
- Production (MVP): $0 (public endpoints + caching)
- Production (Scale): $0-49/month (Alchemy free/paid tier)
