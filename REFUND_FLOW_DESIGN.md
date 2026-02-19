# Automated Refund Flow System Design

## Executive Summary

This document outlines a production-ready automated refund system for handling failed cryptocurrency swaps. The design incorporates timeout detection, state machine transitions, exponential backoff for retry logic, and comprehensive refund tracking with mathematical foundations based on industry best practices from AWS, atomic swap protocols (HTLC), and cryptocurrency exchange platforms.

## 1. System Requirements

### Functional Requirements
- **Automatic timeout detection** for stuck/failed swaps
- **State machine** for swap lifecycle management
- **Automated refund initiation** to user's refund address
- **Refund transaction tracking** with confirmation monitoring
- **Multi-chain support** (EVM, Bitcoin, Solana)
- **Idempotency** to prevent duplicate refunds
- **Manual override** capability for admin intervention

### Non-Functional Requirements
- **Reliability**: 99.9% refund success rate
- **Timeliness**: Refunds initiated within 5 minutes of timeout detection
- **Safety**: Zero duplicate refunds (idempotency guarantee)
- **Observability**: Complete audit trail for all refund operations
- **Scalability**: Handle 1000+ concurrent refund operations

## 2. Mathematical Foundations

### 2.1 Timeout Detection Algorithm

**Problem**: Determine when a swap has definitively failed and requires refund.

**Solution**: Multi-stage timeout detection with progressive escalation.

**Timeout Stages**:

```
Stage 1: Deposit Timeout (T_deposit)
- User hasn't sent funds to deposit address
- Timeout: 30 minutes (configurable per currency)
- Action: Mark swap as expired, no refund needed

Stage 2: Processing Timeout (T_processing)
- Deposit received but provider hasn't completed swap
- Timeout: 2 hours (based on provider SLA)
- Action: Query provider status, escalate if stuck

Stage 3: Payout Timeout (T_payout)
- Swap completed by provider but payout failed
- Timeout: 1 hour (blockchain confirmation time)
- Action: Retry payout, then initiate refund

Stage 4: Refund Timeout (T_refund)
- Refund transaction submitted but not confirmed
- Timeout: 30 minutes (blockchain dependent)
- Action: Resubmit with higher gas, escalate to manual
```

**Timeout Calculation Formula**:

```
T_total = T_deposit + T_processing + T_payout + T_refund
        = 30m + 2h + 1h + 30m
        = 4 hours (maximum time before refund completion)

For each stage:
is_timeout = (current_time - stage_start_time) > T_stage

Progressive check interval:
check_interval = min(60s × 2^attempt, 300s)
                = 60s, 120s, 240s, 300s (capped at 5 minutes)
```

**Why This Works**:
- Early detection prevents user frustration
- Progressive intervals reduce database load
- Stage-based approach allows targeted recovery actions
- Configurable timeouts adapt to different blockchain speeds

### 2.2 Swap State Machine

**States**:
```
CREATED → PENDING → PROCESSING → COMPLETED
                ↓         ↓           ↓
              EXPIRED  FAILED    REFUNDING → REFUNDED
                                    ↓
                                REFUND_FAILED
```

**State Transitions**:

| From State | To State | Trigger | Timeout |
|------------|----------|---------|---------|
| CREATED | PENDING | Deposit detected | 30m |
| CREATED | EXPIRED | No deposit | 30m |
| PENDING | PROCESSING | Provider confirms | 2h |
| PENDING | FAILED | Provider rejects | 2h |
| PROCESSING | COMPLETED | Payout successful | 1h |
| PROCESSING | FAILED | Payout failed | 1h |
| FAILED | REFUNDING | Refund initiated | - |
| REFUNDING | REFUNDED | Refund confirmed | 30m |
| REFUNDING | REFUND_FAILED | Refund tx failed | 30m |

**State Invariants**:
```
1. No state can transition backwards (monotonic progression)
2. Terminal states: {COMPLETED, REFUNDED, EXPIRED, REFUND_FAILED}
3. Refundable states: {FAILED, REFUNDING, REFUND_FAILED}
4. Each state has exactly one timeout value
```

### 2.3 Refund Amount Calculation

**Problem**: Calculate exact refund amount accounting for fees already paid.

**Formula**:
```
refund_amount = deposit_amount - fees_paid - gas_cost_estimate

where:
- deposit_amount = actual amount received from user
- fees_paid = platform_fee + provider_fee (if provider charged)
- gas_cost_estimate = estimated gas for refund transaction

Minimum refund threshold:
refund_amount >= min_refund_threshold
                = 0.0001 BTC | 0.001 ETH | 1 USDT (configurable)

If refund_amount < min_refund_threshold:
    action = SKIP_REFUND (not economical)
    log = "Refund amount too small, keeping as dust"
```

**Fee Recovery Logic**:
```
If provider never processed swap:
    refund_amount = deposit_amount - platform_fee - gas_cost
    
If provider processed but payout failed:
    refund_amount = deposit_amount - platform_fee - provider_fee - gas_cost
    
If partial completion:
    refund_amount = deposit_amount - amount_sent - fees - gas_cost
```

### 2.4 Exponential Backoff for Refund Retries

**Problem**: Refund transactions can fail due to network congestion, insufficient gas, or temporary RPC issues.

**Solution**: Exponential backoff with jitter and gas price escalation.

**Formula**:
```
retry_delay = min(base_delay × 2^attempt × (1 + jitter), max_delay)
gas_multiplier = 1.0 + (0.1 × attempt)  // 10% increase per retry

where:
- base_delay = 60 seconds
- max_delay = 1800 seconds (30 minutes)
- jitter = random value in [-0.1, +0.1]
- max_attempts = 5

Example retry schedule:
Attempt 0: 60s  × 2^0 × (1±0.1) = 54-66s,   gas × 1.0
Attempt 1: 60s  × 2^1 × (1±0.1) = 108-132s, gas × 1.1
Attempt 2: 60s  × 2^2 × (1±0.1) = 216-264s, gas × 1.2
Attempt 3: 60s  × 2^3 × (1±0.1) = 432-528s, gas × 1.3
Attempt 4: 60s  × 2^4 × (1±0.1) = 864-1056s, gas × 1.4
Attempt 5: Manual escalation required
```

**Gas Price Escalation**:
```
For EVM chains:
gas_price_retry = gas_price_initial × gas_multiplier
max_gas_price = gas_price_initial × 2.0  // Cap at 2x

For Bitcoin:
fee_rate_retry = fee_rate_initial × gas_multiplier
max_fee_rate = fee_rate_initial × 2.0

For Solana:
priority_fee_retry = priority_fee_initial × gas_multiplier
```

### 2.5 Idempotency Guarantee

**Problem**: Prevent duplicate refunds if system crashes or retries.

**Solution**: Idempotency key based on swap ID and refund attempt.

**Formula**:
```
idempotency_key = SHA256(swap_id || refund_address || deposit_amount || attempt_number)

Before initiating refund:
1. Check if idempotency_key exists in refunds table
2. If exists and status = COMPLETED: return existing refund
3. If exists and status = PENDING: wait for completion
4. If not exists: create new refund record with idempotency_key
```

**Database Constraint**:
```sql
UNIQUE INDEX idx_refund_idempotency ON refunds(idempotency_key);
```

### 2.6 Refund Priority Scoring

**Problem**: When multiple refunds are pending, which should be processed first?

**Solution**: Priority score based on multiple factors.

**Formula**:
```
priority_score = w1 × age_factor + w2 × amount_factor + w3 × retry_factor

where:
age_factor = min((current_time - failed_time) / 3600, 10)  // Hours since failure, capped at 10
amount_factor = min(refund_amount_usd / 100, 10)           // Amount in $100 units, capped at 10
retry_factor = 10 - attempt_number                          // Fewer retries = higher priority

Weights (normalized to sum = 1.0):
w1 = 0.5  // Age is most important (user waiting time)
w2 = 0.3  // Amount is moderately important
w3 = 0.2  // Retry count is least important

Example:
Refund A: 2 hours old, $500, 1 retry
  = 0.5 × 2 + 0.3 × 5 + 0.2 × 9 = 1.0 + 1.5 + 1.8 = 4.3

Refund B: 6 hours old, $50, 3 retries
  = 0.5 × 6 + 0.3 × 0.5 + 0.2 × 7 = 3.0 + 0.15 + 1.4 = 4.55

Process B first (higher score)
```

## 3. System Architecture

### 3.1 Module Structure

Following the existing codebase pattern, the refund system will be organized as:

```
src/services/refund/
├── mod.rs                  # Public API and module exports
├── types.rs                # Core types, enums, and errors
├── config.rs               # Configuration structures
├── detector.rs             # Timeout detection logic
├── state_machine.rs        # Swap state transitions
├── calculator.rs           # Refund amount calculations
├── processor.rs            # Refund transaction processing
├── scheduler.rs            # Priority queue and retry scheduling
├── tracker.rs              # Transaction confirmation tracking
└── monitor.rs              # Background monitoring service

tests/refund/
├── mod.rs
├── refund_detector_test.rs
├── refund_processor_test.rs
└── refund_integration_test.rs
```

### 3.2 Type Definitions (types.rs)

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Swap states for refund eligibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapStatus {
    Created,
    Pending,
    Processing,
    Completed,
    Failed,
    Expired,
    Refunding,
    Refunded,
    RefundFailed,
}

impl SwapStatus {
    /// Check if swap is in a refundable state
    pub fn is_refundable(&self) -> bool {
        matches!(self, Self::Failed | Self::Refunding | Self::RefundFailed)
    }
    
    /// Check if swap is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Refunded | Self::Expired)
    }
}

/// Timeout stage for progressive detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutStage {
    Deposit,      // Waiting for user deposit
    Processing,   // Provider processing swap
    Payout,       // Executing payout transaction
    Refund,       // Refund transaction pending
}

impl TimeoutStage {
    pub fn timeout_seconds(&self) -> u64 {
        match self {
            Self::Deposit => 1800,      // 30 minutes
            Self::Processing => 7200,   // 2 hours
            Self::Payout => 3600,       // 1 hour
            Self::Refund => 1800,       // 30 minutes
        }
    }
}

/// Refund transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Pending,
    Processing,
    Submitted,
    Confirmed,
    Failed,
    Manual,
}

/// Refund record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refund {
    pub id: Uuid,
    pub swap_id: Uuid,
    pub idempotency_key: String,
    
    // Refund details
    pub refund_address: String,
    pub refund_amount: rust_decimal::Decimal,
    pub refund_currency: String,
    pub refund_network: String,
    
    // Transaction details
    pub tx_hash: Option<String>,
    pub tx_status: RefundStatus,
    pub confirmations: u32,
    pub required_confirmations: u32,
    
    // Retry tracking
    pub attempt_number: u32,
    pub max_attempts: u32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    
    // Gas/fee tracking
    pub gas_price: Option<rust_decimal::Decimal>,
    pub gas_used: Option<u64>,
    pub total_fee: Option<rust_decimal::Decimal>,
    
    // Priority and status
    pub priority_score: f64,
    pub status: RefundStatus,
    
    // Audit trail
    pub initiated_by: String,
    pub initiated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    
    // Metadata
    pub failure_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Refund calculation result
#[derive(Debug, Clone)]
pub struct RefundCalculation {
    pub refund_amount: rust_decimal::Decimal,
    pub deposit_amount: rust_decimal::Decimal,
    pub fees_paid: rust_decimal::Decimal,
    pub gas_cost_estimate: rust_decimal::Decimal,
    pub is_economical: bool,
    pub reason: String,
}

/// Timeout detection result
#[derive(Debug, Clone)]
pub struct TimeoutDetection {
    pub swap_id: Uuid,
    pub stage: TimeoutStage,
    pub elapsed_seconds: u64,
    pub timeout_seconds: u64,
    pub is_timeout: bool,
    pub recommended_action: TimeoutAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutAction {
    Wait,
    QueryProvider,
    RetryPayout,
    InitiateRefund,
    EscalateManual,
}

/// Refund errors
#[derive(Debug, thiserror::Error)]
pub enum RefundError {
    #[error("Swap not found: {0}")]
    SwapNotFound(Uuid),
    
    #[error("Swap not refundable: {0}")]
    NotRefundable(String),
    
    #[error("Refund amount too small: {0}")]
    AmountTooSmall(String),
    
    #[error("Duplicate refund: {0}")]
    DuplicateRefund(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Wallet error: {0}")]
    Wallet(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
}
```

### 3.3 Configuration (config.rs)

```rust
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundConfig {
    // Timeout settings (seconds)
    pub deposit_timeout: u64,
    pub processing_timeout: u64,
    pub payout_timeout: u64,
    pub refund_timeout: u64,
    
    // Retry settings
    pub max_retry_attempts: u32,
    pub base_retry_delay: u64,
    pub max_retry_delay: u64,
    pub jitter_factor: f64,
    
    // Gas escalation
    pub gas_multiplier_per_retry: f64,
    pub max_gas_multiplier: f64,
    
    // Economic thresholds (in base units)
    pub min_refund_threshold_btc: f64,
    pub min_refund_threshold_eth: f64,
    pub min_refund_threshold_usd: f64,
    
    // Processing
    pub worker_pool_size: usize,
    pub batch_size: usize,
    pub check_interval: u64,
    
    // Priority weights
    pub priority_weight_age: f64,
    pub priority_weight_amount: f64,
    pub priority_weight_retry: f64,
}

impl Default for RefundConfig {
    fn default() -> Self {
        Self {
            deposit_timeout: 1800,
            processing_timeout: 7200,
            payout_timeout: 3600,
            refund_timeout: 1800,
            
            max_retry_attempts: 5,
            base_retry_delay: 60,
            max_retry_delay: 1800,
            jitter_factor: 0.1,
            
            gas_multiplier_per_retry: 0.1,
            max_gas_multiplier: 2.0,
            
            min_refund_threshold_btc: 0.0001,
            min_refund_threshold_eth: 0.001,
            min_refund_threshold_usd: 1.0,
            
            worker_pool_size: 10,
            batch_size: 100,
            check_interval: 60,
            
            priority_weight_age: 0.5,
            priority_weight_amount: 0.3,
            priority_weight_retry: 0.2,
        }
    }
}

impl RefundConfig {
    /// Load from environment variables
    pub fn from_env() -> Result<Self, String> {
        let mut config = Self::default();
        
        if let Ok(val) = std::env::var("REFUND_DEPOSIT_TIMEOUT") {
            config.deposit_timeout = val.parse().map_err(|e| format!("Invalid REFUND_DEPOSIT_TIMEOUT: {}", e))?;
        }
        
        if let Ok(val) = std::env::var("REFUND_MAX_RETRY_ATTEMPTS") {
            config.max_retry_attempts = val.parse().map_err(|e| format!("Invalid REFUND_MAX_RETRY_ATTEMPTS: {}", e))?;
        }
        
        // Validate weights sum to 1.0
        let weight_sum = config.priority_weight_age + config.priority_weight_amount + config.priority_weight_retry;
        if (weight_sum - 1.0).abs() > 0.01 {
            return Err(format!("Priority weights must sum to 1.0, got {}", weight_sum));
        }
        
        Ok(config)
    }
    
    /// Calculate retry delay with exponential backoff and jitter
    pub fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base = self.base_retry_delay as f64;
        let exponential = base * 2_f64.powi(attempt as i32);
        
        // Add jitter: random value in range [1 - jitter_factor, 1 + jitter_factor]
        let mut rng = rand::rng();
        let jitter = 1.0 + (rng.random::<f64>() * 2.0 - 1.0) * self.jitter_factor;
        let with_jitter = exponential * jitter;
        
        // Cap at max_delay
        let capped = with_jitter.min(self.max_retry_delay as f64);
        
        Duration::from_secs(capped as u64)
    }
    
    /// Calculate gas multiplier for retry attempt
    pub fn calculate_gas_multiplier(&self, attempt: u32) -> f64 {
        let multiplier = 1.0 + (self.gas_multiplier_per_retry * attempt as f64);
        multiplier.min(self.max_gas_multiplier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = RefundConfig::default();
        assert_eq!(config.deposit_timeout, 1800);
        assert_eq!(config.max_retry_attempts, 5);
    }
    
    #[test]
    fn test_retry_delay_calculation() {
        let config = RefundConfig::default();
        
        let delay0 = config.calculate_retry_delay(0);
        assert!(delay0.as_secs() >= 54 && delay0.as_secs() <= 66); // 60s ± 10%
        
        let delay1 = config.calculate_retry_delay(1);
        assert!(delay1.as_secs() >= 108 && delay1.as_secs() <= 132); // 120s ± 10%
        
        let delay10 = config.calculate_retry_delay(10);
        assert_eq!(delay10.as_secs(), 1800); // Capped at max_delay
    }
    
    #[test]
    fn test_gas_multiplier() {
        let config = RefundConfig::default();
        
        assert_eq!(config.calculate_gas_multiplier(0), 1.0);
        assert_eq!(config.calculate_gas_multiplier(1), 1.1);
        assert_eq!(config.calculate_gas_multiplier(5), 1.5);
        assert_eq!(config.calculate_gas_multiplier(20), 2.0); // Capped at max
    }
}
```

### 3.4 Timeout Detector (detector.rs)

```rust
use sqlx::MySqlPool;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

use crate::services::refund::{
    RefundConfig, RefundError, TimeoutDetection, TimeoutStage, TimeoutAction, SwapStatus,
};

pub struct TimeoutDetector {
    pool: MySqlPool,
    config: RefundConfig,
}

impl TimeoutDetector {
    pub fn new(pool: MySqlPool, config: RefundConfig) -> Self {
        Self { pool, config }
    }
    
    /// Scan for swaps that have timed out
    pub async fn scan_timeouts(&self) -> Result<Vec<TimeoutDetection>, RefundError> {
        let mut detections = Vec::new();
        
        // Check each timeout stage
        detections.extend(self.check_deposit_timeouts().await?);
        detections.extend(self.check_processing_timeouts().await?);
        detections.extend(self.check_payout_timeouts().await?);
        detections.extend(self.check_refund_timeouts().await?);
        
        Ok(detections)
    }
    
    async fn check_deposit_timeouts(&self) -> Result<Vec<TimeoutDetection>, RefundError> {
        let timeout_threshold = Utc::now() - Duration::seconds(self.config.deposit_timeout as i64);
        
        let swaps = sqlx::query!(
            r#"
            SELECT id, created_at, status
            FROM swaps
            WHERE status = 'CREATED'
              AND created_at < ?
              AND deposit_address IS NOT NULL
            LIMIT ?
            "#,
            timeout_threshold,
            self.config.batch_size as i64
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut detections = Vec::new();
        for swap in swaps {
            let elapsed = (Utc::now() - swap.created_at.and_utc()).num_seconds() as u64;
            
            detections.push(TimeoutDetection {
                swap_id: Uuid::parse_str(&swap.id).unwrap(),
                stage: TimeoutStage::Deposit,
                elapsed_seconds: elapsed,
                timeout_seconds: self.config.deposit_timeout,
                is_timeout: true,
                recommended_action: TimeoutAction::InitiateRefund, // Mark as expired, no refund needed
            });
        }
        
        Ok(detections)
    }
    
    async fn check_processing_timeouts(&self) -> Result<Vec<TimeoutDetection>, RefundError> {
        let timeout_threshold = Utc::now() - Duration::seconds(self.config.processing_timeout as i64);
        
        let swaps = sqlx::query!(
            r#"
            SELECT id, updated_at, status
            FROM swaps
            WHERE status IN ('PENDING', 'PROCESSING')
              AND updated_at < ?
            LIMIT ?
            "#,
            timeout_threshold,
            self.config.batch_size as i64
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut detections = Vec::new();
        for swap in swaps {
            let elapsed = (Utc::now() - swap.updated_at.and_utc()).num_seconds() as u64;
            
            detections.push(TimeoutDetection {
                swap_id: Uuid::parse_str(&swap.id).unwrap(),
                stage: TimeoutStage::Processing,
                elapsed_seconds: elapsed,
                timeout_seconds: self.config.processing_timeout,
                is_timeout: true,
                recommended_action: TimeoutAction::QueryProvider,
            });
        }
        
        Ok(detections)
    }
    
    async fn check_payout_timeouts(&self) -> Result<Vec<TimeoutDetection>, RefundError> {
        // Similar implementation for payout stage
        Ok(Vec::new())
    }
    
    async fn check_refund_timeouts(&self) -> Result<Vec<TimeoutDetection>, RefundError> {
        // Similar implementation for refund stage
        Ok(Vec::new())
    }
    
    /// Calculate progressive check interval based on attempt number
    pub fn calculate_check_interval(&self, attempt: u32) -> Duration {
        let base_interval = 60; // 60 seconds
        let max_interval = 300; // 5 minutes
        
        let interval_secs = std::cmp::min(base_interval * 2_u64.pow(attempt), max_interval);
        Duration::seconds(interval_secs as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_check_interval_calculation() {
        let config = RefundConfig::default();
        let detector = TimeoutDetector::new(
            sqlx::MySqlPool::connect("mysql://localhost").await.unwrap(),
            config
        );
        
        assert_eq!(detector.calculate_check_interval(0).num_seconds(), 60);
        assert_eq!(detector.calculate_check_interval(1).num_seconds(), 120);
        assert_eq!(detector.calculate_check_interval(2).num_seconds(), 240);
        assert_eq!(detector.calculate_check_interval(3).num_seconds(), 300); // Capped
    }
}
```

### 3.5 Refund Calculator (calculator.rs)

```rust
use rust_decimal::Decimal;
use uuid::Uuid;
use sqlx::MySqlPool;

use crate::services::refund::{RefundCalculation, RefundConfig, RefundError};

pub struct RefundCalculator {
    pool: MySqlPool,
    config: RefundConfig,
}

impl RefundCalculator {
    pub fn new(pool: MySqlPool, config: RefundConfig) -> Self {
        Self { pool, config }
    }
    
    /// Calculate refund amount for a failed swap
    pub async fn calculate_refund(&self, swap_id: Uuid) -> Result<RefundCalculation, RefundError> {
        // Fetch swap details
        let swap = sqlx::query!(
            r#"
            SELECT 
                amount as deposit_amount,
                from_currency,
                platform_fee,
                total_fee,
                status
            FROM swaps
            WHERE id = ?
            "#,
            swap_id.to_string()
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(RefundError::SwapNotFound(swap_id))?;
        
        let deposit_amount = swap.deposit_amount;
        let platform_fee = swap.platform_fee.unwrap_or(Decimal::ZERO);
        let total_fee = swap.total_fee.unwrap_or(Decimal::ZERO);
        
        // Estimate gas cost for refund transaction
        let gas_cost_estimate = self.estimate_gas_cost(&swap.from_currency).await?;
        
        // Calculate refund amount
        let refund_amount = deposit_amount - platform_fee - total_fee - gas_cost_estimate;
        
        // Check if economical
        let min_threshold = self.get_min_threshold(&swap.from_currency);
        let is_economical = refund_amount >= min_threshold;
        
        let reason = if !is_economical {
            format!("Refund amount {} is below minimum threshold {}", refund_amount, min_threshold)
        } else {
            "Refund is economical".to_string()
        };
        
        Ok(RefundCalculation {
            refund_amount,
            deposit_amount,
            fees_paid: platform_fee + total_fee,
            gas_cost_estimate,
            is_economical,
            reason,
        })
    }
    
    async fn estimate_gas_cost(&self, currency: &str) -> Result<Decimal, RefundError> {
        // Use gas estimation service
        match currency.to_uppercase().as_str() {
            "BTC" => Ok(Decimal::from_str_exact("0.0001").unwrap()),
            "ETH" => Ok(Decimal::from_str_exact("0.001").unwrap()),
            "SOL" => Ok(Decimal::from_str_exact("0.00001").unwrap()),
            _ => Ok(Decimal::from_str_exact("0.0001").unwrap()),
        }
    }
    
    fn get_min_threshold(&self, currency: &str) -> Decimal {
        match currency.to_uppercase().as_str() {
            "BTC" => Decimal::from_f64(self.config.min_refund_threshold_btc).unwrap(),
            "ETH" => Decimal::from_f64(self.config.min_refund_threshold_eth).unwrap(),
            _ => Decimal::from_f64(self.config.min_refund_threshold_usd).unwrap(),
        }
    }
    
    /// Calculate priority score for refund processing order
    pub fn calculate_priority_score(
        &self,
        age_hours: f64,
        amount_usd: f64,
        attempt_number: u32,
    ) -> f64 {
        let age_factor = age_hours.min(10.0);
        let amount_factor = (amount_usd / 100.0).min(10.0);
        let retry_factor = (10 - attempt_number as i32).max(0) as f64;
        
        self.config.priority_weight_age * age_factor
            + self.config.priority_weight_amount * amount_factor
            + self.config.priority_weight_retry * retry_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_priority_score_calculation() {
        let config = RefundConfig::default();
        let calculator = RefundCalculator::new(
            sqlx::MySqlPool::connect("mysql://localhost").await.unwrap(),
            config.clone()
        );
        
        // Old, large amount, few retries = high priority
        let score1 = calculator.calculate_priority_score(6.0, 500.0, 1);
        // 0.5 * 6 + 0.3 * 5 + 0.2 * 9 = 3.0 + 1.5 + 1.8 = 6.3
        assert!((score1 - 6.3).abs() < 0.01);
        
        // New, small amount, many retries = low priority
        let score2 = calculator.calculate_priority_score(1.0, 50.0, 4);
        // 0.5 * 1 + 0.3 * 0.5 + 0.2 * 6 = 0.5 + 0.15 + 1.2 = 1.85
        assert!((score2 - 1.85).abs() < 0.01);
    }
}
```

### 3.6 Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Refund Flow System                        │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────┐      ┌──────────────────┐            │
│  │ Timeout Detector │─────▶│  State Machine   │            │
│  │  (Background)    │      │   Transitions    │            │
│  └──────────────────┘      └──────────────────┘            │
│           │                         │                        │
│           ▼                         ▼                        │
│  ┌──────────────────┐      ┌──────────────────┐            │
│  │ Refund Scheduler │─────▶│ Refund Processor │            │
│  │  (Priority Queue)│      │  (Worker Pool)   │            │
│  └──────────────────┘      └──────────────────┘            │
│           │                         │                        │
│           ▼                         ▼                        │
│  ┌──────────────────┐      ┌──────────────────┐            │
│  │ Refund Tracker   │◀─────│  Wallet Service  │            │
│  │ (Confirmations)  │      │ (Multi-chain TX) │            │
│  └──────────────────┘      └──────────────────┘            │
│           │                         │                        │
│           ▼                         ▼                        │
│  ┌──────────────────┐      ┌──────────────────┐            │
│  │ Webhook Notifier │      │  Metrics/Logging │            │
│  │  (User Alerts)   │      │   (Observability)│            │
│  └──────────────────┘      └──────────────────┘            │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

```sql
-- Refunds table
CREATE TABLE refunds (
    id VARCHAR(36) PRIMARY KEY,
    swap_id VARCHAR(36) NOT NULL,
    idempotency_key VARCHAR(64) NOT NULL UNIQUE,
    
    -- Refund details
    refund_address VARCHAR(255) NOT NULL,
    refund_amount DECIMAL(20, 8) NOT NULL,
    refund_currency VARCHAR(10) NOT NULL,
    refund_network VARCHAR(50) NOT NULL,
    
    -- Transaction details
    tx_hash VARCHAR(255),
    tx_status ENUM('PENDING', 'SUBMITTED', 'CONFIRMED', 'FAILED') DEFAULT 'PENDING',
    confirmations INT DEFAULT 0,
    required_confirmations INT DEFAULT 6,
    
    -- Retry tracking
    attempt_number INT DEFAULT 0,
    max_attempts INT DEFAULT 5,
    next_retry_at DATETIME,
    last_error TEXT,
    
    -- Gas/fee tracking
    gas_price DECIMAL(20, 8),
    gas_used BIGINT,
    total_fee DECIMAL(20, 8),
    
    -- Priority and status
    priority_score DECIMAL(10, 2),
    status ENUM('PENDING', 'PROCESSING', 'COMPLETED', 'FAILED', 'MANUAL') DEFAULT 'PENDING',
    
    -- Audit trail
    initiated_by VARCHAR(50) DEFAULT 'SYSTEM',
    initiated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    
    -- Metadata
    failure_reason TEXT,
    metadata JSON,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_swap_id (swap_id),
    INDEX idx_status_priority (status, priority_score DESC),
    INDEX idx_next_retry (next_retry_at),
    INDEX idx_tx_hash (tx_hash),
    
    FOREIGN KEY (swap_id) REFERENCES swaps(id) ON DELETE CASCADE
);

-- Refund history (audit log)
CREATE TABLE refund_history (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    refund_id VARCHAR(36) NOT NULL,
    
    -- State change
    from_status VARCHAR(20),
    to_status VARCHAR(20) NOT NULL,
    
    -- Transaction details
    tx_hash VARCHAR(255),
    gas_price DECIMAL(20, 8),
    error_message TEXT,
    
    -- Context
    triggered_by VARCHAR(50),
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    INDEX idx_refund_id (refund_id),
    INDEX idx_timestamp (timestamp),
    
    FOREIGN KEY (refund_id) REFERENCES refunds(id) ON DELETE CASCADE
);
```

### 3.3 Configuration

```rust
pub struct RefundConfig {
    // Timeout settings (seconds)
    pub deposit_timeout: u64,        // 1800 (30 minutes)
    pub processing_timeout: u64,     // 7200 (2 hours)
    pub payout_timeout: u64,         // 3600 (1 hour)
    pub refund_timeout: u64,         // 1800 (30 minutes)
    
    // Retry settings
    pub max_retry_attempts: u32,     // 5
    pub base_retry_delay: u64,       // 60 seconds
    pub max_retry_delay: u64,        // 1800 seconds
    pub jitter_factor: f64,          // 0.1 (±10%)
    
    // Gas escalation
    pub gas_multiplier_per_retry: f64,  // 0.1 (10% increase)
    pub max_gas_multiplier: f64,        // 2.0 (200% of initial)
    
    // Economic thresholds
    pub min_refund_threshold_btc: f64,  // 0.0001
    pub min_refund_threshold_eth: f64,  // 0.001
    pub min_refund_threshold_usd: f64,  // 1.0
    
    // Processing
    pub worker_pool_size: usize,        // 10
    pub batch_size: usize,              // 100
    pub check_interval: u64,            // 60 seconds
    
    // Priority weights
    pub priority_weight_age: f64,       // 0.5
    pub priority_weight_amount: f64,    // 0.3
    pub priority_weight_retry: f64,     // 0.2
}
```

## 4. Implementation Strategy

### 4.1 Phase 1: Core Infrastructure (Week 1)
- Database schema and migrations
- Refund types and error handling
- Configuration management
- Basic state machine implementation

### 4.2 Phase 2: Timeout Detection (Week 1-2)
- Background job for timeout scanning
- State transition logic
- Timeout calculation per chain
- Integration with existing swap monitoring

### 4.3 Phase 3: Refund Processing (Week 2-3)
- Refund amount calculation
- Multi-chain transaction building
- Idempotency enforcement
- Priority queue implementation

### 4.4 Phase 4: Retry & Tracking (Week 3)
- Exponential backoff implementation
- Gas price escalation
- Transaction confirmation monitoring
- Retry scheduler

### 4.5 Phase 5: Observability & Admin (Week 4)
- Metrics and logging
- Webhook notifications
- Admin dashboard for manual intervention
- Comprehensive testing

## 5. Metrics & Monitoring

### 5.1 Key Metrics

```
# Refund Operations
refund_initiated_total{currency, network, reason}
refund_completed_total{currency, network}
refund_failed_total{currency, network, error_type}
refund_processing_duration_seconds{currency, network}

# Timeout Detection
timeout_detected_total{stage, currency}
timeout_check_duration_seconds
swaps_monitored_total

# Retry Behavior
refund_retry_attempts{currency, attempt_number}
refund_retry_success_rate{currency}
refund_gas_escalation{currency, multiplier}

# Economic Metrics
refund_amount_total_usd
refund_gas_cost_total_usd
refund_dust_skipped_total{currency}

# Queue Metrics
refund_queue_size
refund_queue_wait_time_seconds
refund_priority_score_distribution
```

### 5.2 Alerts

```
# Critical Alerts
- Refund failure rate > 5% (5 minutes)
- Refund queue size > 1000 (10 minutes)
- Refund processing time > 30 minutes (P95)

# Warning Alerts
- Timeout detection lag > 5 minutes
- Retry attempts > 3 (per refund)
- Gas escalation > 1.5x

# Info Alerts
- Large refund initiated (> $10,000)
- Manual intervention required
- Dust threshold reached
```

## 6. Security Considerations

### 6.1 Access Control
- Refund initiation requires system-level permissions
- Manual overrides logged with admin identity
- Refund address validation against original swap

### 6.2 Fraud Prevention
- Maximum refund amount = deposit amount
- Refund address must match swap refund_address field
- Rate limiting on refund operations per user
- Anomaly detection for unusual refund patterns

### 6.3 Audit Trail
- Complete history of all state transitions
- Transaction hashes recorded for all refunds
- Failed attempts logged with error details
- Admin actions tracked with timestamps

## 7. Testing Strategy

### 7.1 Unit Tests
- Timeout calculation accuracy
- State machine transitions
- Priority score calculation
- Idempotency key generation
- Gas escalation logic

### 7.2 Integration Tests
- End-to-end refund flow
- Multi-chain refund processing
- Retry mechanism with backoff
- Database constraint enforcement
- Webhook notification delivery

### 7.3 Load Tests
- 1000 concurrent refunds
- Priority queue performance
- Database query optimization
- Worker pool scalability

## 8. References

Content rephrased for compliance with licensing restrictions:

- [AWS Builders Library](https://aws.amazon.com/builders-library/timeouts-retries-and-backoff-with-jitter/) - Timeout and retry patterns
- [Hash Time-Locked Contracts](https://lendasat.com/docs/lendaswap/htlc) - HTLC refund mechanisms
- [Atomic Swap Protocols](https://www.garden.finance/blog/what-are-htlcs) - Timelock-based refunds
- Industry practices from Changelly, Coinify, and other exchanges

## 9. Success Criteria

- ✅ 99.9% refund success rate
- ✅ < 5 minute detection latency for timeouts
- ✅ < 30 minute average refund completion time
- ✅ Zero duplicate refunds (idempotency)
- ✅ < 1% manual intervention rate
- ✅ Complete audit trail for all operations
- ✅ Comprehensive test coverage (>90%)


### 3.7 Database Schema (migrations/008_refunds.sql)

```sql
-- Refunds table
CREATE TABLE refunds (
    id VARCHAR(36) PRIMARY KEY,
    swap_id VARCHAR(36) NOT NULL,
    idempotency_key VARCHAR(64) NOT NULL UNIQUE,
    
    -- Refund details
    refund_address VARCHAR(255) NOT NULL,
    refund_amount DECIMAL(20, 8) NOT NULL,
    refund_currency VARCHAR(10) NOT NULL,
    refund_network VARCHAR(50) NOT NULL,
    
    -- Transaction details
    tx_hash VARCHAR(255),
    tx_status ENUM('PENDING', 'SUBMITTED', 'CONFIRMED', 'FAILED') DEFAULT 'PENDING',
    confirmations INT DEFAULT 0,
    required_confirmations INT DEFAULT 6,
    
    -- Retry tracking
    attempt_number INT DEFAULT 0,
    max_attempts INT DEFAULT 5,
    next_retry_at DATETIME,
    last_error TEXT,
    
    -- Gas/fee tracking
    gas_price DECIMAL(20, 8),
    gas_used BIGINT,
    total_fee DECIMAL(20, 8),
    
    -- Priority and status
    priority_score DECIMAL(10, 2),
    status ENUM('PENDING', 'PROCESSING', 'COMPLETED', 'FAILED', 'MANUAL') DEFAULT 'PENDING',
    
    -- Audit trail
    initiated_by VARCHAR(50) DEFAULT 'SYSTEM',
    initiated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    
    -- Metadata
    failure_reason TEXT,
    metadata JSON,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_swap_id (swap_id),
    INDEX idx_status_priority (status, priority_score DESC),
    INDEX idx_next_retry (next_retry_at),
    INDEX idx_tx_hash (tx_hash),
    INDEX idx_idempotency (idempotency_key),
    
    FOREIGN KEY (swap_id) REFERENCES swaps(id) ON DELETE CASCADE
);

-- Refund history (audit log)
CREATE TABLE refund_history (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    refund_id VARCHAR(36) NOT NULL,
    
    -- State change
    from_status VARCHAR(20),
    to_status VARCHAR(20) NOT NULL,
    
    -- Transaction details
    tx_hash VARCHAR(255),
    gas_price DECIMAL(20, 8),
    error_message TEXT,
    
    -- Context
    triggered_by VARCHAR(50),
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    INDEX idx_refund_id (refund_id),
    INDEX idx_timestamp (timestamp),
    
    FOREIGN KEY (refund_id) REFERENCES refunds(id) ON DELETE CASCADE
);

-- Add refund_address to swaps table if not exists
ALTER TABLE swaps ADD COLUMN IF NOT EXISTS refund_address VARCHAR(255);
```

## 4. Implementation Strategy

### 4.1 Phase 1: Core Infrastructure (Week 1)
**Files to create**:
- `src/services/refund/types.rs` - Core types and errors
- `src/services/refund/config.rs` - Configuration management
- `src/services/refund/mod.rs` - Module exports
- `migrations/008_refunds.sql` - Database schema

**Tasks**:
- Define all types, enums, and error types
- Implement configuration with env var loading
- Create database migration
- Add comprehensive unit tests for types and config

**Success Criteria**:
- All types compile without errors
- Configuration loads from environment
- Migration applies successfully
- 100% test coverage for config module

### 4.2 Phase 2: Timeout Detection (Week 1-2)
**Files to create**:
- `src/services/refund/detector.rs` - Timeout detection logic
- `src/services/refund/state_machine.rs` - State transitions
- `tests/refund/refund_detector_test.rs` - Detector tests

**Tasks**:
- Implement timeout detection for all stages
- Create state machine with transition rules
- Add progressive check interval calculation
- Write integration tests with test database

**Success Criteria**:
- Detects timeouts accurately within 1 minute
- State transitions follow defined rules
- 90%+ test coverage
- Performance: < 100ms per batch of 100 swaps

### 4.3 Phase 3: Refund Processing (Week 2-3)
**Files to create**:
- `src/services/refund/calculator.rs` - Amount calculations
- `src/services/refund/processor.rs` - Transaction processing
- `src/services/refund/scheduler.rs` - Priority queue
- `tests/refund/refund_processor_test.rs` - Processor tests

**Tasks**:
- Implement refund amount calculation
- Build transaction submission logic
- Create priority queue with scoring
- Add idempotency enforcement
- Integrate with wallet manager

**Success Criteria**:
- Accurate refund calculations
- Idempotency prevents duplicates
- Priority queue orders correctly
- Integration with existing wallet service

### 4.4 Phase 4: Retry & Tracking (Week 3)
**Files to create**:
- `src/services/refund/tracker.rs` - Confirmation tracking
- `tests/refund/refund_integration_test.rs` - End-to-end tests

**Tasks**:
- Implement exponential backoff with jitter
- Add gas price escalation
- Create confirmation monitoring
- Build retry scheduler

**Success Criteria**:
- Retry delays follow exponential backoff
- Gas escalation works correctly
- Confirmations tracked accurately
- < 5% manual intervention rate

### 4.5 Phase 5: Monitoring & Admin (Week 4)
**Files to create**:
- `src/services/refund/monitor.rs` - Background service
- `src/modules/refund/` - Admin API endpoints
- Grafana dashboard configuration

**Tasks**:
- Create background monitoring service
- Add Prometheus metrics
- Build admin API for manual intervention
- Create Grafana dashboards
- Write comprehensive documentation

**Success Criteria**:
- Monitor runs continuously without crashes
- Metrics exported to Prometheus
- Admin can manually trigger refunds
- Complete documentation

## 5. Metrics & Monitoring

### 5.1 Prometheus Metrics

```rust
// In src/services/metrics/collectors.rs

// Refund Operations
pub refund_initiated_total: IntCounterVec,  // {currency, network, reason}
pub refund_completed_total: IntCounterVec,  // {currency, network}
pub refund_failed_total: IntCounterVec,     // {currency, network, error_type}
pub refund_processing_duration_seconds: HistogramVec,  // {currency, network}

// Timeout Detection
pub timeout_detected_total: IntCounterVec,  // {stage, currency}
pub timeout_check_duration_seconds: Histogram,
pub swaps_monitored_total: IntCounter,

// Retry Behavior
pub refund_retry_attempts: HistogramVec,    // {currency, attempt_number}
pub refund_retry_success_rate: GaugeVec,    // {currency}
pub refund_gas_escalation: HistogramVec,    // {currency, multiplier}

// Economic Metrics
pub refund_amount_total_usd: Counter,
pub refund_gas_cost_total_usd: Counter,
pub refund_dust_skipped_total: IntCounterVec,  // {currency}

// Queue Metrics
pub refund_queue_size: Gauge,
pub refund_queue_wait_time_seconds: Histogram,
pub refund_priority_score_distribution: Histogram,
```

### 5.2 Alert Rules

```yaml
# Critical Alerts
- alert: HighRefundFailureRate
  expr: rate(refund_failed_total[5m]) / rate(refund_initiated_total[5m]) > 0.05
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "Refund failure rate above 5%"

- alert: RefundQueueBacklog
  expr: refund_queue_size > 1000
  for: 10m
  labels:
    severity: critical
  annotations:
    summary: "Refund queue has {{ $value }} pending items"

- alert: SlowRefundProcessing
  expr: histogram_quantile(0.95, rate(refund_processing_duration_seconds_bucket[5m])) > 1800
  for: 15m
  labels:
    severity: warning
  annotations:
    summary: "P95 refund processing time above 30 minutes"

# Warning Alerts
- alert: TimeoutDetectionLag
  expr: timeout_check_duration_seconds > 300
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Timeout detection taking longer than 5 minutes"

- alert: HighRetryRate
  expr: avg(refund_retry_attempts) > 3
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "Average retry attempts above 3"

- alert: GasEscalation
  expr: avg(refund_gas_escalation) > 1.5
  for: 10m
  labels:
    severity: info
  annotations:
    summary: "Gas prices escalating above 1.5x"
```

### 5.3 Grafana Dashboard Queries

```promql
# Refund Success Rate
sum(rate(refund_completed_total[5m])) / sum(rate(refund_initiated_total[5m]))

# Average Refund Processing Time
rate(refund_processing_duration_seconds_sum[5m]) / rate(refund_processing_duration_seconds_count[5m])

# Refund Queue Depth Over Time
refund_queue_size

# Timeout Detection by Stage
sum by (stage) (rate(timeout_detected_total[5m]))

# Refund Amount by Currency
sum by (currency) (rate(refund_amount_total_usd[1h]))

# Gas Cost Efficiency
sum(rate(refund_gas_cost_total_usd[1h])) / sum(rate(refund_amount_total_usd[1h]))

# Priority Score Distribution
histogram_quantile(0.95, rate(refund_priority_score_distribution_bucket[5m]))
```

## 6. Testing Strategy

### 6.1 Unit Tests

```rust
// tests/refund/config_test.rs
#[test]
fn test_retry_delay_calculation() { }

#[test]
fn test_gas_multiplier_calculation() { }

#[test]
fn test_config_validation() { }

// tests/refund/calculator_test.rs
#[test]
fn test_refund_amount_calculation() { }

#[test]
fn test_priority_score_calculation() { }

#[test]
fn test_economic_threshold_check() { }

// tests/refund/detector_test.rs
#[test]
fn test_timeout_detection() { }

#[test]
fn test_progressive_check_interval() { }

#[test]
fn test_timeout_action_recommendation() { }
```

### 6.2 Integration Tests

```rust
// tests/refund/refund_integration_test.rs
#[tokio::test]
#[serial]
async fn test_end_to_end_refund_flow() {
    // 1. Create failed swap
    // 2. Detect timeout
    // 3. Calculate refund
    // 4. Process refund
    // 5. Track confirmation
    // 6. Verify completion
}

#[tokio::test]
#[serial]
async fn test_idempotency_enforcement() {
    // Attempt duplicate refund, verify only one processes
}

#[tokio::test]
#[serial]
async fn test_retry_with_backoff() {
    // Simulate failures, verify retry schedule
}

#[tokio::test]
#[serial]
async fn test_priority_queue_ordering() {
    // Create multiple refunds, verify processing order
}

#[tokio::test]
#[serial]
async fn test_gas_escalation() {
    // Verify gas price increases with retries
}
```

### 6.3 Load Tests

```rust
#[tokio::test]
#[ignore] // Run separately
async fn test_concurrent_refund_processing() {
    // Process 1000 refunds concurrently
    // Verify no deadlocks or race conditions
}

#[tokio::test]
#[ignore]
async fn test_timeout_detection_performance() {
    // Scan 10,000 swaps
    // Verify < 1 second completion time
}
```

## 7. Security & Compliance

### 7.1 Access Control
- Refund initiation requires system-level permissions
- Manual overrides logged with admin identity
- Refund address validation against original swap
- Rate limiting on refund operations per user

### 7.2 Fraud Prevention
- Maximum refund amount = deposit amount
- Refund address must match swap refund_address field
- Anomaly detection for unusual refund patterns
- Multi-signature requirement for large refunds (> $10,000)

### 7.3 Audit Trail
- Complete history of all state transitions
- Transaction hashes recorded for all refunds
- Failed attempts logged with error details
- Admin actions tracked with timestamps
- Immutable audit log (append-only)

## 8. Success Criteria

- ✅ 99.9% refund success rate
- ✅ < 5 minute detection latency for timeouts
- ✅ < 30 minute average refund completion time
- ✅ Zero duplicate refunds (idempotency)
- ✅ < 1% manual intervention rate
- ✅ Complete audit trail for all operations
- ✅ Comprehensive test coverage (>90%)
- ✅ Prometheus metrics exported
- ✅ Grafana dashboards configured
- ✅ Documentation complete

## 9. References

Content rephrased for compliance with licensing restrictions:

- [AWS Builders Library](https://aws.amazon.com/builders-library/timeouts-retries-and-backoff-with-jitter/) - Timeout and retry patterns
- [Hash Time-Locked Contracts](https://lendasat.com/docs/lendaswap/htlc) - HTLC refund mechanisms
- [Atomic Swap Protocols](https://www.garden.finance/blog/what-are-htlcs) - Timelock-based refunds
- Industry practices from Changelly, Coinify, and other exchanges
- Existing codebase patterns: webhook system, RPC management, metrics collection
