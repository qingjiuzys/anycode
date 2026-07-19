use sales_metrics::{region_totals, SalesRow};

fn main() {
    let rows = vec![SalesRow {
        region: "demo".into(),
        sales: 1,
        orders: 1,
        refunds: 0,
    }];
    let _ = region_totals(&rows);
    println!("sales-report ok");
}
