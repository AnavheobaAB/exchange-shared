-- Create polling_states table for adaptive background monitoring
CREATE TABLE IF NOT EXISTS polling_states (
    swap_id VARCHAR(36) PRIMARY KEY,
    last_polled_at TIMESTAMP NULL,
    next_poll_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    poll_count INT NOT NULL DEFAULT 0,
    last_status VARCHAR(50) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (swap_id) REFERENCES swaps(id) ON DELETE CASCADE,
    INDEX idx_next_poll (next_poll_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
