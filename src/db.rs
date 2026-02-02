use serde_json::Value;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
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
        "INSERT INTO trades (symbol, amount_thb, rat, type, response_json) VALUES ($1, $2, $3, $4, $5) RETURNING id"
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
    sqlx::query("UPDATE trades SET receive_amount = $1 WHERE id = $2")
        .bind(receive_amount)
        .bind(id)
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
