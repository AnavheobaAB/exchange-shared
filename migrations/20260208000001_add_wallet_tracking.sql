-- Add address_index to swaps table
ALTER TABLE swaps ADD COLUMN address_index INT UNSIGNED DEFAULT NULL AFTER deposit_extra_id;

-- Create swap_address_info table for more detailed tracking
CREATE TABLE IF NOT EXISTS swap_address_info (
    swap_id VARCHAR(36) PRIMARY KEY,
    our_address VARCHAR(255) NOT NULL,
    address_index INT UNSIGNED NOT NULL,
    blockchain_id INT NOT NULL,
    coin_type INT NOT NULL,
    recipient_address VARCHAR(255) NOT NULL,
    recipient_extra_id VARCHAR(100),
    commission_rate DOUBLE NOT NULL DEFAULT 0.0,
    payout_tx_hash VARCHAR(255),
    payout_amount DOUBLE,
    status ENUM('pending', 'success', 'failed') NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    signed_at TIMESTAMP NULL,
    broadcast_at TIMESTAMP NULL,
    confirmed_at TIMESTAMP NULL,
    FOREIGN KEY (swap_id) REFERENCES swaps(id) ON DELETE CASCADE,
    INDEX idx_swap_address_index (address_index)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
