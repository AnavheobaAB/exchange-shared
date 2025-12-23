use sqlx::{MySql, Pool};

use super::model::Provider;
use super::schema::{ProvidersQuery, ProviderResponse, ProviderDetailResponse};

// =============================================================================
// SWAP ERROR
// =============================================================================

#[derive(Debug)]
pub enum SwapError {
    ProviderNotFound,
    CurrencyNotFound,
    PairNotAvailable,
    AmountOutOfRange { min: f64, max: f64 },
    InvalidAddress,
    SwapNotFound,
    ProviderUnavailable(String),
    DatabaseError(String),
    ExternalApiError(String),
}

impl std::fmt::Display for SwapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapError::ProviderNotFound => write!(f, "Provider not found"),
            SwapError::CurrencyNotFound => write!(f, "Currency not found"),
            SwapError::PairNotAvailable => write!(f, "Trading pair not available"),
            SwapError::AmountOutOfRange { min, max } => {
                write!(f, "Amount out of range: min={}, max={}", min, max)
            }
            SwapError::InvalidAddress => write!(f, "Invalid address"),
            SwapError::SwapNotFound => write!(f, "Swap not found"),
            SwapError::ProviderUnavailable(msg) => write!(f, "Provider unavailable: {}", msg),
            SwapError::DatabaseError(e) => write!(f, "Database error: {}", e),
            SwapError::ExternalApiError(e) => write!(f, "External API error: {}", e),
        }
    }
}

// =============================================================================
// SWAP CRUD
// =============================================================================

pub struct SwapCrud {
    pool: Pool<MySql>,
}

impl SwapCrud {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }

    // =========================================================================
    // PROVIDERS
    // =========================================================================

    pub async fn get_providers(&self, query: ProvidersQuery) -> Result<Vec<ProviderResponse>, SwapError> {
        let mut sql = String::from("SELECT * FROM providers WHERE 1=1");

        // Filter by active status
        if !query.include_inactive {
            sql.push_str(" AND is_active = TRUE");
        }

        // Filter by KYC requirement
        if let Some(kyc_required) = query.kyc_required {
            sql.push_str(&format!(" AND kyc_required = {}", kyc_required));
        }

        // Filter by rate type
        if let Some(ref rate_type) = query.rate_type {
            match rate_type.to_lowercase().as_str() {
                "fixed" => sql.push_str(" AND supports_fixed_rate = TRUE"),
                "floating" => sql.push_str(" AND supports_floating_rate = TRUE"),
                _ => {} // Invalid rate type, ignore
            }
        }

        // Sort
        match query.sort.as_deref() {
            Some("name") => sql.push_str(" ORDER BY name ASC"),
            Some("rating") => sql.push_str(" ORDER BY rating DESC"),
            _ => sql.push_str(" ORDER BY rating DESC"), // Default sort by rating
        }

        let providers = sqlx::query_as::<_, Provider>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SwapError::DatabaseError(e.to_string()))?;

        let responses: Vec<ProviderResponse> = providers
            .into_iter()
            .map(|p| ProviderResponse {
                id: p.id,
                name: p.name,
                is_active: p.is_active,
                kyc_required: p.kyc_required,
                rating: p.rating,
                supports_fixed_rate: p.supports_fixed_rate,
                supports_floating_rate: p.supports_floating_rate,
                logo_url: p.logo_url,
                website_url: p.website_url,
            })
            .collect();

        Ok(responses)
    }

    pub async fn get_provider_by_id(&self, id: &str) -> Result<ProviderDetailResponse, SwapError> {
        // Validate input to prevent SQL injection
        if id.contains(['\'', '"', ';', '-', ' ']) {
            return Err(SwapError::ProviderNotFound);
        }

        let provider = sqlx::query_as::<_, Provider>(
            "SELECT * FROM providers WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SwapError::DatabaseError(e.to_string()))?
        .ok_or(SwapError::ProviderNotFound)?;

        // Get supported currencies for this provider
        let currencies: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT c.symbol
            FROM provider_currencies pc
            JOIN currencies c ON pc.currency_id = c.id
            WHERE pc.provider_id = ? AND pc.is_active = TRUE
            "#
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SwapError::DatabaseError(e.to_string()))?;

        let supported_currencies: Vec<String> = currencies.into_iter().map(|(s,)| s).collect();

        // Get min/max amounts from provider_currencies
        let limits: Option<(Option<f64>, Option<f64>)> = sqlx::query_as(
            r#"
            SELECT MIN(min_amount), MAX(max_amount)
            FROM provider_currencies
            WHERE provider_id = ? AND is_active = TRUE
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SwapError::DatabaseError(e.to_string()))?;

        let (min_amount, max_amount) = limits.unwrap_or((None, None));

        Ok(ProviderDetailResponse {
            id: provider.id,
            name: provider.name,
            is_active: provider.is_active,
            kyc_required: provider.kyc_required,
            rating: provider.rating,
            supports_fixed_rate: provider.supports_fixed_rate,
            supports_floating_rate: provider.supports_floating_rate,
            logo_url: provider.logo_url,
            website_url: provider.website_url,
            supported_currencies,
            min_amount,
            max_amount,
            fee_percentage: 0.5, // Default platform fee, could be provider-specific
        })
    }
}
