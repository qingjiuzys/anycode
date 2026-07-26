CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE organizations (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE organizations IS 'Customer organizations that own users and billing records.';

CREATE TABLE users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id uuid NOT NULL,
    email text NOT NULL,
    display_name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT users_organization_fk
        FOREIGN KEY (organization_id)
        REFERENCES organizations (id)
        ON DELETE CASCADE,
    CONSTRAINT users_organization_email_key
        UNIQUE (organization_id, email)
);

COMMENT ON TABLE users IS 'Application users, each belonging to exactly one organization.';

CREATE TABLE plans (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code text NOT NULL UNIQUE,
    name text NOT NULL,
    currency char(3) NOT NULL,
    monthly_price_cents bigint NOT NULL CHECK (monthly_price_cents >= 0),
    is_active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE plans IS 'Billable subscription plans with prices stored in integer cents.';

CREATE TABLE subscriptions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id uuid NOT NULL,
    plan_id uuid NOT NULL,
    status text NOT NULL CHECK (status IN ('trialing', 'active', 'past_due', 'canceled')),
    current_period_start timestamptz NOT NULL,
    current_period_end timestamptz NOT NULL,
    canceled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT subscriptions_organization_fk
        FOREIGN KEY (organization_id)
        REFERENCES organizations (id)
        ON DELETE RESTRICT,
    CONSTRAINT subscriptions_plan_fk
        FOREIGN KEY (plan_id)
        REFERENCES plans (id)
        ON DELETE RESTRICT,
    CONSTRAINT subscriptions_period_check
        CHECK (current_period_end > current_period_start)
);

COMMENT ON TABLE subscriptions IS 'Organization plan subscriptions and their current billing periods.';

CREATE TABLE invoices (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id uuid NOT NULL,
    subscription_id uuid NOT NULL,
    invoice_number text NOT NULL UNIQUE,
    status text NOT NULL CHECK (status IN ('draft', 'open', 'paid', 'void', 'uncollectible')),
    currency char(3) NOT NULL,
    subtotal_cents bigint NOT NULL CHECK (subtotal_cents >= 0),
    tax_cents bigint NOT NULL CHECK (tax_cents >= 0),
    total_cents bigint NOT NULL CHECK (total_cents >= 0),
    amount_due_cents bigint NOT NULL CHECK (amount_due_cents >= 0),
    amount_paid_cents bigint NOT NULL DEFAULT 0 CHECK (amount_paid_cents >= 0),
    issued_at timestamptz NOT NULL,
    due_at timestamptz NOT NULL,
    paid_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT invoices_organization_fk
        FOREIGN KEY (organization_id)
        REFERENCES organizations (id)
        ON DELETE RESTRICT,
    CONSTRAINT invoices_subscription_fk
        FOREIGN KEY (subscription_id)
        REFERENCES subscriptions (id)
        ON DELETE RESTRICT,
    CONSTRAINT invoices_totals_check
        CHECK (total_cents = subtotal_cents + tax_cents),
    CONSTRAINT invoices_due_date_check
        CHECK (due_at >= issued_at)
);

COMMENT ON TABLE invoices IS 'Immutable billing statements with monetary amounts stored in integer cents.';

CREATE INDEX users_organization_idx
    ON users (organization_id);

CREATE INDEX subscriptions_organization_status_idx
    ON subscriptions (organization_id, status);

CREATE INDEX subscriptions_plan_idx
    ON subscriptions (plan_id);

CREATE UNIQUE INDEX subscriptions_one_current_per_org_idx
    ON subscriptions (organization_id)
    WHERE status IN ('trialing', 'active', 'past_due');

CREATE INDEX invoices_organization_issued_idx
    ON invoices (organization_id, issued_at DESC);

CREATE INDEX invoices_subscription_idx
    ON invoices (subscription_id);

CREATE INDEX invoices_status_due_idx
    ON invoices (status, due_at)
    WHERE status = 'open';
