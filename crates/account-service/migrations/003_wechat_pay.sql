-- Prepaid payment orders (WeChat Pay Native / future providers)
CREATE TABLE IF NOT EXISTS payment_orders (
  id TEXT PRIMARY KEY,
  organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  provider TEXT NOT NULL,
  plan TEXT NOT NULL,
  billing_cycle TEXT NOT NULL DEFAULT 'monthly',
  amount_cents INT NOT NULL,
  currency TEXT NOT NULL DEFAULT 'CNY',
  status TEXT NOT NULL DEFAULT 'pending',
  out_trade_no TEXT NOT NULL UNIQUE,
  provider_trade_no TEXT,
  code_url TEXT,
  expires_at TIMESTAMPTZ NOT NULL,
  paid_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_payment_orders_org ON payment_orders(organization_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_payment_orders_status ON payment_orders(status, expires_at);

ALTER TABLE subscriptions
  ADD COLUMN IF NOT EXISTS payment_provider TEXT,
  ADD COLUMN IF NOT EXISTS prepaid_until DATE;

ALTER TABLE invoices
  ADD COLUMN IF NOT EXISTS payment_order_id TEXT REFERENCES payment_orders(id),
  ADD COLUMN IF NOT EXISTS amount_cny NUMERIC(10, 2);
