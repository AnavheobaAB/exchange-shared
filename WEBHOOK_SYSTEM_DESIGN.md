# Webhook System Design

## Executive Summary

This document outlines a production-ready webhook system for the exchange platform, enabling real-time notifications for swap status changes. The design incorporates industry best practices including exponential backoff with jitter, dead letter queues, HMAC signature verification, idempotency, and circuit breakers.

## 1. System Requirements

### Functional Requirements
- Register webhook URLs per swap
- Trigger webhooks on swap status changes
- Retry failed deliveries with exponential backoff
- Verify webhook authenticity with HMAC signatures
- Handle duplicate deliveries idempotently
- Provide webhook delivery history and logs

### Non-Functional Requirements
- **Reliability**: At-least-once delivery guarantee
- **Performance**: < 100ms webhook dispatch latency
- **Scalability**: Handle 10,000+ webhooks/minute
- **Security**: HMAC-SHA256 signature verification
- **Observability**: Comprehensive metrics and logging

## 2. Mathematical Foundations

### 2.1 Exponential Backoff with Jitter

**Problem**: Naive retry strategies cause "thundering herd" when many webhooks fail simultaneously.

**Solution**: Exponential backoff with jitter spreads retries over time.

**Formula**:
```
delay = min(base_delay × 2^attempt × (1 + jitter), max_delay)

where:
- base_delay = initial delay (e.g., 30 seconds)
- attempt = retry attempt number (0-indexed)
- jitter = random value in range [-0.1, +0.1] (±10%)
- max_delay = maximum delay cap (e.g., 24 hours)
```

**Example Retry Schedule**:
```
Attempt 0: 30s  × 2^0 × (1 ± 0.1) = 27-33s
Attempt 1: 30s  × 2^1 × (1 ± 0.1) = 54-66s
Attempt 2: 30s  × 2^2 × (1 ± 0.1) = 108-132s (1.8-2.2 min)
Attempt 3: 30s  × 2^3 × (1 ± 0.1) = 216-264s (3.6-4.4 min)
Attempt 4: 30s  × 2^4 × (1 ± 0.1) = 432-528s (7.2-8.8 min)
Attempt 5: 30s  × 2^5 × (1 ± 0.1) = 864-1056s (14.4-17.6 min)
Attempt 6: 30s  × 2^6 × (1 ± 0.1) = 1728-2112s (28.8-35.2 min)
Attempt 7: 30s  × 2^7 × (1 ± 0.1) = 3456-4224s (57.6-70.4 min)
Attempt 8: 30s  × 2^8 × (1 ± 0.1) = 6912-8448s (115.2-140.8 min)
Attempt 9: 30s  × 2^9 × (1 ± 0.1) = 13824-16896s (230.4-281.6 min)
Attempt 10: capped at max_delay = 24 hours
```

**Total Retry Window**: ~24 hours (industry standard)

**Why This Works**:
- Early retries catch transient failures (network blips, brief outages)
- Later retries allow time for extended maintenance windows
- Jitter prevents synchronized retry storms
- Cap prevents infinite growth

### 2.2 Retry Budget Calculation

**Concept**: Limit total retry attempts to balance reliability vs. resource usage.

**Formula**:
```
max_attempts = ceil(log₂(max_delay / base_delay)) + 1

For base_delay=30s, max_delay=86400s (24h):
max_attempts = ceil(log₂(86400 / 30)) + 1
             = ceil(log₂(2880)) + 1
             = ceil(11.49) + 1
             = 13 attempts
```

**Industry Standards**:
- Stripe: 3 days, ~16 attempts
- GitHub: 3 days, ~15 attempts
- Shopify: 48 hours, ~19 attempts
- **Our Choice**: 24 hours, 10-12 attempts (balanced approach)

### 2.3 HMAC Signature Verification

**Purpose**: Authenticate webhook origin and prevent tampering.

**Algorithm**: HMAC-SHA256

**Signature Generation**:
```
signature = HMAC-SHA256(secret_key, payload)
          = HMAC-SHA256(secret_key, timestamp + "." + json_body)

where:
- secret_key = per-webhook secret (32+ bytes, cryptographically random)
- timestamp = Unix timestamp in seconds
- json_body = JSON-serialized webhook payload
```

**Verification Process**:
```
1. Extract signature from X-Webhook-Signature header
2. Extract timestamp from payload
3. Check timestamp freshness: |now - timestamp| < tolerance (e.g., 5 minutes)
4. Recompute signature using secret_key
5. Compare signatures using constant-time comparison
6. Accept if signatures match AND timestamp is fresh
```

**Security Properties**:
- **Authentication**: Only holder of secret_key can generate valid signatures
- **Integrity**: Any modification to payload invalidates signature
- **Replay Protection**: Timestamp prevents replay attacks
- **Constant-Time Comparison**: Prevents timing attacks

### 2.4 Idempotency Key Design

**Problem**: Retries can cause duplicate processing (e.g., double-charging).

**Solution**: Idempotency keys ensure operations are processed exactly once.

**Key Generation**:
```
idempotency_key = SHA256(swap_id + event_type + event_timestamp)

Example:
swap_id = "550e8400-e29b-41d4-a716-446655440000"
event_type = "swap.completed"
event_timestamp = "1640000000"

idempotency_key = SHA256("550e8400-e29b-41d4-a716-446655440000.swap.completed.1640000000")
                = "a3f5b8c9d2e1f4a7b6c5d8e9f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0"
```

**Storage**:
```sql
CREATE TABLE webhook_deliveries (
    id UUID PRIMARY KEY,
    idempotency_key VARCHAR(64) UNIQUE NOT NULL,
    swap_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    delivered_at TIMESTAMP,
    response_status INT,
    INDEX idx_idempotency (idempotency_key)
);
```

**Deduplication Logic**:
```
1. Check if idempotency_key exists in webhook_deliveries
2. If exists AND delivered_at IS NOT NULL:
   - Return cached response (200 OK, already processed)
3. If exists AND delivered_at IS NULL:
   - Retry in progress, return 409 Conflict
4. If not exists:
   - Insert row with idempotency_key
   - Process webhook
   - Update delivered_at and response_status
```

### 2.5 Circuit Breaker for Webhook Endpoints

**Purpose**: Stop sending to consistently failing endpoints to conserve resources.

**State Machine**:
```
States: Closed → Open → Half-Open → Closed

Closed (Normal):
  - Allow all requests
  - Track failure rate
  - If failure_rate > threshold: transition to Open

Open (Failing):
  - Block all requests
  - Wait for timeout period
  - After timeout: transition to Half-Open

Half-Open (Testing):
  - Allow limited test requests
  - If test succeeds: transition to Closed
  - If test fails: transition back to Open
```

**Failure Rate Calculation**:
```
failure_rate = failures / total_requests

Threshold: 50% over last 10 requests
Timeout: 1 hour (exponentially increasing: 1h, 2h, 4h, 8h, max 24h)
Half-Open Test Requests: 3
```

**Example**:
```
Requests: [✓, ✓, ✗, ✗, ✗, ✗, ✗, ✗, ✗, ✗]
Failure Rate: 8/10 = 80% > 50% threshold
Action: Open circuit for 1 hour

After 1 hour:
State: Half-Open
Test Requests: [✓, ✓, ✓]
Action: Close circuit (endpoint recovered)
```

### 2.6 Rate Limiting per Endpoint

**Purpose**: Prevent overwhelming webhook consumers.

**Algorithm**: Token Bucket

**Formula**:
```
tokens_available = min(capacity, tokens + (now - last_refill) × refill_rate)

where:
- capacity = maximum tokens (e.g., 100)
- refill_rate = tokens per second (e.g., 10/s = 600/min)
- tokens = current token count
- last_refill = last refill timestamp
```

**Example Configuration**:
```
Default: 10 requests/second (600/minute)
Burst: 100 requests (capacity)

Timeline:
t=0s:   tokens=100, send 50 requests → tokens=50
t=1s:   tokens=60 (50 + 10×1), send 20 requests → tokens=40
t=2s:   tokens=50 (40 + 10×1), send 10 requests → tokens=40
t=10s:  tokens=100 (40 + 10×6, capped at capacity)
```

### 2.7 Dead Letter Queue (DLQ) Metrics

**Purpose**: Track permanently failed webhooks for manual intervention.

**Metrics**:
```
DLQ Entry Rate = failed_webhooks / total_webhooks

Target: < 0.1% (1 in 1000)
Alert: > 1% (indicates systemic issue)

DLQ Age Distribution:
- < 1 hour: Recent failures (investigate immediately)
- 1-24 hours: Retry exhausted (review endpoint health)
- > 24 hours: Stale (manual intervention or discard)
```

**Retention Policy**:
```
Keep DLQ entries for 30 days
After 30 days: Archive to cold storage or delete
Provide manual replay API for recovered endpoints
```

## 3. System Architecture

### 3.1 Components

```
┌─────────────┐
│   Swap      │
│   Service   │
└──────┬──────┘
       │ Status Change
       ▼
┌─────────────────┐
│  Webhook        │
│  Dispatcher     │◄─── Retry Queue
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Rate Limiter   │
│  (Token Bucket) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Circuit        │
│  Breaker        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  HTTP Client    │
│  (with timeout) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Customer       │
│  Endpoint       │
└─────────────────┘
         │
         │ Success/Failure
         ▼
┌─────────────────┐
│  Delivery Log   │
│  + Metrics      │
└─────────────────┘
         │
         │ If Failed
         ▼
┌─────────────────┐
│  Retry Queue    │
│  (with backoff) │
└─────────────────┘
         │
         │ If Exhausted
         ▼
┌─────────────────┐
│  Dead Letter    │
│  Queue (DLQ)    │
└─────────────────┘
```

### 3.2 Database Schema

```sql
-- Webhook registrations
CREATE TABLE webhooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    swap_id UUID NOT NULL REFERENCES swaps(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    secret_key VARCHAR(64) NOT NULL, -- HMAC secret
    events TEXT[] NOT NULL, -- ['swap.completed', 'swap.failed', etc.]
    enabled BOOLEAN DEFAULT true,
    rate_limit_per_second INT DEFAULT 10,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    
    INDEX idx_swap_id (swap_id),
    INDEX idx_enabled (enabled)
);

-- Webhook delivery attempts
CREATE TABLE webhook_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    webhook_id UUID NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    swap_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    idempotency_key VARCHAR(64) UNIQUE NOT NULL,
    
    -- Request details
    payload JSONB NOT NULL,
    signature VARCHAR(128) NOT NULL,
    
    -- Delivery tracking
    attempt_number INT NOT NULL DEFAULT 0,
    max_attempts INT NOT NULL DEFAULT 10,
    next_retry_at TIMESTAMP,
    
    -- Response tracking
    delivered_at TIMESTAMP,
    response_status INT,
    response_body TEXT,
    response_time_ms INT,
    
    -- Error tracking
    error_message TEXT,
    is_dlq BOOLEAN DEFAULT false,
    
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    
    INDEX idx_webhook_id (webhook_id),
    INDEX idx_swap_id (swap_id),
    INDEX idx_idempotency (idempotency_key),
    INDEX idx_next_retry (next_retry_at) WHERE next_retry_at IS NOT NULL,
    INDEX idx_dlq (is_dlq) WHERE is_dlq = true
);

-- Circuit breaker state
CREATE TABLE webhook_circuit_breakers (
    webhook_id UUID PRIMARY KEY REFERENCES webhooks(id) ON DELETE CASCADE,
    state VARCHAR(20) NOT NULL DEFAULT 'closed', -- closed, open, half_open
    failure_count INT NOT NULL DEFAULT 0,
    success_count INT NOT NULL DEFAULT 0,
    total_requests INT NOT NULL DEFAULT 0,
    opened_at TIMESTAMP,
    half_open_attempts INT NOT NULL DEFAULT 0,
    timeout_seconds INT NOT NULL DEFAULT 3600, -- 1 hour
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Rate limiter state
CREATE TABLE webhook_rate_limiters (
    webhook_id UUID PRIMARY KEY REFERENCES webhooks(id) ON DELETE CASCADE,
    tokens_available FLOAT NOT NULL DEFAULT 100.0,
    capacity FLOAT NOT NULL DEFAULT 100.0,
    refill_rate FLOAT NOT NULL DEFAULT 10.0, -- tokens per second
    last_refill_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
```

### 3.3 Event Types

```rust
pub enum WebhookEvent {
    // Swap lifecycle
    SwapCreated,
    SwapPending,
    SwapProcessing,
    SwapCompleted,
    SwapFailed,
    SwapExpired,
    
    // Payout events
    PayoutInitiated,
    PayoutCompleted,
    PayoutFailed,
}
```

### 3.4 Payload Structure

```json
{
  "id": "evt_1234567890abcdef",
  "type": "swap.completed",
  "created_at": 1640000000,
  "data": {
    "swap_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "completed",
    "base_currency": "BTC",
    "quote_currency": "ETH",
    "base_amount": "0.5",
    "quote_amount": "8.5",
    "exchange_rate": "17.0",
    "commission_usd": "15.50",
    "payout_tx_hash": "0x1234...abcd",
    "completed_at": 1640000000
  }
}
```

**Headers**:
```
X-Webhook-Signature: sha256=a3f5b8c9d2e1f4a7b6c5d8e9f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0
X-Webhook-Id: evt_1234567890abcdef
X-Webhook-Timestamp: 1640000000
X-Webhook-Attempt: 1
Content-Type: application/json
User-Agent: ExchangePlatform-Webhooks/1.0
```

## 4. Implementation Specifications

### 4.1 Retry Logic

```rust
pub struct RetryConfig {
    pub base_delay_secs: u64,      // 30 seconds
    pub max_delay_secs: u64,       // 86400 seconds (24 hours)
    pub max_attempts: u32,         // 10 attempts
    pub jitter_factor: f64,        // 0.1 (±10%)
    pub timeout_secs: u64,         // 30 seconds per request
}

impl RetryConfig {
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base = self.base_delay_secs as f64;
        let exponential = base * 2_f64.powi(attempt as i32);
        
        // Add jitter: ±10%
        let jitter = 1.0 + (rand::random::<f64>() * 0.2 - 0.1);
        let with_jitter = exponential * jitter;
        
        // Cap at max_delay
        let capped = with_jitter.min(self.max_delay_secs as f64);
        
        Duration::from_secs(capped as u64)
    }
}
```

### 4.2 HMAC Signature

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn generate_signature(secret: &str, timestamp: i64, payload: &str) -> String {
    let message = format!("{}.{}", timestamp, payload);
    
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    
    let result = mac.finalize();
    format!("sha256={}", hex::encode(result.into_bytes()))
}

pub fn verify_signature(
    secret: &str,
    signature: &str,
    timestamp: i64,
    payload: &str,
    tolerance_secs: i64,
) -> Result<(), WebhookError> {
    // Check timestamp freshness
    let now = chrono::Utc::now().timestamp();
    if (now - timestamp).abs() > tolerance_secs {
        return Err(WebhookError::TimestampTooOld);
    }
    
    // Verify signature
    let expected = generate_signature(secret, timestamp, payload);
    
    // Constant-time comparison
    if !constant_time_eq(signature.as_bytes(), expected.as_bytes()) {
        return Err(WebhookError::InvalidSignature);
    }
    
    Ok(())
}
```

### 4.3 Idempotency Check

```rust
pub async fn ensure_idempotent(
    pool: &PgPool,
    idempotency_key: &str,
) -> Result<IdempotencyStatus, WebhookError> {
    let result = sqlx::query!(
        r#"
        INSERT INTO webhook_deliveries (idempotency_key, ...)
        VALUES ($1, ...)
        ON CONFLICT (idempotency_key) DO NOTHING
        RETURNING id
        "#,
        idempotency_key
    )
    .fetch_optional(pool)
    .await?;
    
    match result {
        Some(_) => Ok(IdempotencyStatus::New),
        None => {
            // Check if already delivered
            let existing = sqlx::query!(
                "SELECT delivered_at, response_status FROM webhook_deliveries WHERE idempotency_key = $1",
                idempotency_key
            )
            .fetch_one(pool)
            .await?;
            
            if existing.delivered_at.is_some() {
                Ok(IdempotencyStatus::AlreadyDelivered(existing.response_status.unwrap()))
            } else {
                Ok(IdempotencyStatus::InProgress)
            }
        }
    }
}
```

### 4.4 Circuit Breaker

```rust
pub struct CircuitBreaker {
    state: CircuitState,
    failure_threshold: f64,  // 0.5 (50%)
    min_requests: u32,       // 10
    timeout_secs: u64,       // 3600 (1 hour)
    half_open_max: u32,      // 3
}

impl CircuitBreaker {
    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if self.timeout_expired() {
                    self.transition_to_half_open();
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                self.half_open_attempts < self.half_open_max
            }
        }
    }
    
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                self.half_open_attempts += 1;
                if self.half_open_attempts >= self.half_open_max {
                    self.transition_to_closed();
                }
            }
            _ => {}
        }
    }
    
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.total_requests += 1;
        
        match self.state {
            CircuitState::Closed => {
                if self.should_open() {
                    self.transition_to_open();
                }
            }
            CircuitState::HalfOpen => {
                self.transition_to_open();
            }
            _ => {}
        }
    }
    
    fn should_open(&self) -> bool {
        if self.total_requests < self.min_requests {
            return false;
        }
        
        let failure_rate = self.failure_count as f64 / self.total_requests as f64;
        failure_rate >= self.failure_threshold
    }
}
```

## 5. Monitoring & Metrics

### 5.1 Key Metrics

```rust
// Webhook delivery metrics
webhook_deliveries_total{status="success|failure|timeout"}
webhook_delivery_duration_seconds
webhook_retry_attempts
webhook_dlq_entries_total

// Circuit breaker metrics
webhook_circuit_breaker_state{state="closed|open|half_open"}
webhook_circuit_breaker_transitions_total

// Rate limiter metrics
webhook_rate_limit_tokens_available
webhook_rate_limited_requests_total
```

### 5.2 Alerting Rules

```yaml
- alert: HighWebhookFailureRate
  expr: |
    sum(rate(webhook_deliveries_total{status="failure"}[5m])) 
    / 
    sum(rate(webhook_deliveries_total[5m])) > 0.1
  for: 10m
  annotations:
    summary: "Webhook failure rate above 10%"

- alert: WebhookDLQGrowing
  expr: increase(webhook_dlq_entries_total[1h]) > 100
  annotations:
    summary: "Dead letter queue growing rapidly"
```

## 6. Security Considerations

### 6.1 Secret Key Management
- Generate cryptographically random 32-byte secrets
- Store secrets encrypted at rest
- Rotate secrets periodically (every 90 days)
- Provide secret rotation API without downtime

### 6.2 Request Validation
- Verify HMAC signature on every request
- Check timestamp freshness (±5 minutes)
- Validate URL scheme (https:// only in production)
- Sanitize and validate all input data

### 6.3 Rate Limiting
- Per-endpoint rate limits (default: 10/s)
- Global rate limit (1000/s across all webhooks)
- Burst allowance (100 requests)

## 7. Testing Strategy

### 7.1 Unit Tests
- Retry delay calculation with jitter
- HMAC signature generation and verification
- Idempotency key deduplication
- Circuit breaker state transitions
- Rate limiter token bucket algorithm

### 7.2 Integration Tests
- End-to-end webhook delivery
- Retry logic with mock failures
- Circuit breaker opening and recovery
- DLQ entry creation
- Signature verification

### 7.3 Load Tests
- 10,000 webhooks/minute sustained
- Retry storm handling
- Circuit breaker under load
- Database connection pool sizing

## 8. Operational Procedures

### 8.1 Manual Replay
```sql
-- Replay failed webhooks from DLQ
UPDATE webhook_deliveries
SET is_dlq = false,
    attempt_number = 0,
    next_retry_at = NOW()
WHERE webhook_id = $1 AND is_dlq = true;
```

### 8.2 Circuit Breaker Reset
```sql
-- Manually reset circuit breaker
UPDATE webhook_circuit_breakers
SET state = 'closed',
    failure_count = 0,
    success_count = 0,
    total_requests = 0
WHERE webhook_id = $1;
```

### 8.3 Webhook Disabling
```sql
-- Disable problematic webhook
UPDATE webhooks
SET enabled = false
WHERE id = $1;
```

## 9. Performance Characteristics

- **Dispatch Latency**: < 100ms (P95)
- **Retry Overhead**: < 1% CPU per 1000 webhooks/min
- **Storage**: ~1KB per delivery attempt
- **Database Load**: ~10 queries per webhook delivery
- **Memory**: ~10MB per 1000 active webhooks

## 10. Future Enhancements

1. **Batching**: Group multiple events into single webhook
2. **Filtering**: Allow customers to filter events by criteria
3. **Transformation**: Custom payload transformations
4. **Webhook Forwarding**: Proxy webhooks through our infrastructure
5. **GraphQL Subscriptions**: Real-time alternative to webhooks
6. **Webhook Replay UI**: Dashboard for manual intervention

## References

- [Stripe Webhooks Best Practices](https://stripe.com/docs/webhooks/best-practices)
- [GitHub Webhooks Documentation](https://docs.github.com/en/webhooks)
- [Svix Webhook University](https://www.svix.com/resources/webhook-university/)
- [RFC 2104 - HMAC](https://tools.ietf.org/html/rfc2104)
- [Exponential Backoff And Jitter](https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/)
