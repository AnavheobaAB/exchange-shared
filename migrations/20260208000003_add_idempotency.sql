-- Add payout_idempotency_key to prevent double spending
ALTER TABLE swap_address_info ADD COLUMN payout_idempotency_key VARCHAR(100) UNIQUE DEFAULT NULL AFTER payout_tx_hash;
