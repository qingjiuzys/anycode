use sales_metrics::{region_totals_by_sales, total_sales, SalesRow};

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
fn report_pipeline_totals() {
    let rows = fixture_rows();
    assert_eq!(total_sales(&rows), 909_900);
    let top = &region_totals_by_sales(&rows)[0];
    assert_eq!(top.0, "华东");
    assert_eq!(top.1, 393_800);
}
