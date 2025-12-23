use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// =============================================================================
// PROVIDERS
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ProvidersQuery {
    #[serde(default)]
    pub include_inactive: bool,
    pub kyc_required: Option<bool>,
    pub rate_type: Option<String>,
    pub sort: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProviderResponse {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub kyc_required: bool,
    pub rating: f32,
    pub supports_fixed_rate: bool,
    pub supports_floating_rate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProviderDetailResponse {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub kyc_required: bool,
    pub rating: f32,
    pub supports_fixed_rate: bool,
    pub supports_floating_rate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    pub supported_currencies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<f64>,
    pub fee_percentage: f64,
}

// =============================================================================
// CURRENCIES
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CurrenciesQuery {
    pub network: Option<String>,
    pub search: Option<String>,
    #[serde(default)]
    pub include_inactive: bool,
}

#[derive(Debug, Serialize)]
pub struct CurrencyResponse {
    pub symbol: String,
    pub name: String,
    pub network: String,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_address: Option<String>,
    pub requires_extra_id: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_id_name: Option<String>,
}

// =============================================================================
// PAIRS
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct PairsQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub from_network: Option<String>,
    pub to_network: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PairResponse {
    pub from: String,
    pub to: String,
    pub from_network: String,
    pub to_network: String,
    pub is_active: bool,
}

// =============================================================================
// RATES
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct RatesQuery {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub rate_type: Option<RateType>,
    pub provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RateType {
    Fixed,
    Floating,
}

impl Default for RateType {
    fn default() -> Self {
        RateType::Floating
    }
}

#[derive(Debug, Serialize)]
pub struct RateResponse {
    pub provider: String,
    pub provider_name: String,
    pub rate: f64,
    pub estimated_amount: f64,
    pub min_amount: f64,
    pub max_amount: f64,
    pub network_fee: f64,
    pub provider_fee: f64,
    pub platform_fee: f64,
    pub total_fee: f64,
    pub rate_type: RateType,
    pub kyc_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_minutes: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct RatesResponse {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub rates: Vec<RateResponse>,
}

// =============================================================================
// ESTIMATE
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct EstimateRequest {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub provider: String,
    #[serde(default)]
    pub rate_type: RateType,
}

#[derive(Debug, Serialize)]
pub struct EstimateResponse {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub provider: String,
    pub rate: f64,
    pub estimated_amount: f64,
    pub min_amount: f64,
    pub max_amount: f64,
    pub network_fee: f64,
    pub total_fee: f64,
    pub rate_type: RateType,
    pub valid_until: DateTime<Utc>,
}

// =============================================================================
// CREATE SWAP
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateSwapRequest {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub provider: String,
    pub recipient_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_extra_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_extra_id: Option<String>,
    #[serde(default)]
    pub rate_type: RateType,
    #[serde(default)]
    pub sandbox: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateSwapResponse {
    pub swap_id: String,
    pub provider: String,
    pub from: String,
    pub to: String,
    pub deposit_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_extra_id: Option<String>,
    pub deposit_amount: f64,
    pub recipient_address: String,
    pub estimated_receive: f64,
    pub rate: f64,
    pub status: SwapStatus,
    pub rate_type: RateType,
    pub is_sandbox: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// SWAP STATUS
// =============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum SwapStatus {
    Waiting,
    Confirming,
    Exchanging,
    Sending,
    Completed,
    Failed,
    Refunded,
    Expired,
}

impl Default for SwapStatus {
    fn default() -> Self {
        SwapStatus::Waiting
    }
}

#[derive(Debug, Serialize)]
pub struct SwapStatusResponse {
    pub swap_id: String,
    pub provider: String,
    pub provider_swap_id: Option<String>,
    pub status: SwapStatus,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub deposit_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_extra_id: Option<String>,
    pub recipient_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_extra_id: Option<String>,
    pub rate: f64,
    pub estimated_receive: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_receive: Option<f64>,
    pub network_fee: f64,
    pub total_fee: f64,
    pub rate_type: RateType,
    pub is_sandbox: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash_in: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash_out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

// =============================================================================
// SWAP HISTORY
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
    pub status: Option<SwapStatus>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    #[serde(default)]
    pub sandbox: Option<bool>,
}

fn default_page() -> u32 { 1 }
fn default_limit() -> u32 { 20 }

#[derive(Debug, Serialize)]
pub struct SwapSummary {
    pub swap_id: String,
    pub provider: String,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub estimated_receive: f64,
    pub status: SwapStatus,
    pub rate_type: RateType,
    pub is_sandbox: bool,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub swaps: Vec<SwapSummary>,
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
}

// =============================================================================
// ERROR RESPONSE
// =============================================================================

#[derive(Debug, Serialize)]
pub struct SwapErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<f64>,
}

impl SwapErrorResponse {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: None,
            min_amount: None,
            max_amount: None,
        }
    }

    pub fn with_code(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: Some(code.into()),
            min_amount: None,
            max_amount: None,
        }
    }

    pub fn with_limits(error: impl Into<String>, min: f64, max: f64) -> Self {
        Self {
            error: error.into(),
            code: None,
            min_amount: Some(min),
            max_amount: Some(max),
        }
    }
}
