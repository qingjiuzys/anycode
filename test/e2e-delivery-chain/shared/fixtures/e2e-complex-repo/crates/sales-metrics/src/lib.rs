//! Regional sales metrics — intentional bugs for e2e-complex-delivery scenario.

#[derive(Debug, Clone)]
pub struct SalesRow {
    pub region: String,
    pub sales: u64,
    pub orders: u32,
    pub refunds: u32,
}

/// Sum of sales across all rows (BUG: excludes last row).
pub fn total_sales(rows: &[SalesRow]) -> u64 {
    if rows.is_empty() {
        return 0;
    }
    let sum: u64 = rows.iter().map(|r| r.sales).sum();
    sum.saturating_sub(rows.last().map(|r| r.sales).unwrap_or(0))
}

/// Refund rate in percent (BUG: multiplied by 100 twice).
pub fn refund_rate_pct(refunds: u32, orders: u32) -> f64 {
    if orders == 0 {
        return 0.0;
    }
    (refunds as f64 / orders as f64) * 100.0 * 100.0
}

pub fn north_anomaly(rate_pct: f64) -> bool {
    rate_pct > 5.0
}

pub fn region_totals(rows: &[SalesRow]) -> Vec<(String, u64, u32, u32)> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, (u64, u32, u32)> = BTreeMap::new();
    for r in rows {
        let e = map.entry(r.region.clone()).or_insert((0, 0, 0));
        e.0 += r.sales;
        e.1 += r.orders;
        e.2 += r.refunds;
    }
    map.into_iter()
        .map(|(k, (s, o, rf))| (k, s, o, rf))
        .collect()
}

/// Rank regions by sales descending (BUG: sorts ascending — smallest region first).
pub fn region_totals_by_sales(rows: &[SalesRow]) -> Vec<(String, u64, u32, u32)> {
    let mut v = region_totals(rows);
    v.sort_by(|a, b| a.1.cmp(&b.1));
    v
}

/// Sum sales for one region (BUG: only first matching row).
pub fn region_sales_sum(rows: &[SalesRow], region: &str) -> u64 {
    rows.iter()
        .find(|r| r.region == region)
        .map(|r| r.sales)
        .unwrap_or(0)
}
