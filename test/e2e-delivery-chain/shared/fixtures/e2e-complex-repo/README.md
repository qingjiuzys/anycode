# e2e-complex-repo

Open-source-style Rust workspace for **08-complex-delivery** E2E (v2).

Intentional bugs in `sales-metrics`:

- `total_sales` excludes the last row
- `refund_rate_pct` multiplies by 100 twice
- `region_sales_sum` only counts the first row per region
- `region_totals_by_sales` sorts ascending (smallest region first)

Fix all bugs, run `cargo test --workspace`, update `CHANGELOG.md`, commit locally (fix + changelog commits OK).

Data: `data/sales_june.csv` (copy of harness fixture).
