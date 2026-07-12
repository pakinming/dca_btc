use serde_json::Value;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use tracing::info;
use std::env;
use std::error::Error;

pub async fn init_pool() -> Result<PgPool, Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run Migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("✅ DB Connection Success");

    Ok(pool)
}

pub async fn save_trade(
    pool: &PgPool,
    symbol: &str,
    amount: i32,
    rat: i32,
    type_trade: &str,
    response: &Value,
) -> Result<i32, sqlx::Error> {
    // ใช้ query function แบบ runtime เพื่อป้องกัน macro compilation error ถ้าไม่มี DB จริงขณะ build
    // และเพื่อให้ยืดหยุ่นกว่า
    let row = sqlx::query(
        "INSERT INTO trades (symbol, amount_thb, rat, type, response_json, status) VALUES ($1, $2, $3, $4, $5, 'open') RETURNING id"
    )
    .bind(symbol)
    .bind(amount)
    .bind(rat)
    .bind(type_trade)
    .bind(response)
    .fetch_one(pool)
    .await?;

    let id: i32 = row.try_get("id")?;
    Ok(id)
}

pub async fn update_trade_receive(
    pool: &PgPool,
    id: i32,
    receive_amount: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE trades SET receive_amount = $1, status = 'filled' WHERE id = $2")
        .bind(receive_amount)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_trade_status(
    pool: &PgPool,
    id: i32,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE trades SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_trade_status_by_order_id(
    pool: &PgPool,
    order_id: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE trades SET status = $1 WHERE response_json->>'id' = $2")
        .bind(status)
        .bind(order_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_latest_trade(pool: &PgPool) -> Result<Option<crate::models::Trade>, sqlx::Error> {
    let row = sqlx::query_as::<_, crate::models::Trade>(
        "SELECT * FROM trades WHERE response_json->>'error' IS NULL ORDER BY id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_trade(
    pool: &PgPool,
    id: i32,
) -> Result<Option<crate::models::Trade>, sqlx::Error> {
    let row = sqlx::query_as::<_, crate::models::Trade>("SELECT * FROM trades WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn get_trade_by_order_id(
    pool: &PgPool,
    order_id: &str,
) -> Result<Option<crate::models::Trade>, sqlx::Error> {
    let row = sqlx::query_as::<_, crate::models::Trade>(
        "SELECT * FROM trades WHERE response_json->>'id' = $1"
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[derive(Debug)]
pub struct PortfolioStats {
    pub total_capital: f64,
    pub total_btc: f64,
}

pub async fn get_portfolio_stats(pool: &PgPool) -> Result<PortfolioStats, sqlx::Error> {
    let row = sqlx::query(
        "SELECT 
            COALESCE(SUM(amount_thb), 0)::DOUBLE PRECISION as total_capital,
            COALESCE(SUM(receive_amount), 0.0) as total_btc
         FROM trades 
         WHERE status = 'filled' AND response_json->>'txn_id' IS NULL"
    )
    .fetch_one(pool)
    .await?;

    let total_capital: f64 = row.try_get("total_capital")?;
    let total_btc: f64 = row.try_get("total_btc")?;

    Ok(PortfolioStats {
        total_capital,
        total_btc,
    })
}

#[cfg(test)]
mod db_tests {
    use super::*;

    #[tokio::test]
    async fn test_verify_stats() {
        dotenvy::dotenv().ok();
        let pool = init_pool().await.unwrap();

        // 1. Fetch filtered stats
        let stats = get_portfolio_stats(&pool).await.unwrap();
        println!("\n=== VERIFICATION RESULTS (FILTERED BY TXN_ID IS NULL) ===");
        println!("TOTAL CAPITAL SPENT (ขนาดทุน): {} THB", stats.total_capital);
        println!("TOTAL BTC BOUGHT (BTC ที่ซื้อทั้งหมด): {:.8} BTC", stats.total_btc);

        // 2. Fetch unfiltered stats
        let unfiltered_row = sqlx::query(
            "SELECT 
                COALESCE(SUM(amount_thb), 0)::DOUBLE PRECISION as total_capital,
                COALESCE(SUM(receive_amount), 0.0) as total_btc
             FROM trades 
             WHERE status = 'filled'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let unfiltered_capital: f64 = unfiltered_row.try_get("total_capital").unwrap();
        let unfiltered_btc: f64 = unfiltered_row.try_get("total_btc").unwrap();

        println!("\n=== UNFILTERED RESULTS (ALL FILLED ROWS - DOUBLE COUNTED) ===");
        println!("UNFILTERED CAPITAL: {} THB", unfiltered_capital);
        println!("UNFILTERED BTC: {:.8} BTC", unfiltered_btc);

        // 3. Count split rows
        let split_rows_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM trades WHERE status = 'filled' AND response_json->>'txn_id' IS NOT NULL"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        println!("\n=================================");
        println!("NUMBER OF SPLIT MATCH RECORDS: {}", split_rows_count);
        println!("=================================\n");

        assert!(stats.total_capital <= unfiltered_capital);
    }
}




