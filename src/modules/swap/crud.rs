use chrono::Utc;
use sqlx::{MySql, Pool};

use super::model::{Currency, Provider};
use super::schema::{CurrenciesQuery, ProvidersQuery, TrocadorCurrency, TrocadorProvider};
use crate::services::trocador::{TrocadorClient, TrocadorError};

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

impl From<TrocadorError> for SwapError {
    fn from(err: TrocadorError) -> Self {
        SwapError::ExternalApiError(err.to_string())
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
    // CURRENCIES
    // =========================================================================

    /// Check if currencies cache needs refresh (>5 minutes old)
    pub async fn should_sync_currencies(&self) -> Result<bool, SwapError> {
        let result: Option<(Option<chrono::DateTime<Utc>>,)> = sqlx::query_as(
            "SELECT MAX(last_synced_at) FROM currencies"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SwapError::DatabaseError(e.to_string()))?;

        match result {
            Some((Some(last_sync),)) => {
                let cache_age = Utc::now() - last_sync;
                // Refresh if older than 5 minutes
                Ok(cache_age.num_minutes() > 5)
            }
            _ => Ok(true), // No sync found, need to sync
        }
    }

    /// Sync currencies from Trocador API and upsert into database
    pub async fn sync_currencies_from_trocador(
        &self,
        trocador_client: &TrocadorClient,
    ) -> Result<usize, SwapError> {
        // Fetch from Trocador API
        let trocador_currencies = trocador_client.get_currencies().await?;

        let mut synced_count = 0;

        // Upsert each currency
        for trocador_currency in trocador_currencies {
            self.upsert_currency_from_trocador(&trocador_currency).await?;
            synced_count += 1;
        }

        Ok(synced_count)
    }

    /// Upsert a single currency from Trocador data
    async fn upsert_currency_from_trocador(
        &self,
        trocador_currency: &TrocadorCurrency,
    ) -> Result<(), SwapError> {
        sqlx::query(
            r#"
            INSERT INTO currencies (
                symbol, name, network, is_active, logo_url,
                requires_extra_id, min_amount, max_amount, last_synced_at
            )
            VALUES (?, ?, ?, TRUE, ?, ?, ?, ?, NOW())
            ON DUPLICATE KEY UPDATE
                name = VALUES(name),
                logo_url = VALUES(logo_url),
                min_amount = VALUES(min_amount),
                max_amount = VALUES(max_amount),
                last_synced_at = NOW()
            "#
        )
        .bind(&trocador_currency.ticker)
        .bind(&trocador_currency.name)
        .bind(&trocador_currency.network)
        .bind(&trocador_currency.image)
        .bind(trocador_currency.memo)
        .bind(trocador_currency.minimum)
        .bind(trocador_currency.maximum)
        .execute(&self.pool)
        .await
        .map_err(|e| SwapError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get currencies from database with optional filtering
    pub async fn get_currencies(
        &self,
        query: CurrenciesQuery,
    ) -> Result<Vec<Currency>, SwapError> {
        let mut sql = String::from(
            "SELECT id, symbol, name, network, is_active, logo_url, contract_address, 
             decimals, requires_extra_id, extra_id_name, min_amount, max_amount, 
             last_synced_at, created_at, updated_at 
             FROM currencies 
             WHERE is_active = TRUE"
        );

        // Build query based on filters
        let mut sql_parts = Vec::new();

        if let Some(ref ticker) = query.ticker {
            sql_parts.push(format!("LOWER(symbol) = LOWER('{}')", ticker.replace("'", "''")));
        }

        if let Some(ref network) = query.network {
            sql_parts.push(format!("network = '{}'", network.replace("'", "''")));
        }

        if let Some(memo) = query.memo {
            sql_parts.push(format!("requires_extra_id = {}", memo));
        }

        if !sql_parts.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&sql_parts.join(" AND "));
        }

        sql.push_str(" ORDER BY symbol, network");

        let currencies = sqlx::query_as::<_, Currency>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SwapError::DatabaseError(e.to_string()))?;

        Ok(currencies)
    }

    // =========================================================================
    // PROVIDERS
    // =========================================================================

    /// Check if providers cache needs refresh (>5 minutes old)
    pub async fn should_sync_providers(&self) -> Result<bool, SwapError> {
        let result: Option<(Option<chrono::DateTime<Utc>>,)> = sqlx::query_as(
            "SELECT MAX(last_synced_at) FROM providers"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SwapError::DatabaseError(e.to_string()))?;

        match result {
            Some((Some(last_sync),)) => {
                let cache_age = Utc::now() - last_sync;
                Ok(cache_age.num_minutes() > 5)
            }
            _ => Ok(true),
        }
    }

    /// Sync providers from Trocador API and upsert into database
    pub async fn sync_providers_from_trocador(
        &self,
        trocador_client: &TrocadorClient,
    ) -> Result<usize, SwapError> {
        let trocador_providers = trocador_client.get_providers().await?;

        let mut synced_count = 0;

        for trocador_provider in trocador_providers {
            self.upsert_provider_from_trocador(&trocador_provider).await?;
            synced_count += 1;
        }

        Ok(synced_count)
    }

    /// Upsert a single provider from Trocador data
    async fn upsert_provider_from_trocador(
        &self,
        trocador_provider: &TrocadorProvider,
    ) -> Result<(), SwapError> {
        // Generate slug from name
        let slug = trocador_provider.name.to_lowercase().replace(" ", "-");
        let id = slug.clone();

        // First, try to find existing provider by name (case-insensitive)
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM providers WHERE LOWER(name) = LOWER(?) LIMIT 1"
        )
        .bind(&trocador_provider.name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SwapError::DatabaseError(e.to_string()))?;

        if let Some((existing_id,)) = existing {
            // Update existing provider
            sqlx::query(
                r#"
                UPDATE providers SET
                    name = ?,
                    slug = ?,
                    kyc_rating = ?,
                    insurance_percentage = ?,
                    eta_minutes = ?,
                    markup_enabled = ?,
                    last_synced_at = NOW()
                WHERE id = ?
                "#
            )
            .bind(&trocador_provider.name)
            .bind(&slug)
            .bind(&trocador_provider.rating)
            .bind(trocador_provider.insurance)
            .bind(trocador_provider.eta as i32)  // Convert f64 to i32
            .bind(trocador_provider.enabled_markup)
            .bind(&existing_id)
            .execute(&self.pool)
            .await
            .map_err(|e| SwapError::DatabaseError(e.to_string()))?;
        } else {
            // Insert new provider
            sqlx::query(
                r#"
                INSERT INTO providers (
                    id, name, slug, is_active, kyc_rating,
                    insurance_percentage, eta_minutes, markup_enabled, last_synced_at
                )
                VALUES (?, ?, ?, TRUE, ?, ?, ?, ?, NOW())
                "#
            )
            .bind(&id)
            .bind(&trocador_provider.name)
            .bind(&slug)
            .bind(&trocador_provider.rating)
            .bind(trocador_provider.insurance)
            .bind(trocador_provider.eta as i32)  // Convert f64 to i32
            .bind(trocador_provider.enabled_markup)
            .execute(&self.pool)
            .await
            .map_err(|e| SwapError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Get providers from database with optional filtering
    pub async fn get_providers(
        &self,
        query: ProvidersQuery,
    ) -> Result<Vec<Provider>, SwapError> {
        let mut sql = String::from(
            "SELECT id, name, slug, is_active, kyc_rating, insurance_percentage,
             eta_minutes, markup_enabled, api_url, logo_url, website_url,
             last_synced_at, created_at, updated_at
             FROM providers
             WHERE is_active = TRUE"
        );

        let mut sql_parts = Vec::new();

        if let Some(ref rating) = query.rating {
            sql_parts.push(format!("kyc_rating = '{}'", rating.replace("'", "''")));
        }

        if let Some(markup_enabled) = query.markup_enabled {
            sql_parts.push(format!("markup_enabled = {}", markup_enabled));
        }

        if !sql_parts.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&sql_parts.join(" AND "));
        }

        // Apply sorting
        match query.sort.as_deref() {
            Some("name") => sql.push_str(" ORDER BY name ASC"),
            Some("rating") => sql.push_str(" ORDER BY kyc_rating ASC, name ASC"),
            Some("eta") => sql.push_str(" ORDER BY eta_minutes ASC"),
            _ => sql.push_str(" ORDER BY name ASC"), // Default
        }

        let providers = sqlx::query_as::<_, Provider>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SwapError::DatabaseError(e.to_string()))?;

        Ok(providers)
    }
}
