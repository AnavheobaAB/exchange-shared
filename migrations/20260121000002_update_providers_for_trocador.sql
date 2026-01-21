-- ============================================================================
-- Migration: Update providers table for Trocador integration
-- Created: 2026-01-21
-- Description: Update providers to match Trocador's /exchanges API structure
-- ============================================================================

-- Drop old columns if they exist (might have been applied manually)
SET @sql = IFNULL((SELECT CONCAT('ALTER TABLE providers DROP COLUMN ', column_name) 
    FROM INFORMATION_SCHEMA.COLUMNS 
    WHERE table_name = 'providers' AND column_name = 'rating' AND table_schema = DATABASE()), 'SELECT 1');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- Add KYC rating column if not exists
SET @sql = IFNULL((SELECT 'SELECT 1' 
    FROM INFORMATION_SCHEMA.COLUMNS 
    WHERE table_name = 'providers' AND column_name = 'kyc_rating' AND table_schema = DATABASE()), 
    'ALTER TABLE providers ADD COLUMN kyc_rating ENUM(''A'', ''B'', ''C'', ''D'') NOT NULL DEFAULT ''C'' AFTER is_active');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- Add other Trocador fields if they don't exist
SET @sql = IFNULL((SELECT 'SELECT 1' FROM INFORMATION_SCHEMA.COLUMNS WHERE table_name = 'providers' AND column_name = 'insurance_percentage' AND table_schema = DATABASE()), 
    'ALTER TABLE providers ADD COLUMN insurance_percentage DOUBLE DEFAULT 0.015 AFTER kyc_rating');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @sql = IFNULL((SELECT 'SELECT 1' FROM INFORMATION_SCHEMA.COLUMNS WHERE table_name = 'providers' AND column_name = 'eta_minutes' AND table_schema = DATABASE()), 
    'ALTER TABLE providers ADD COLUMN eta_minutes INT DEFAULT 10 AFTER insurance_percentage');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @sql = IFNULL((SELECT 'SELECT 1' FROM INFORMATION_SCHEMA.COLUMNS WHERE table_name = 'providers' AND column_name = 'markup_enabled' AND table_schema = DATABASE()), 
    'ALTER TABLE providers ADD COLUMN markup_enabled BOOLEAN DEFAULT FALSE AFTER eta_minutes');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @sql = IFNULL((SELECT 'SELECT 1' FROM INFORMATION_SCHEMA.COLUMNS WHERE table_name = 'providers' AND column_name = 'last_synced_at' AND table_schema = DATABASE()), 
    'ALTER TABLE providers ADD COLUMN last_synced_at TIMESTAMP NULL AFTER markup_enabled');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- Drop old fields if they exist
SET @sql = IFNULL((SELECT CONCAT('ALTER TABLE providers DROP COLUMN ', column_name) 
    FROM INFORMATION_SCHEMA.COLUMNS 
    WHERE table_name = 'providers' AND column_name = 'kyc_required' AND table_schema = DATABASE()), 'SELECT 1');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @sql = IFNULL((SELECT CONCAT('ALTER TABLE providers DROP COLUMN ', column_name) 
    FROM INFORMATION_SCHEMA.COLUMNS 
    WHERE table_name = 'providers' AND column_name = 'supports_fixed_rate' AND table_schema = DATABASE()), 'SELECT 1');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @sql = IFNULL((SELECT CONCAT('ALTER TABLE providers DROP COLUMN ', column_name) 
    FROM INFORMATION_SCHEMA.COLUMNS 
    WHERE table_name = 'providers' AND column_name = 'supports_floating_rate' AND table_schema = DATABASE()), 'SELECT 1');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- Create indexes (ignore if already exists)
SET @sql = IFNULL((SELECT 'SELECT 1' FROM INFORMATION_SCHEMA.STATISTICS 
    WHERE table_name = 'providers' AND index_name = 'idx_providers_last_synced' AND table_schema = DATABASE()), 
    'CREATE INDEX idx_providers_last_synced ON providers(last_synced_at)');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

SET @sql = IFNULL((SELECT 'SELECT 1' FROM INFORMATION_SCHEMA.STATISTICS 
    WHERE table_name = 'providers' AND index_name = 'idx_providers_kyc_rating' AND table_schema = DATABASE()), 
    'CREATE INDEX idx_providers_kyc_rating ON providers(kyc_rating)');
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;
