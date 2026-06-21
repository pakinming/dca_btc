use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::FromRow;

pub fn deserialize_f64_from_string_or_number<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(n) => n.as_f64().ok_or_else(|| serde::de::Error::custom("Invalid f64")),
        serde_json::Value::String(s) => s.parse::<f64>().map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom("Expected string or number")),
    }
}

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
    //  {"error":0,"result":{"amt":9.97,"ci":"","cre":0,"fee":0.03,"id":"6980d4a97a28cae03ba2ab4bm8a2qe","rat":2491999,"rec":0.000004,"ts":"1770050729","typ":"limit"}
    pub id: String,
    pub typ: String,
    pub amt: f64,
    pub rat: i32,
    pub fee: f64,
    pub cre: f64,
    pub rec: f64,
    pub ts: String,
    pub ci: String,
}

#[derive(Deserialize, Debug)]
pub struct BitkubResponse<T> {
    pub error: i32,
    pub result: Option<T>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OrderInfoHistory {
    pub id: Option<String>,
    pub amount: Option<f64>,
    pub credit: Option<f64>,
    pub fee: Option<f64>,
    pub rate: Option<f64>,
    pub timestamp: Option<i64>,
    pub txn_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OrderInfoResult {
    pub id: String,
    pub first: Option<String>,
    pub parent: Option<String>,
    pub last: Option<String>,
    pub client_id: Option<String>,
    pub side: Option<String>,
    pub amount: Option<f64>,
    pub rate: Option<f64>,
    pub fee: Option<f64>,
    pub credit: Option<f64>,
    pub filled: Option<f64>,
    pub total: Option<f64>,
    pub status: Option<String>,
    #[serde(rename = "partial_filled")]
    pub partial: Option<bool>,
    pub remaining: Option<f64>,
    pub post_only: Option<bool>,
    pub history: Option<Vec<OrderInfoHistory>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenOrder {
    pub id: String,
    pub hash: Option<String>,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    #[serde(deserialize_with = "deserialize_f64_from_string_or_number")]
    pub rate: f64,
    #[serde(deserialize_with = "deserialize_f64_from_string_or_number")]
    pub amount: f64,
    #[serde(deserialize_with = "deserialize_f64_from_string_or_number")]
    pub receive: f64,
    #[serde(deserialize_with = "deserialize_f64_from_string_or_number")]
    pub fee: f64,
    pub client_id: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct CancelOrderPayload {
    pub sym: String,
    pub id: String,
    pub sd: String,
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
    pub receive_amount: Option<f64>,
    pub status: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct TickerData {
    pub symbol: Option<String>,
    pub last: String,
    pub lowest_ask: String,
    pub highest_bid: String,
    pub percent_change: String,
    pub base_volume: String,
    pub quote_volume: String,
    pub high_24_hr: String,
    pub low_24_hr: String,
}
// ฟิลด์ (Field),ความหมาย,คำอธิบายเพิ่มเติม
// symbol,ชื่อคู่เหรียญ,ในที่นี้คือ ADA_THB หมายถึงการซื้อขายเหรียญ ADA ด้วยเงินบาท
// base_volume,ปริมาณการซื้อขาย (Base Asset),จำนวนเหรียญ ADA ที่มีการซื้อขายไปทั้งหมดใน 24 ชั่วโมงที่ผ่านมา
// quote_volume,มูลค่าการซื้อขาย (Quote Asset),มูลค่ารวมที่เป็นเงินบาท (THB) จากการซื้อขายทั้งหมดใน 24 ชั่วโมง
// high_24_hr,ราคาสูงสุด 24 ชม.,ราคาที่สูงที่สุดที่มีการจับคู่ซื้อขายสำเร็จในรอบ 1 วัน 📈
// low_24_hr,ราคาต่ำสุด 24 ชม.,ราคาที่ต่ำที่สุดที่มีการจับคู่ซื้อขายสำเร็จในรอบ 1 วัน 📉
// last,ราคาล่าสุด,ราคาที่เพิ่งมีการตกลงซื้อขายกันสำเร็จล่าสุด (Market Price)
// highest_bid,ราคารับซื้อสูงสุด,ราคาที่ดีที่สุดที่ ฝั่งคนซื้อ ตั้งรอไว้ (ถ้าคุณจะขายทันที คุณจะได้ราคานี้)
// lowest_ask,ราคาเสนอขายต่ำสุด,ราคาที่ดีที่สุดที่ ฝั่งคนขาย ตั้งรอไว้ (ถ้าคุณจะซื้อทันที คุณต้องซื้อราคานี้)
// percent_change,เปอร์เซ็นต์การเปลี่ยนแปลง,ราคาเปลี่ยนแปลงไปกี่เปอร์เซ็นต์เมื่อเทียบกับ 24 ชั่วโมงก่อนหน้า
