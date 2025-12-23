use chrono::{DateTime, Utc};
use sqlx::FromRow;
use serde::{Deserialize, Serialize};

use super::schema::{RateType, SwapStatus};

// =============================================================================
// PROVIDER
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub is_active: bool,
    pub kyc_required: bool,
    pub rating: f32,
    pub supports_fixed_rate: bool,
    pub supports_floating_rate: bool,
    pub api_url: Option<String>,
    pub logo_url: Option<String>,
    pub website_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// CURRENCY
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Currency {
    pub id: i64,
    pub symbol: String,
    pub name: String,
    pub network: String,
    pub is_active: bool,
    pub logo_url: Option<String>,
    pub contract_address: Option<String>,
    pub decimals: i32,
    pub requires_extra_id: bool,
    pub extra_id_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// TRADING PAIR
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TradingPair {
    pub id: i64,
    pub from_currency_id: i64,
    pub to_currency_id: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Joined view of trading pair with currency details
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TradingPairView {
    pub id: i64,
    pub from_symbol: String,
    pub from_network: String,
    pub to_symbol: String,
    pub to_network: String,
    pub is_active: bool,
}

// =============================================================================
// SWAP
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Swap {
    pub id: String,
    pub user_id: Option<String>,
    pub provider_id: String,
    pub provider_swap_id: Option<String>,

    // Currencies
    pub from_currency: String,
    pub from_network: String,
    pub to_currency: String,
    pub to_network: String,

    // Amounts
    pub amount: f64,
    pub estimated_receive: f64,
    pub actual_receive: Option<f64>,
    pub rate: f64,

    // Fees
    pub network_fee: f64,
    pub provider_fee: f64,
    pub platform_fee: f64,
    pub total_fee: f64,

    // Addresses
    pub deposit_address: String,
    pub deposit_extra_id: Option<String>,
    pub recipient_address: String,
    pub recipient_extra_id: Option<String>,
    pub refund_address: Option<String>,
    pub refund_extra_id: Option<String>,

    // Transaction hashes
    pub tx_hash_in: Option<String>,
    pub tx_hash_out: Option<String>,

    // Status
    pub status: SwapStatus,
    pub rate_type: RateType,
    pub is_sandbox: bool,
    pub error: Option<String>,

    // Timestamps
    pub expires_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// SWAP STATUS HISTORY
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SwapStatusHistory {
    pub id: i64,
    pub swap_id: String,
    pub status: SwapStatus,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// PROVIDER CURRENCY (which currencies each provider supports)
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProviderCurrency {
    pub id: i64,
    pub provider_id: String,
    pub currency_id: i64,
    pub is_active: bool,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// RATE CACHE (optional - for caching provider rates)
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RateCache {
    pub id: i64,
    pub provider_id: String,
    pub from_currency: String,
    pub to_currency: String,
    pub amount: f64,
    pub rate: f64,
    pub estimated_amount: f64,
    pub min_amount: f64,
    pub max_amount: f64,
    pub network_fee: f64,
    pub rate_type: RateType,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
