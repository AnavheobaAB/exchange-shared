# ERC-20 Token Support Design

## Executive Summary

This document outlines a production-ready system for supporting ERC-20 token transfers on the exchange platform. The design extends the existing native token support (ETH, BTC, SOL) to include thousands of ERC-20 tokens across multiple EVM-compatible chains. The implementation leverages alloy-rs for contract interaction, incorporates gas optimization techniques, handles variable decimal precision, and provides a scalable token registry architecture.

## 1. System Requirements

### Functional Requirements
- Support ERC-20 token transfers on EVM chains (Ethereum, BSC, Polygon, Arbitrum, etc.)
- Automatic token contract detection and validation
- Handle approve/transferFrom pattern for token transfers
- Support variable decimal precision (6, 8, 18 decimals)
- Gas estimation for token transfers (higher than native transfers)
- Token balance checking and validation
- Multi-chain token registry with contract addresses
- Automatic token metadata fetching (name, symbol, decimals)

### Non-Functional Requirements
- **Performance**: < 200ms token transfer submission
- **Reliability**: 99.9% successful token transfers
- **Scalability**: Support 1000+ tokens across 10+ chains
- **Security**: Contract address validation, reentrancy protection
- **Gas Efficiency**: Optimal gas usage for approve + transfer operations

## 2. ERC-20 Standard Overview

### 2.1 Core Functions

Content rephrased for compliance with licensing restrictions:

ERC-20 defines six essential functions that all compliant tokens must implement:

```solidity
// Read-only functions
function totalSupply() external view returns (uint256);
function balanceOf(address account) external view returns (uint256);
function allowance(address owner, address spender) external view returns (uint256);

// State-changing functions
function transfer(address recipient, uint256 amount) external returns (bool);
function approve(address spender, uint256 amount) external returns (bool);
function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);
```

### 2.2 Transfer Patterns

**Direct Transfer** (user-to-user):
```
User A → transfer(User B, amount) → User B receives tokens
Gas Cost: ~50,000-65,000 gas
```

**Delegated Transfer** (contract-mediated):
```
Step 1: User A → approve(Contract, amount) → Contract gets allowance
Step 2: Contract → transferFrom(User A, User B, amount) → User B receives tokens
Total Gas Cost: ~70,000-90,000 gas (approve) + ~65,000-80,000 gas (transferFrom)
```


### 2.3 Decimal Precision Handling

**Key Concept**: EVM doesn't support floating-point numbers. All token amounts are stored as integers.

**Formula**:
```
actual_amount = display_amount × 10^decimals

Examples:
- 1.5 USDT (6 decimals) = 1,500,000 (1.5 × 10^6)
- 0.5 DAI (18 decimals) = 500,000,000,000,000,000 (0.5 × 10^18)
- 100 USDC (6 decimals) = 100,000,000 (100 × 10^6)
```

**Common Decimal Values**:
- 18 decimals: ETH, DAI, WETH, most ERC-20 tokens (standard)
- 6 decimals: USDT, USDC (stablecoins)
- 8 decimals: WBTC (matches Bitcoin)

**Conversion Functions**:
```rust
// Display amount to base units
fn to_base_units(amount: Decimal, decimals: u8) -> U256 {
    let multiplier = Decimal::from(10u64.pow(decimals as u32));
    let base_amount = amount * multiplier;
    U256::from_str(&base_amount.to_string()).unwrap()
}

// Base units to display amount
fn from_base_units(amount: U256, decimals: u8) -> Decimal {
    let divisor = Decimal::from(10u64.pow(decimals as u32));
    Decimal::from_str(&amount.to_string()).unwrap() / divisor
}
```

## 3. Mathematical Foundations

### 3.1 Gas Cost Estimation

**Problem**: ERC-20 transfers cost more gas than native ETH transfers.

**Gas Cost Breakdown**:
```
Native ETH transfer: 21,000 gas (base transaction)

ERC-20 transfer():
- Base transaction: 21,000 gas
- SLOAD (balance check): 2,100 gas
- SSTORE (balance update): 20,000 gas × 2 (sender + recipient)
- Event emission: 375 gas
- Total: ~50,000-65,000 gas

ERC-20 approve():
- Base transaction: 21,000 gas
- SSTORE (allowance): 20,000 gas
- Event emission: 375 gas
- Total: ~45,000-50,000 gas

ERC-20 transferFrom():
- Base transaction: 21,000 gas
- SLOAD (allowance check): 2,100 gas
- SSTORE (allowance update): 5,000 gas
- SSTORE (balance update): 20,000 gas × 2
- Event emission: 375 gas
- Total: ~65,000-80,000 gas
```

**Gas Multiplier Formula**:
```
For our exchange platform:
gas_limit_token = gas_limit_native × multiplier

Multipliers:
- transfer(): 3.0x (65,000 / 21,000)
- approve(): 2.5x (50,000 / 21,000)
- transferFrom(): 3.5x (80,000 / 21,000)

Safety margin: Add 20% buffer for complex tokens
final_gas_limit = gas_limit_token × 1.2
```

### 3.2 Approval Amount Strategy

**Problem**: Should we approve exact amount or unlimited amount?

**Options**:
```
Option 1: Exact Approval
approve(spender, exact_amount)
Pros: Maximum security, no risk of over-spending
Cons: Requires approval for every transfer, 2x gas cost

Option 2: Unlimited Approval
approve(spender, type(uint256).max)
Pros: One-time approval, saves gas on future transfers
Cons: Security risk if contract is compromised

Option 3: Batch Approval (Recommended)
approve(spender, amount × batch_size)
Pros: Balance between security and gas efficiency
Cons: Requires tracking remaining allowance
```

**Recommended Strategy**:
```
For exchange platform:
1. Check existing allowance first
2. If allowance >= required_amount: skip approval
3. If allowance < required_amount: approve(required_amount × 2)
4. This reduces approval transactions by ~50%

Formula:
if current_allowance < required_amount:
    new_approval = required_amount × 2  // Approve 2x for next swap
    approve(contract, new_approval)
```


### 3.3 Token Balance Validation

**Problem**: Ensure user has sufficient token balance before initiating swap.

**Validation Formula**:
```
required_balance = swap_amount + estimated_gas_cost_in_tokens

For token swaps:
1. Check token balance: balanceOf(user_address)
2. Check native balance for gas: user_eth_balance >= gas_estimate × gas_price
3. Validate: token_balance >= swap_amount

Validation logic:
if token_balance < swap_amount:
    return Error("Insufficient token balance")
if native_balance < gas_cost:
    return Error("Insufficient ETH for gas")
```

### 3.4 Slippage Protection for Token Swaps

**Problem**: Token prices can change between approval and transfer.

**Formula**:
```
min_receive_amount = expected_amount × (1 - slippage_tolerance)

where:
- slippage_tolerance = 0.005 (0.5% default)
- expected_amount = from_amount × exchange_rate

Example:
Swap 1000 USDT → ETH at rate 0.0003 ETH/USDT
expected_amount = 1000 × 0.0003 = 0.3 ETH
min_receive = 0.3 × (1 - 0.005) = 0.2985 ETH

If actual_receive < min_receive: revert transaction
```

## 4. System Architecture

### 4.1 Module Structure

```
src/services/token/
├── mod.rs                  # Public API and module exports
├── types.rs                # Token types, enums, and errors
├── registry.rs             # Token registry and metadata
├── erc20_client.rs         # ERC-20 contract interaction
├── balance_checker.rs      # Balance validation
├── approval_manager.rs     # Approval optimization
├── gas_estimator.rs        # Token-specific gas estimation
└── metadata_fetcher.rs     # Automatic token metadata fetching

src/services/wallet/
├── erc20_signer.rs         # ERC-20 transaction signing
└── token_payout.rs         # Token payout execution

tests/token/
├── mod.rs
├── erc20_client_test.rs
├── registry_test.rs
├── approval_test.rs
└── token_integration_test.rs
```

### 4.2 Database Schema

```sql
-- Token registry table
CREATE TABLE tokens (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    symbol VARCHAR(20) NOT NULL,
    name VARCHAR(100) NOT NULL,
    network VARCHAR(50) NOT NULL,
    contract_address VARCHAR(42) NOT NULL,  -- 0x + 40 hex chars
    decimals INT NOT NULL,
    token_type ENUM('NATIVE', 'ERC20', 'BEP20', 'TRC20', 'SPL') NOT NULL DEFAULT 'ERC20',
    
    -- Metadata
    logo_url VARCHAR(255),
    coingecko_id VARCHAR(100),
    coinmarketcap_id VARCHAR(100),
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- Limits
    min_swap_amount DECIMAL(20, 8),
    max_swap_amount DECIMAL(20, 8),
    
    -- Gas configuration
    gas_multiplier DECIMAL(5, 2) DEFAULT 3.0,
    
    -- Audit
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    UNIQUE KEY uk_token_contract_network (contract_address, network),
    INDEX idx_tokens_symbol (symbol),
    INDEX idx_tokens_network (network),
    INDEX idx_tokens_is_active (is_active),
    INDEX idx_tokens_token_type (token_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Token approvals tracking (for optimization)
CREATE TABLE token_approvals (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    user_address VARCHAR(42) NOT NULL,
    token_address VARCHAR(42) NOT NULL,
    spender_address VARCHAR(42) NOT NULL,
    network VARCHAR(50) NOT NULL,
    
    -- Approval details
    approved_amount DECIMAL(30, 0) NOT NULL,  -- Store as base units
    remaining_amount DECIMAL(30, 0) NOT NULL,
    
    -- Transaction details
    tx_hash VARCHAR(66) NOT NULL,
    block_number BIGINT NOT NULL,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    
    -- Timestamps
    approved_at TIMESTAMP NOT NULL,
    last_used_at TIMESTAMP,
    expires_at TIMESTAMP,  -- Optional expiry
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    UNIQUE KEY uk_approval (user_address, token_address, spender_address, network),
    INDEX idx_approvals_user (user_address),
    INDEX idx_approvals_token (token_address),
    INDEX idx_approvals_active (is_active),
    INDEX idx_approvals_expires (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Token transfer history
CREATE TABLE token_transfers (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    swap_id VARCHAR(36),
    token_address VARCHAR(42) NOT NULL,
    network VARCHAR(50) NOT NULL,
    
    -- Transfer details
    from_address VARCHAR(42) NOT NULL,
    to_address VARCHAR(42) NOT NULL,
    amount DECIMAL(30, 0) NOT NULL,  -- Base units
    decimals INT NOT NULL,
    
    -- Transaction details
    tx_hash VARCHAR(66) NOT NULL,
    block_number BIGINT,
    gas_used BIGINT,
    gas_price DECIMAL(20, 8),
    
    -- Status
    status ENUM('PENDING', 'CONFIRMED', 'FAILED') NOT NULL DEFAULT 'PENDING',
    confirmations INT DEFAULT 0,
    error_message TEXT,
    
    -- Timestamps
    submitted_at TIMESTAMP NOT NULL,
    confirmed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_transfers_swap (swap_id),
    INDEX idx_transfers_token (token_address),
    INDEX idx_transfers_tx_hash (tx_hash),
    INDEX idx_transfers_status (status),
    INDEX idx_transfers_from (from_address),
    INDEX idx_transfers_to (to_address),
    
    FOREIGN KEY (swap_id) REFERENCES swaps(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
```


### 4.3 Type Definitions (types.rs)

```rust
use serde::{Deserialize, Serialize};
use alloy::primitives::{Address, U256};
use rust_decimal::Decimal;

/// Token type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenType {
    Native,    // ETH, BTC, SOL
    Erc20,     // Ethereum ERC-20
    Bep20,     // BSC BEP-20
    Trc20,     // Tron TRC-20
    Spl,       // Solana SPL
}

/// Token metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: i64,
    pub symbol: String,
    pub name: String,
    pub network: String,
    pub contract_address: Option<Address>,
    pub decimals: u8,
    pub token_type: TokenType,
    
    // Metadata
    pub logo_url: Option<String>,
    pub coingecko_id: Option<String>,
    
    // Status
    pub is_active: bool,
    pub is_verified: bool,
    
    // Limits
    pub min_swap_amount: Option<Decimal>,
    pub max_swap_amount: Option<Decimal>,
    
    // Gas configuration
    pub gas_multiplier: Decimal,
}

impl Token {
    pub fn is_native(&self) -> bool {
        self.token_type == TokenType::Native
    }
    
    pub fn is_erc20(&self) -> bool {
        matches!(self.token_type, TokenType::Erc20 | TokenType::Bep20)
    }
    
    pub fn requires_approval(&self) -> bool {
        self.is_erc20()
    }
}

/// Token balance information
#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub token_address: Address,
    pub owner_address: Address,
    pub balance: U256,
    pub decimals: u8,
    pub balance_decimal: Decimal,
}

/// Token approval information
#[derive(Debug, Clone)]
pub struct TokenApproval {
    pub token_address: Address,
    pub owner_address: Address,
    pub spender_address: Address,
    pub allowance: U256,
    pub decimals: u8,
    pub allowance_decimal: Decimal,
}

/// Token transfer request
#[derive(Debug, Clone)]
pub struct TokenTransferRequest {
    pub token_address: Address,
    pub from_address: Address,
    pub to_address: Address,
    pub amount: U256,
    pub decimals: u8,
    pub network: String,
    pub gas_price: Option<U256>,
    pub gas_limit: Option<u64>,
}

/// Token errors
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("Token not found: {0}")]
    TokenNotFound(String),
    
    #[error("Invalid contract address: {0}")]
    InvalidContractAddress(String),
    
    #[error("Insufficient token balance: required {required}, available {available}")]
    InsufficientBalance { required: String, available: String },
    
    #[error("Insufficient allowance: required {required}, approved {approved}")]
    InsufficientAllowance { required: String, approved: String },
    
    #[error("Token not active: {0}")]
    TokenNotActive(String),
    
    #[error("Invalid decimals: {0}")]
    InvalidDecimals(u8),
    
    #[error("Contract call failed: {0}")]
    ContractCallFailed(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("RPC error: {0}")]
    Rpc(String),
}
```

### 4.4 ERC-20 Client Implementation (erc20_client.rs)

```rust
use alloy::{
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    sol,
};
use std::sync::Arc;

use crate::services::token::{TokenError, TokenBalance, TokenApproval};

// Generate ERC-20 contract bindings
sol! {
    #[sol(rpc)]
    contract IERC20 {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
        function allowance(address owner, address spender) external view returns (uint256);
        function transfer(address recipient, uint256 amount) external returns (bool);
        function approve(address spender, uint256 amount) external returns (bool);
        function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);
        
        event Transfer(address indexed from, address indexed to, uint256 value);
        event Approval(address indexed owner, address indexed spender, uint256 value);
    }
}

pub struct Erc20Client<P> {
    provider: Arc<P>,
}

impl<P: Provider> Erc20Client<P> {
    pub fn new(provider: Arc<P>) -> Self {
        Self { provider }
    }
    
    /// Get token metadata (name, symbol, decimals)
    pub async fn get_metadata(&self, token_address: Address) -> Result<(String, String, u8), TokenError> {
        let contract = IERC20::new(token_address, self.provider.clone());
        
        let name = contract.name().call().await
            .map_err(|e| TokenError::ContractCallFailed(e.to_string()))?
            ._0;
        
        let symbol = contract.symbol().call().await
            .map_err(|e| TokenError::ContractCallFailed(e.to_string()))?
            ._0;
        
        let decimals = contract.decimals().call().await
            .map_err(|e| TokenError::ContractCallFailed(e.to_string()))?
            ._0;
        
        Ok((name, symbol, decimals))
    }
    
    /// Get token balance for an address
    pub async fn get_balance(&self, token_address: Address, owner: Address) -> Result<TokenBalance, TokenError> {
        let contract = IERC20::new(token_address, self.provider.clone());
        
        let balance = contract.balanceOf(owner).call().await
            .map_err(|e| TokenError::ContractCallFailed(e.to_string()))?
            ._0;
        
        let decimals = contract.decimals().call().await
            .map_err(|e| TokenError::ContractCallFailed(e.to_string()))?
            ._0;
        
        let balance_decimal = Self::to_decimal(balance, decimals);
        
        Ok(TokenBalance {
            token_address,
            owner_address: owner,
            balance,
            decimals,
            balance_decimal,
        })
    }
    
    /// Get token allowance
    pub async fn get_allowance(
        &self,
        token_address: Address,
        owner: Address,
        spender: Address,
    ) -> Result<TokenApproval, TokenError> {
        let contract = IERC20::new(token_address, self.provider.clone());
        
        let allowance = contract.allowance(owner, spender).call().await
            .map_err(|e| TokenError::ContractCallFailed(e.to_string()))?
            ._0;
        
        let decimals = contract.decimals().call().await
            .map_err(|e| TokenError::ContractCallFailed(e.to_string()))?
            ._0;
        
        let allowance_decimal = Self::to_decimal(allowance, decimals);
        
        Ok(TokenApproval {
            token_address,
            owner_address: owner,
            spender_address: spender,
            allowance,
            decimals,
            allowance_decimal,
        })
    }
    
    /// Helper: Convert U256 to Decimal
    fn to_decimal(amount: U256, decimals: u8) -> rust_decimal::Decimal {
        let amount_str = amount.to_string();
        let amount_dec = rust_decimal::Decimal::from_str_exact(&amount_str).unwrap();
        let divisor = rust_decimal::Decimal::from(10u64.pow(decimals as u32));
        amount_dec / divisor
    }
    
    /// Helper: Convert Decimal to U256
    pub fn to_base_units(amount: rust_decimal::Decimal, decimals: u8) -> U256 {
        let multiplier = rust_decimal::Decimal::from(10u64.pow(decimals as u32));
        let base_amount = amount * multiplier;
        U256::from_str_radix(&base_amount.to_string(), 10).unwrap()
    }
}
```


### 4.5 Token Registry (registry.rs)

```rust
use sqlx::MySqlPool;
use alloy::primitives::Address;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::services::token::{Token, TokenType, TokenError};

pub struct TokenRegistry {
    pool: MySqlPool,
    cache: Arc<RwLock<HashMap<String, Token>>>,  // key: "network:contract_address"
}

impl TokenRegistry {
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get token by contract address and network
    pub async fn get_token(&self, contract_address: &str, network: &str) -> Result<Token, TokenError> {
        let cache_key = format!("{}:{}", network, contract_address);
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(token) = cache.get(&cache_key) {
                return Ok(token.clone());
            }
        }
        
        // Query database
        let token = sqlx::query_as!(
            Token,
            r#"
            SELECT 
                id,
                symbol,
                name,
                network,
                contract_address,
                decimals as "decimals: u8",
                token_type as "token_type: TokenType",
                logo_url,
                coingecko_id,
                is_active,
                is_verified,
                min_swap_amount,
                max_swap_amount,
                gas_multiplier
            FROM tokens
            WHERE contract_address = ? AND network = ? AND is_active = TRUE
            "#,
            contract_address,
            network
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| TokenError::TokenNotFound(format!("{} on {}", contract_address, network)))?;
        
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, token.clone());
        }
        
        Ok(token)
    }
    
    /// Get token by symbol and network
    pub async fn get_token_by_symbol(&self, symbol: &str, network: &str) -> Result<Token, TokenError> {
        let token = sqlx::query_as!(
            Token,
            r#"
            SELECT 
                id,
                symbol,
                name,
                network,
                contract_address,
                decimals as "decimals: u8",
                token_type as "token_type: TokenType",
                logo_url,
                coingecko_id,
                is_active,
                is_verified,
                min_swap_amount,
                max_swap_amount,
                gas_multiplier
            FROM tokens
            WHERE symbol = ? AND network = ? AND is_active = TRUE
            LIMIT 1
            "#,
            symbol,
            network
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| TokenError::TokenNotFound(format!("{} on {}", symbol, network)))?;
        
        Ok(token)
    }
    
    /// Register a new token
    pub async fn register_token(
        &self,
        symbol: &str,
        name: &str,
        network: &str,
        contract_address: Address,
        decimals: u8,
        token_type: TokenType,
    ) -> Result<i64, TokenError> {
        let contract_addr_str = format!("{:?}", contract_address);
        
        let result = sqlx::query!(
            r#"
            INSERT INTO tokens (symbol, name, network, contract_address, decimals, token_type, is_verified)
            VALUES (?, ?, ?, ?, ?, ?, FALSE)
            "#,
            symbol,
            name,
            network,
            contract_addr_str,
            decimals,
            token_type
        )
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_id() as i64)
    }
    
    /// List all active tokens for a network
    pub async fn list_tokens(&self, network: &str) -> Result<Vec<Token>, TokenError> {
        let tokens = sqlx::query_as!(
            Token,
            r#"
            SELECT 
                id,
                symbol,
                name,
                network,
                contract_address,
                decimals as "decimals: u8",
                token_type as "token_type: TokenType",
                logo_url,
                coingecko_id,
                is_active,
                is_verified,
                min_swap_amount,
                max_swap_amount,
                gas_multiplier
            FROM tokens
            WHERE network = ? AND is_active = TRUE
            ORDER BY is_verified DESC, symbol ASC
            "#,
            network
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(tokens)
    }
    
    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}
```

### 4.6 Approval Manager (approval_manager.rs)

```rust
use sqlx::MySqlPool;
use alloy::primitives::{Address, U256};
use chrono::{DateTime, Utc};

use crate::services::token::{TokenError, Erc20Client};

pub struct ApprovalManager {
    pool: MySqlPool,
}

impl ApprovalManager {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
    
    /// Check if approval is needed
    pub async fn needs_approval(
        &self,
        user_address: Address,
        token_address: Address,
        spender_address: Address,
        required_amount: U256,
        network: &str,
    ) -> Result<bool, TokenError> {
        // Check database cache first
        let cached_approval = sqlx::query!(
            r#"
            SELECT remaining_amount
            FROM token_approvals
            WHERE user_address = ?
              AND token_address = ?
              AND spender_address = ?
              AND network = ?
              AND is_active = TRUE
              AND (expires_at IS NULL OR expires_at > NOW())
            "#,
            format!("{:?}", user_address),
            format!("{:?}", token_address),
            format!("{:?}", spender_address),
            network
        )
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(approval) = cached_approval {
            let remaining = U256::from_str_radix(&approval.remaining_amount.to_string(), 10)
                .map_err(|e| TokenError::ContractCallFailed(e.to_string()))?;
            
            if remaining >= required_amount {
                return Ok(false);  // Sufficient approval exists
            }
        }
        
        Ok(true)  // Need new approval
    }
    
    /// Calculate optimal approval amount
    pub fn calculate_approval_amount(&self, required_amount: U256) -> U256 {
        // Approve 2x the required amount for future swaps
        required_amount * U256::from(2)
    }
    
    /// Record approval in database
    pub async fn record_approval(
        &self,
        user_address: Address,
        token_address: Address,
        spender_address: Address,
        approved_amount: U256,
        network: &str,
        tx_hash: &str,
        block_number: u64,
    ) -> Result<(), TokenError> {
        sqlx::query!(
            r#"
            INSERT INTO token_approvals 
            (user_address, token_address, spender_address, network, approved_amount, remaining_amount, tx_hash, block_number, approved_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, NOW())
            ON DUPLICATE KEY UPDATE
                approved_amount = VALUES(approved_amount),
                remaining_amount = VALUES(remaining_amount),
                tx_hash = VALUES(tx_hash),
                block_number = VALUES(block_number),
                approved_at = NOW(),
                is_active = TRUE
            "#,
            format!("{:?}", user_address),
            format!("{:?}", token_address),
            format!("{:?}", spender_address),
            network,
            approved_amount.to_string(),
            approved_amount.to_string(),
            tx_hash,
            block_number as i64
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Update remaining allowance after transfer
    pub async fn update_allowance(
        &self,
        user_address: Address,
        token_address: Address,
        spender_address: Address,
        used_amount: U256,
        network: &str,
    ) -> Result<(), TokenError> {
        sqlx::query!(
            r#"
            UPDATE token_approvals
            SET remaining_amount = GREATEST(0, CAST(remaining_amount AS SIGNED) - ?),
                last_used_at = NOW()
            WHERE user_address = ?
              AND token_address = ?
              AND spender_address = ?
              AND network = ?
              AND is_active = TRUE
            "#,
            used_amount.to_string(),
            format!("{:?}", user_address),
            format!("{:?}", token_address),
            format!("{:?}", spender_address),
            network
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}
```


### 4.7 Token Gas Estimator (gas_estimator.rs)

```rust
use alloy::primitives::U256;
use rust_decimal::Decimal;

use crate::services::token::{Token, TokenType};

pub struct TokenGasEstimator;

impl TokenGasEstimator {
    /// Estimate gas limit for token transfer
    pub fn estimate_transfer_gas(token: &Token) -> u64 {
        match token.token_type {
            TokenType::Native => 21_000,
            TokenType::Erc20 | TokenType::Bep20 => {
                let base_gas = 65_000u64;
                let multiplier = token.gas_multiplier.to_f64().unwrap_or(3.0);
                let safety_margin = 1.2;
                (base_gas as f64 * multiplier * safety_margin) as u64
            }
            TokenType::Trc20 => 100_000,  // Tron uses different gas model
            TokenType::Spl => 5_000,      // Solana uses compute units
        }
    }
    
    /// Estimate gas limit for approval
    pub fn estimate_approval_gas(token: &Token) -> u64 {
        match token.token_type {
            TokenType::Native => 0,  // No approval needed
            TokenType::Erc20 | TokenType::Bep20 => {
                let base_gas = 50_000u64;
                let safety_margin = 1.2;
                (base_gas as f64 * safety_margin) as u64
            }
            TokenType::Trc20 => 80_000,
            TokenType::Spl => 0,  // SPL uses different approval model
        }
    }
    
    /// Estimate total gas cost in native currency
    pub fn estimate_gas_cost(
        token: &Token,
        gas_price: U256,
        include_approval: bool,
    ) -> U256 {
        let transfer_gas = Self::estimate_transfer_gas(token);
        let approval_gas = if include_approval {
            Self::estimate_approval_gas(token)
        } else {
            0
        };
        
        let total_gas = U256::from(transfer_gas + approval_gas);
        total_gas * gas_price
    }
    
    /// Calculate gas multiplier based on token complexity
    pub fn calculate_gas_multiplier(token: &Token) -> Decimal {
        // Base multiplier
        let mut multiplier = Decimal::from(3);
        
        // Adjust for token characteristics
        if !token.is_verified {
            multiplier += Decimal::from_str_exact("0.5").unwrap();  // Unverified tokens may be complex
        }
        
        // Cap at reasonable maximum
        multiplier.min(Decimal::from(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_native_gas_estimation() {
        let token = Token {
            token_type: TokenType::Native,
            gas_multiplier: Decimal::from(1),
            ..Default::default()
        };
        
        assert_eq!(TokenGasEstimator::estimate_transfer_gas(&token), 21_000);
        assert_eq!(TokenGasEstimator::estimate_approval_gas(&token), 0);
    }
    
    #[test]
    fn test_erc20_gas_estimation() {
        let token = Token {
            token_type: TokenType::Erc20,
            gas_multiplier: Decimal::from(3),
            is_verified: true,
            ..Default::default()
        };
        
        let transfer_gas = TokenGasEstimator::estimate_transfer_gas(&token);
        assert!(transfer_gas > 65_000 && transfer_gas < 300_000);
        
        let approval_gas = TokenGasEstimator::estimate_approval_gas(&token);
        assert!(approval_gas > 50_000 && approval_gas < 100_000);
    }
}
```

## 5. Integration with Existing Systems

### 5.1 Wallet Manager Integration

Extend `src/services/wallet/manager.rs` to support ERC-20 tokens:

```rust
impl WalletManager {
    /// Process token payout (ERC-20)
    pub async fn process_token_payout(
        &self,
        swap_id: Uuid,
        token_address: Address,
        recipient_address: Address,
        amount: U256,
        network: &str,
    ) -> Result<String, WalletError> {
        // 1. Get token metadata
        let token = self.token_registry.get_token(&format!("{:?}", token_address), network).await?;
        
        // 2. Check if approval is needed
        let needs_approval = self.approval_manager.needs_approval(
            self.platform_address,
            token_address,
            recipient_address,
            amount,
            network,
        ).await?;
        
        // 3. If needed, approve tokens
        if needs_approval {
            let approval_amount = self.approval_manager.calculate_approval_amount(amount);
            let approval_tx = self.approve_tokens(
                token_address,
                recipient_address,
                approval_amount,
                network,
            ).await?;
            
            // Wait for approval confirmation
            self.wait_for_confirmation(&approval_tx, network).await?;
        }
        
        // 4. Execute token transfer
        let transfer_tx = self.transfer_tokens(
            token_address,
            recipient_address,
            amount,
            network,
        ).await?;
        
        // 5. Record transfer in database
        self.record_token_transfer(swap_id, token_address, recipient_address, amount, &transfer_tx, network).await?;
        
        Ok(transfer_tx)
    }
    
    async fn approve_tokens(
        &self,
        token_address: Address,
        spender: Address,
        amount: U256,
        network: &str,
    ) -> Result<String, WalletError> {
        // Build approval transaction
        let contract = IERC20::new(token_address, self.provider.clone());
        let tx = contract.approve(spender, amount);
        
        // Sign and send
        let pending_tx = tx.send().await?;
        let tx_hash = format!("{:?}", pending_tx.tx_hash());
        
        // Wait for confirmation
        let receipt = pending_tx.watch().await?;
        
        // Record approval
        self.approval_manager.record_approval(
            self.platform_address,
            token_address,
            spender,
            amount,
            network,
            &tx_hash,
            receipt.block_number.unwrap(),
        ).await?;
        
        Ok(tx_hash)
    }
    
    async fn transfer_tokens(
        &self,
        token_address: Address,
        recipient: Address,
        amount: U256,
        network: &str,
    ) -> Result<String, WalletError> {
        // Build transfer transaction
        let contract = IERC20::new(token_address, self.provider.clone());
        let tx = contract.transfer(recipient, amount);
        
        // Sign and send
        let pending_tx = tx.send().await?;
        let tx_hash = format!("{:?}", pending_tx.tx_hash());
        
        // Wait for confirmation
        pending_tx.watch().await?;
        
        Ok(tx_hash)
    }
}
```

### 5.2 Swap CRUD Integration

Extend `src/modules/swap/crud.rs` to handle token swaps:

```rust
impl SwapCrud {
    /// Create swap with token support
    pub async fn create_swap_with_token(
        &self,
        user_id: Uuid,
        from_token: &Token,
        to_token: &Token,
        amount: Decimal,
        recipient_address: &str,
    ) -> Result<Swap, SwapError> {
        // 1. Validate token balances
        if from_token.is_erc20() {
            self.validate_token_balance(user_id, from_token, amount).await?;
        }
        
        // 2. Calculate gas costs (higher for tokens)
        let gas_cost = if from_token.is_erc20() {
            self.estimate_token_gas_cost(from_token).await?
        } else {
            self.estimate_native_gas_cost(&from_token.network).await?
        };
        
        // 3. Get exchange rate from provider
        let rate = self.get_rate(from_token, to_token, amount).await?;
        
        // 4. Calculate fees
        let fees = self.calculate_fees(amount, gas_cost, from_token);
        
        // 5. Create swap record
        let swap = self.create_swap_record(
            user_id,
            from_token,
            to_token,
            amount,
            rate,
            fees,
            recipient_address,
        ).await?;
        
        Ok(swap)
    }
    
    async fn validate_token_balance(
        &self,
        user_id: Uuid,
        token: &Token,
        required_amount: Decimal,
    ) -> Result<(), SwapError> {
        let user_address = self.get_user_address(user_id).await?;
        
        let balance = self.erc20_client.get_balance(
            token.contract_address.unwrap(),
            user_address,
        ).await?;
        
        if balance.balance_decimal < required_amount {
            return Err(SwapError::InsufficientBalance {
                required: required_amount.to_string(),
                available: balance.balance_decimal.to_string(),
            });
        }
        
        Ok(())
    }
}
```


## 6. Implementation Strategy

### 6.1 Phase 1: Core Infrastructure (Week 1)
**Files to create**:
- `src/services/token/types.rs` - Core types and errors
- `src/services/token/mod.rs` - Module exports
- `migrations/009_tokens.sql` - Database schema
- `Cargo.toml` - Add alloy dependencies

**Tasks**:
- Define all types, enums, and error types
- Create database migration for tokens, approvals, transfers
- Add alloy-rs dependencies to Cargo.toml
- Add comprehensive unit tests for types

**Dependencies to add**:
```toml
[dependencies]
alloy = { version = "0.6", features = ["full"] }
alloy-primitives = "0.8"
alloy-sol-types = "0.8"
```

**Success Criteria**:
- All types compile without errors
- Migration applies successfully
- 100% test coverage for types module

### 6.2 Phase 2: ERC-20 Client (Week 1-2)
**Files to create**:
- `src/services/token/erc20_client.rs` - Contract interaction
- `src/services/token/registry.rs` - Token registry
- `tests/token/erc20_client_test.rs` - Client tests

**Tasks**:
- Implement ERC-20 contract bindings with alloy sol! macro
- Create methods for balance, allowance, metadata fetching
- Build token registry with database and cache
- Write integration tests with test tokens

**Success Criteria**:
- Can read token metadata from mainnet
- Balance and allowance queries work
- Registry caches tokens correctly
- 90%+ test coverage

### 6.3 Phase 3: Approval Management (Week 2)
**Files to create**:
- `src/services/token/approval_manager.rs` - Approval optimization
- `src/services/token/gas_estimator.rs` - Gas estimation
- `tests/token/approval_test.rs` - Approval tests

**Tasks**:
- Implement approval checking and optimization
- Create gas estimation for token operations
- Build approval tracking in database
- Add metrics for approval operations

**Success Criteria**:
- Approval optimization reduces gas by 50%
- Gas estimates accurate within 20%
- Approval tracking works correctly
- Metrics exported to Prometheus

### 6.4 Phase 4: Wallet Integration (Week 2-3)
**Files to create**:
- `src/services/wallet/token_payout.rs` - Token payout execution
- `tests/wallet/token_payout_test.rs` - Payout tests

**Tasks**:
- Extend WalletManager for token payouts
- Implement approve + transfer flow
- Add transaction confirmation tracking
- Integrate with existing payout system

**Success Criteria**:
- Token payouts execute successfully
- Approval + transfer flow works end-to-end
- Confirmations tracked correctly
- Integration with existing wallet service

### 6.5 Phase 5: Swap Integration (Week 3-4)
**Files to create**:
- Update `src/modules/swap/crud.rs` - Token swap support
- Update `src/modules/swap/schema.rs` - Token DTOs
- `tests/swap/token_swap_test.rs` - Token swap tests

**Tasks**:
- Extend swap creation for token swaps
- Add token balance validation
- Update fee calculation for token gas costs
- Create comprehensive integration tests

**Success Criteria**:
- Token swaps work end-to-end
- Balance validation prevents insufficient funds
- Fees calculated correctly
- 95%+ test coverage

### 6.6 Phase 6: Token Registry Population (Week 4)
**Tasks**:
- Populate database with top 100 ERC-20 tokens
- Add token logos and metadata
- Verify contract addresses on multiple chains
- Create admin API for token management

**Token List Priority**:
1. Stablecoins: USDT, USDC, DAI, BUSD (all chains)
2. Wrapped tokens: WETH, WBTC
3. DeFi tokens: UNI, AAVE, LINK, COMP
4. Exchange tokens: BNB, CRO, FTT
5. Top 50 by market cap

**Success Criteria**:
- 100+ tokens registered across 5+ chains
- All tokens verified and tested
- Metadata complete with logos
- Admin API functional

## 7. Testing Strategy

### 7.1 Unit Tests

```rust
// tests/token/types_test.rs
#[test]
fn test_token_type_classification() { }

#[test]
fn test_decimal_conversion() { }

#[test]
fn test_base_units_conversion() { }

// tests/token/gas_estimator_test.rs
#[test]
fn test_native_gas_estimation() { }

#[test]
fn test_erc20_gas_estimation() { }

#[test]
fn test_gas_multiplier_calculation() { }

// tests/token/approval_manager_test.rs
#[test]
fn test_approval_optimization() { }

#[test]
fn test_allowance_tracking() { }
```

### 7.2 Integration Tests

```rust
// tests/token/erc20_client_test.rs
#[tokio::test]
#[serial]
async fn test_get_token_metadata() {
    // Test with real USDT contract on testnet
}

#[tokio::test]
#[serial]
async fn test_get_token_balance() {
    // Test balance query
}

#[tokio::test]
#[serial]
async fn test_get_allowance() {
    // Test allowance query
}

// tests/token/token_integration_test.rs
#[tokio::test]
#[serial]
async fn test_end_to_end_token_swap() {
    // 1. Create token swap
    // 2. Check approval needed
    // 3. Approve tokens
    // 4. Execute transfer
    // 5. Verify completion
}

#[tokio::test]
#[serial]
async fn test_approval_optimization() {
    // Test that second swap reuses approval
}
```

### 7.3 Load Tests

```rust
#[tokio::test]
#[ignore]
async fn test_concurrent_token_transfers() {
    // Process 100 token transfers concurrently
}

#[tokio::test]
#[ignore]
async fn test_registry_cache_performance() {
    // Query 1000 tokens, verify cache hit rate > 90%
}
```

## 8. Security Considerations

### 8.1 Contract Address Validation
- Verify contract addresses against known registries (Etherscan, CoinGecko)
- Implement checksum validation for addresses
- Maintain whitelist of verified tokens
- Reject unverified tokens by default

### 8.2 Approval Security
- Never approve unlimited amounts (type(uint256).max)
- Implement approval expiry (optional)
- Track and revoke old approvals
- Monitor for approval front-running attacks

### 8.3 Reentrancy Protection
- Use checks-effects-interactions pattern
- Implement reentrancy guards on critical functions
- Validate token contracts don't have malicious callbacks
- Test with known attack vectors

### 8.4 Gas Limit Protection
- Set reasonable gas limits to prevent griefing
- Implement gas price caps
- Monitor for gas price manipulation
- Fail gracefully on out-of-gas errors

## 9. Monitoring & Metrics

### 9.1 Prometheus Metrics

```rust
// Token operation metrics
pub token_transfers_total: IntCounterVec,  // {token, network, status}
pub token_transfer_duration_seconds: HistogramVec,  // {token, network}
pub token_approvals_total: IntCounterVec,  // {token, network}
pub token_approval_reuse_rate: GaugeVec,  // {token, network}

// Balance metrics
pub token_balance_checks_total: IntCounter,
pub token_insufficient_balance_total: IntCounterVec,  // {token}

// Gas metrics
pub token_gas_used: HistogramVec,  // {token, operation}
pub token_gas_cost_usd: CounterVec,  // {token, network}

// Registry metrics
pub token_registry_size: Gauge,
pub token_registry_cache_hits: Counter,
pub token_registry_cache_misses: Counter,
```

### 9.2 Alert Rules

```yaml
# Critical Alerts
- alert: HighTokenTransferFailureRate
  expr: rate(token_transfers_total{status="failed"}[5m]) / rate(token_transfers_total[5m]) > 0.05
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "Token transfer failure rate above 5%"

- alert: TokenApprovalFailures
  expr: rate(token_approvals_total{status="failed"}[5m]) > 10
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "Multiple token approval failures detected"

# Warning Alerts
- alert: LowApprovalReuseRate
  expr: token_approval_reuse_rate < 0.3
  for: 15m
  labels:
    severity: warning
  annotations:
    summary: "Approval reuse rate below 30%, optimization not working"

- alert: HighTokenGasCost
  expr: rate(token_gas_cost_usd[1h]) > 1000
  for: 1h
  labels:
    severity: warning
  annotations:
    summary: "Token gas costs exceeding $1000/hour"
```

## 10. Performance Optimization

### 10.1 Approval Optimization
- Batch approvals for multiple swaps
- Approve 2x required amount to reduce approval frequency
- Cache approval status in database
- Target: 50% reduction in approval transactions

### 10.2 Gas Optimization
- Use gas estimation before submission
- Implement EIP-1559 for dynamic gas pricing
- Batch multiple transfers when possible
- Monitor gas prices and delay non-urgent transfers

### 10.3 Registry Caching
- Cache token metadata in memory (RwLock)
- TTL: 1 hour for metadata
- Invalidate cache on token updates
- Target: 95%+ cache hit rate

### 10.4 Database Optimization
- Index on (contract_address, network)
- Index on (user_address, token_address) for approvals
- Partition token_transfers by month
- Archive old transfers to cold storage

## 11. Migration Path

### 11.1 Existing Currency Table
Current `currencies` table already has `contract_address` field:
```sql
contract_address VARCHAR(100),
```

**Migration Strategy**:
1. Keep existing `currencies` table for backward compatibility
2. Create new `tokens` table with enhanced fields
3. Migrate existing ERC-20 entries from `currencies` to `tokens`
4. Add foreign key relationship: `currencies.id` → `tokens.id`
5. Gradually deprecate `currencies` table

### 11.2 Data Migration Script
```sql
-- Migrate existing ERC-20 tokens
INSERT INTO tokens (symbol, name, network, contract_address, decimals, token_type, is_active, is_verified)
SELECT 
    symbol,
    name,
    network,
    contract_address,
    decimals,
    'ERC20' as token_type,
    is_active,
    TRUE as is_verified
FROM currencies
WHERE contract_address IS NOT NULL
  AND network IN ('ethereum', 'bsc', 'polygon');
```

## 12. Success Criteria

- ✅ Support 100+ ERC-20 tokens across 5+ chains
- ✅ < 200ms token transfer submission time
- ✅ 99.9% successful token transfers
- ✅ 50% reduction in approval transactions (via optimization)
- ✅ Gas estimates accurate within 20%
- ✅ 95%+ registry cache hit rate
- ✅ Comprehensive test coverage (>90%)
- ✅ Prometheus metrics exported
- ✅ Complete documentation

## 13. References

Content rephrased for compliance with licensing restrictions:

- [Alloy-rs Documentation](https://alloy.rs/) - Rust Ethereum library
- [ERC-20 Token Standard](https://eips.ethereum.org/EIPS/eip-20) - Official specification
- [OpenZeppelin ERC-20](https://docs.openzeppelin.com/contracts/erc20) - Implementation guide
- Industry practices from Uniswap, 1inch, and other DEX platforms
- Existing codebase patterns: wallet manager, RPC management, gas estimation

