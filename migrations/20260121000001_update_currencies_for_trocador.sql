-- ============================================================================
-- Migration: Update currencies table for Trocador integration
-- Created: 2026-01-21 (REVISED)
-- Description: Add min/max amounts and cache tracking to currencies table
-- ============================================================================

-- Add global min/max amounts (from Trocador /coins endpoint)
-- Using DOUBLE instead of DECIMAL for f64 compatibility
ALTER TABLE currencies 
ADD COLUMN min_amount DOUBLE DEFAULT NULL AFTER requires_extra_id,
ADD COLUMN max_amount DOUBLE DEFAULT NULL AFTER min_amount,
ADD COLUMN last_synced_at TIMESTAMP NULL AFTER max_amount;

-- Add index for cache freshness checks
CREATE INDEX idx_currencies_last_synced ON currencies(last_synced_at);

-- Remove min/max from provider_currencies (Trocador uses global limits)
ALTER TABLE provider_currencies
DROP COLUMN min_amount,
DROP COLUMN max_amount;

-- Update existing seed data with placeholder values (will be synced from Trocador)
UPDATE currencies SET last_synced_at = NULL WHERE last_synced_at IS NULL;
