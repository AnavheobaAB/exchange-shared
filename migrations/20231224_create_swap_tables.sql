-- =============================================================================
-- PROVIDERS TABLE
-- Exchange providers (ChangeNOW, Changelly, etc.)
-- =============================================================================

CREATE TABLE IF NOT EXISTS providers (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    slug VARCHAR(100) NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    kyc_required BOOLEAN NOT NULL DEFAULT FALSE,
    rating FLOAT NOT NULL DEFAULT 0,
    supports_fixed_rate BOOLEAN NOT NULL DEFAULT TRUE,
    supports_floating_rate BOOLEAN NOT NULL DEFAULT TRUE,
    api_url VARCHAR(255),
    logo_url VARCHAR(255),
    website_url VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    INDEX idx_providers_is_active (is_active),
    INDEX idx_providers_slug (slug)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- CURRENCIES TABLE
-- Supported cryptocurrencies
-- =============================================================================

CREATE TABLE IF NOT EXISTS currencies (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    symbol VARCHAR(20) NOT NULL,
    name VARCHAR(100) NOT NULL,
    network VARCHAR(50) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    logo_url VARCHAR(255),
    contract_address VARCHAR(100),
    decimals INT NOT NULL DEFAULT 8,
    requires_extra_id BOOLEAN NOT NULL DEFAULT FALSE,
    extra_id_name VARCHAR(50),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE KEY uk_currency_symbol_network (symbol, network),
    INDEX idx_currencies_symbol (symbol),
    INDEX idx_currencies_network (network),
    INDEX idx_currencies_is_active (is_active)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- TRADING PAIRS TABLE
-- Available trading pairs
-- =============================================================================

CREATE TABLE IF NOT EXISTS trading_pairs (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    from_currency_id BIGINT NOT NULL,
    to_currency_id BIGINT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE KEY uk_trading_pair (from_currency_id, to_currency_id),
    INDEX idx_pairs_from (from_currency_id),
    INDEX idx_pairs_to (to_currency_id),
    INDEX idx_pairs_is_active (is_active),

    FOREIGN KEY (from_currency_id) REFERENCES currencies(id) ON DELETE CASCADE,
    FOREIGN KEY (to_currency_id) REFERENCES currencies(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- PROVIDER CURRENCIES TABLE
-- Which currencies each provider supports
-- =============================================================================

CREATE TABLE IF NOT EXISTS provider_currencies (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    provider_id VARCHAR(50) NOT NULL,
    currency_id BIGINT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    min_amount DECIMAL(20, 8),
    max_amount DECIMAL(20, 8),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE KEY uk_provider_currency (provider_id, currency_id),
    INDEX idx_provider_currencies_provider (provider_id),
    INDEX idx_provider_currencies_currency (currency_id),

    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE,
    FOREIGN KEY (currency_id) REFERENCES currencies(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- SWAPS TABLE
-- Main swap transactions
-- =============================================================================

CREATE TABLE IF NOT EXISTS swaps (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36),
    provider_id VARCHAR(50) NOT NULL,
    provider_swap_id VARCHAR(100),

    -- Currencies
    from_currency VARCHAR(20) NOT NULL,
    from_network VARCHAR(50) NOT NULL,
    to_currency VARCHAR(20) NOT NULL,
    to_network VARCHAR(50) NOT NULL,

    -- Amounts
    amount DECIMAL(20, 8) NOT NULL,
    estimated_receive DECIMAL(20, 8) NOT NULL,
    actual_receive DECIMAL(20, 8),
    rate DECIMAL(30, 18) NOT NULL,

    -- Fees
    network_fee DECIMAL(20, 8) NOT NULL DEFAULT 0,
    provider_fee DECIMAL(20, 8) NOT NULL DEFAULT 0,
    platform_fee DECIMAL(20, 8) NOT NULL DEFAULT 0,
    total_fee DECIMAL(20, 8) NOT NULL DEFAULT 0,

    -- Addresses
    deposit_address VARCHAR(255) NOT NULL,
    deposit_extra_id VARCHAR(100),
    recipient_address VARCHAR(255) NOT NULL,
    recipient_extra_id VARCHAR(100),
    refund_address VARCHAR(255),
    refund_extra_id VARCHAR(100),

    -- Transaction hashes
    tx_hash_in VARCHAR(100),
    tx_hash_out VARCHAR(100),

    -- Status
    status ENUM('waiting', 'confirming', 'exchanging', 'sending', 'completed', 'failed', 'refunded', 'expired') NOT NULL DEFAULT 'waiting',
    rate_type ENUM('fixed', 'floating') NOT NULL DEFAULT 'floating',
    is_sandbox BOOLEAN NOT NULL DEFAULT FALSE,
    error TEXT,

    -- Timestamps
    expires_at TIMESTAMP NULL,
    completed_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    INDEX idx_swaps_user_id (user_id),
    INDEX idx_swaps_provider_id (provider_id),
    INDEX idx_swaps_provider_swap_id (provider_swap_id),
    INDEX idx_swaps_status (status),
    INDEX idx_swaps_is_sandbox (is_sandbox),
    INDEX idx_swaps_created_at (created_at),
    INDEX idx_swaps_from_currency (from_currency),
    INDEX idx_swaps_to_currency (to_currency),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- SWAP STATUS HISTORY TABLE
-- Track status changes for swaps
-- =============================================================================

CREATE TABLE IF NOT EXISTS swap_status_history (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    swap_id VARCHAR(36) NOT NULL,
    status ENUM('waiting', 'confirming', 'exchanging', 'sending', 'completed', 'failed', 'refunded', 'expired') NOT NULL,
    message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_swap_status_history_swap_id (swap_id),
    INDEX idx_swap_status_history_status (status),
    INDEX idx_swap_status_history_created_at (created_at),

    FOREIGN KEY (swap_id) REFERENCES swaps(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- RATE CACHE TABLE
-- Cache provider rates for performance
-- =============================================================================

CREATE TABLE IF NOT EXISTS rate_cache (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    provider_id VARCHAR(50) NOT NULL,
    from_currency VARCHAR(20) NOT NULL,
    to_currency VARCHAR(20) NOT NULL,
    amount DECIMAL(20, 8) NOT NULL,
    rate DECIMAL(30, 18) NOT NULL,
    estimated_amount DECIMAL(20, 8) NOT NULL,
    min_amount DECIMAL(20, 8) NOT NULL,
    max_amount DECIMAL(20, 8) NOT NULL,
    network_fee DECIMAL(20, 8) NOT NULL,
    rate_type ENUM('fixed', 'floating') NOT NULL DEFAULT 'floating',
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_rate_cache_lookup (provider_id, from_currency, to_currency, rate_type),
    INDEX idx_rate_cache_expires (expires_at),

    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- =============================================================================
-- SEED DATA: Default Providers
-- =============================================================================

INSERT INTO providers (id, name, slug, is_active, kyc_required, rating, supports_fixed_rate, supports_floating_rate, website_url) VALUES
('changenow', 'ChangeNOW', 'changenow', TRUE, FALSE, 4.5, TRUE, TRUE, 'https://changenow.io'),
('changelly', 'Changelly', 'changelly', TRUE, FALSE, 4.3, TRUE, TRUE, 'https://changelly.com'),
('sideshift', 'SideShift', 'sideshift', TRUE, FALSE, 4.4, FALSE, TRUE, 'https://sideshift.ai'),
('exch', 'Exch', 'exch', TRUE, FALSE, 4.2, FALSE, TRUE, 'https://exch.cx'),
('stealthex', 'StealthEX', 'stealthex', TRUE, FALSE, 4.3, TRUE, TRUE, 'https://stealthex.io'),
('godex', 'Godex', 'godex', TRUE, FALSE, 4.1, FALSE, TRUE, 'https://godex.io'),
('letsexchange', 'LetsExchange', 'letsexchange', TRUE, FALSE, 4.2, TRUE, TRUE, 'https://letsexchange.io'),
('simpleswap', 'SimpleSwap', 'simpleswap', TRUE, FALSE, 4.3, TRUE, TRUE, 'https://simpleswap.io')
ON DUPLICATE KEY UPDATE updated_at = CURRENT_TIMESTAMP;

-- =============================================================================
-- SEED DATA: Common Cryptocurrencies
-- =============================================================================

INSERT INTO currencies (symbol, name, network, is_active, decimals, requires_extra_id, extra_id_name) VALUES
-- Bitcoin
('BTC', 'Bitcoin', 'bitcoin', TRUE, 8, FALSE, NULL),
('BTC', 'Bitcoin', 'lightning', TRUE, 8, FALSE, NULL),

-- Ethereum & ERC-20
('ETH', 'Ethereum', 'ethereum', TRUE, 18, FALSE, NULL),
('USDT', 'Tether', 'ethereum', TRUE, 6, FALSE, NULL),
('USDC', 'USD Coin', 'ethereum', TRUE, 6, FALSE, NULL),
('DAI', 'Dai', 'ethereum', TRUE, 18, FALSE, NULL),

-- Tron & TRC-20
('TRX', 'Tron', 'tron', TRUE, 6, FALSE, NULL),
('USDT', 'Tether', 'tron', TRUE, 6, FALSE, NULL),
('USDC', 'USD Coin', 'tron', TRUE, 6, FALSE, NULL),

-- BSC & BEP-20
('BNB', 'BNB', 'bsc', TRUE, 18, FALSE, NULL),
('USDT', 'Tether', 'bsc', TRUE, 18, FALSE, NULL),
('BUSD', 'Binance USD', 'bsc', TRUE, 18, FALSE, NULL),

-- Solana
('SOL', 'Solana', 'solana', TRUE, 9, FALSE, NULL),
('USDT', 'Tether', 'solana', TRUE, 6, FALSE, NULL),
('USDC', 'USD Coin', 'solana', TRUE, 6, FALSE, NULL),

-- Privacy coins
('XMR', 'Monero', 'monero', TRUE, 12, FALSE, NULL),
('ZEC', 'Zcash', 'zcash', TRUE, 8, FALSE, NULL),

-- Other major
('LTC', 'Litecoin', 'litecoin', TRUE, 8, FALSE, NULL),
('DOGE', 'Dogecoin', 'dogecoin', TRUE, 8, FALSE, NULL),
('XRP', 'Ripple', 'ripple', TRUE, 6, TRUE, 'Destination Tag'),
('XLM', 'Stellar', 'stellar', TRUE, 7, TRUE, 'Memo'),
('ADA', 'Cardano', 'cardano', TRUE, 6, FALSE, NULL),
('DOT', 'Polkadot', 'polkadot', TRUE, 10, FALSE, NULL),
('MATIC', 'Polygon', 'polygon', TRUE, 18, FALSE, NULL),
('AVAX', 'Avalanche', 'avalanche', TRUE, 18, FALSE, NULL),
('ATOM', 'Cosmos', 'cosmos', TRUE, 6, TRUE, 'Memo')
ON DUPLICATE KEY UPDATE updated_at = CURRENT_TIMESTAMP;
