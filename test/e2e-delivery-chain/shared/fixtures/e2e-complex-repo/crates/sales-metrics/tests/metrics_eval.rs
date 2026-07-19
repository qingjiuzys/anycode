use sales_metrics::{north_anomaly, refund_rate_pct, region_sales_sum, region_totals_by_sales, total_sales, SalesRow};

fn fixture_rows() -> Vec<SalesRow> {
    vec![
        SalesRow { region: "华东".into(), sales: 120300, orders: 310, refunds: 8 },
        SalesRow { region: "华南".into(), sales: 98200, orders: 260, refunds: 5 },
        SalesRow { region: "华北".into(), sales: 76000, orders: 190, refunds: 9 },
        SalesRow { region: "华东".into(), sales: 132500, orders: 335, refunds: 7 },
        SalesRow { region: "华南".into(), sales: 100800, orders: 271, refunds: 6 },
        SalesRow { region: "华北".into(), sales: 70200, orders: 181, refunds: 12 },
        SalesRow { region: "华东".into(), sales: 141000, orders: 352, refunds: 8 },
        SalesRow { region: "华南".into(), sales: 105400, orders: 280, refunds: 4 },
        SalesRow { region: "华北".into(), sales: 65500, orders: 170, refunds: 15 },
    ]
}

#[test]
fn total_sales_matches_june_fixture() {
    assert_eq!(total_sales(&fixture_rows()), 909_900);
}

#[test]
fn north_refund_rate_is_anomaly() {
    let north_orders = 541u32;
    let north_refunds = 36u32;
    let rate = refund_rate_pct(north_refunds, north_orders);
    assert!((rate - 6.65).abs() < 0.1, "rate was {rate}");
    assert!(north_anomaly(rate));
}

#[test]
fn east_region_sales_sum_matches_fixture() {
    assert_eq!(region_sales_sum(&fixture_rows(), "华东"), 393_800);
}

#[test]
fn top_region_is_east_by_sales() {
    let ranked = region_totals_by_sales(&fixture_rows());
    assert_eq!(ranked[0].0, "华东");
    assert_eq!(ranked[0].1, 393_800);
}
