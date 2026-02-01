use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// โครงสร้าง Request Body สำหรับ Bitkub (เก็บไว้เผื่อใช้ในอนาคต)
#[derive(Serialize, Debug)]
pub struct PlaceBidPayload {
    pub sym: String,
    pub amt: i32,
    pub rat: i32,
    pub typ: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResultPlaceBid {
    // "result": {
    // "id": "1", // order id
    // "typ": "limit", // order type
    // "amt": 1000, // spending amount
    // "rat": 15000, // rate
    // "fee": 2.5, // fee
    // "cre": 2.5, // fee credit used
    // "rec": 0.06666666, // amount to receive
    // "ts": "1707220636" // timestamp
    // "ci": "input_client_id" // input id for reference
    //   }
    pub id: String,
    pub typ: String,
    pub amt: i32,
    pub rat: i32,
    pub fee: f64,
    pub cre: f64,
    pub rec: f64,
    pub ts: String,
    pub ci: String,
}

// โครงสร้าง Response จาก Bitkub (ย่อ)
#[derive(Deserialize, Debug)]
pub struct BitkubResponse {
    pub error: i32,
    pub result: Option<ResultPlaceBid>,
}

// โครงสร้างข้อมูล Trade ใน Database
#[derive(FromRow, Debug)]
pub struct Trade {
    pub id: i32,
    pub symbol: String,
    pub amount_thb: i32,
    pub rat: i32,
    #[sqlx(rename = "type")]
    pub type_trade: String,
    pub timestamp: DateTime<Utc>,
    pub response_json: serde_json::Value,
}
