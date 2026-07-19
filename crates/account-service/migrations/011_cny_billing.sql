-- CNY is the billing source of truth. Legacy USD columns remain read-only for rollback.
ALTER TABLE payment_orders ADD COLUMN amount_fen INT NULL AFTER billing_cycle;

UPDATE payment_orders
SET amount_fen = amount_cents
WHERE amount_fen IS NULL;

ALTER TABLE invoices ADD COLUMN amount_fen INT NULL AFTER period_end;
ALTER TABLE invoices ADD COLUMN currency VARCHAR(8) NOT NULL DEFAULT 'CNY' AFTER amount_fen;

UPDATE invoices
SET amount_fen = ROUND(COALESCE(amount_cny, 0) * 100)
WHERE amount_fen IS NULL;

ALTER TABLE cloud_models
  ADD COLUMN price_per_1m_input_cny DECIMAL(12,4) NULL AFTER price_per_1m_input;
ALTER TABLE cloud_models
  ADD COLUMN price_per_1m_output_cny DECIMAL(12,4) NULL AFTER price_per_1m_output;
ALTER TABLE cloud_models
  ADD COLUMN currency VARCHAR(8) NOT NULL DEFAULT 'CNY' AFTER price_per_1m_output_cny;

UPDATE cloud_models
SET price_per_1m_input_cny = CASE id WHEN 'agnes-chat' THEN 3.6000 ELSE 0.0000 END,
    price_per_1m_output_cny = CASE id WHEN 'agnes-chat' THEN 10.8000 ELSE 0.0000 END
WHERE price_per_1m_input_cny IS NULL OR price_per_1m_output_cny IS NULL;
