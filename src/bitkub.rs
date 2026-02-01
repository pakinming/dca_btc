use crate::db;
use crate::models::{BitkubResponse, PlaceBidPayload};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use sqlx::PgPool;
use std::env;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn place_bid(
    pool: &PgPool,
    amount_thb: i32,
) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
    // Config
    let api_key = env::var("API_KEY").expect("API_KEY must be set");
    let api_secret = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
    let host = "https://api.bitkub.com";
    let path = "/api/v3/market/place-bid";

    // Prepare Data
    let payload = PlaceBidPayload {
        sym: "btc_thb".to_string(),
        amt: amount_thb,
        rat: 0,
        typ: "market".to_string(),
    };

    // Prepare Timestamp
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .to_string();

    // Serialize Payload
    let payload_str = serde_json::to_string(&payload)?;

    // Create Signature
    let method = "POST";
    let data_to_sign = format!("{}{}{}{}", ts, method, path, payload_str);

    let mut mac = Hmac::<Sha256>::new_from_slice(api_secret.as_bytes())?;
    mac.update(data_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    // NOTE: สำหรับตอนนี้เราจะ Return Mock Data ตามที่ User Request ไว้ก่อน
    // แต่โค้ดข้างบนคือ Logic จริงสำหรับการ Sign Request

    let client = reqwest::Client::new();
    let url = format!("{}{}", host, path);

    let res = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("X-BTK-APIKEY", api_key)
        .header("X-BTK-TIMESTAMP", ts)
        .header("X-BTK-SIGN", signature)
        .body(payload_str)
        .send()
        .await?;

    let resp_text = res.text().await?;
    println!("Response: {}", resp_text);
    let resp_json: BitkubResponse = serde_json::from_str(&resp_text)?;

    if resp_json.error != 0 {
        let err_msg = format!("Bitkub Error (place_bid): {}", resp_json.error);
        let _ = crate::bot::send_alert(&err_msg).await;
        //save error to db
        let _ = db::save_trade(
            pool,
            "btc_thb",
            amount_thb,
            0,
            "market",
            &json!({ "error": resp_json.error.to_string() }),
        )
        .await?;
        return Err(format!("Bitkub Error: {}", resp_json.error).into());
    }

    let result_json = match resp_json.result {
        Some(data) => serde_json::to_value(data)?,
        None => json!({}),
    };

    // Save to Database
    let _ = db::save_trade(pool, "btc_thb", amount_thb, 0, "market", &result_json).await?;

    //get my order history
    let my_order_history = get_my_order_history("BTC_THB", 1).await?;
    // let rate = my_order_history.get("rate").and_then(|v| v.as_str()).unwrap_or("0");
    let rate = my_order_history["result"][0]["rate"]
        .as_str()
        .unwrap_or("0");

    let amount_formatted = add_commas(&amount_thb.to_string());
    let rate_formatted = add_commas(rate);
    let _ = crate::bot::send_alert(&format!(
        "✅ Bitkub Buy: {} THB @ {} BTC",
        amount_formatted, rate_formatted
    ))
    .await;

    Ok(result_json)
}

fn add_commas(s: &str) -> String {
    let parts: Vec<&str> = s.split('.').collect();
    let int_part = parts[0];
    let mut result = String::new();
    let mut count = 0;
    for c in int_part.chars().rev() {
        if count > 0 && count % 3 == 0 && c != '-' {
            result.push(',');
        }
        result.push(c);
        count += 1;
    }
    let formatted_int: String = result.chars().rev().collect();

    if parts.len() > 1 {
        format!("{}.{}", formatted_int, parts[1])
    } else {
        formatted_int
    }
}

pub async fn get_my_order_history(
    sym: &str,
    limit: i32,
) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
    // Config
    let api_key = env::var("API_KEY").expect("API_KEY must be set");
    let api_secret = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
    let host = "https://api.bitkub.com";
    let path = "/api/v3/market/my-order-history";
    // Construct query properly
    let query = format!("?sym={}&lmt={}", sym, limit);

    // Prepare Timestamp
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .to_string();

    // Create Signature
    let method = "GET";
    // NOTE: For GET requests with query params, usage of query string in signature depends on API specifics.
    // Standard Bitkub V3 practice for GET is ts + method + path + query_string
    let data_to_sign = format!("{}{}{}{}", ts, method, path, query);

    let mut mac = Hmac::<Sha256>::new_from_slice(api_secret.as_bytes())?;
    mac.update(data_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let client = reqwest::Client::new();
    let url = format!("{}{}{}", host, path, query);

    let res = client
        .get(&url)
        .header("Content-Type", "application/json")
        .header("X-BTK-APIKEY", api_key)
        .header("X-BTK-TIMESTAMP", ts)
        .header("X-BTK-SIGN", signature)
        .send()
        .await?;

    let resp_json: serde_json::Value = res.json().await?;

    // Check if it's an error response
    if let Some(error) = resp_json.get("error") {
        if error.as_i64().unwrap_or(0) != 0 {
            let err_msg = format!("Bitkub Error (history): {}", error);
            let _ = crate::bot::send_alert(&err_msg).await;
            return Err(format!("Bitkub Error: {}", error).into());
        }
    }

    let _ = crate::bot::send_alert("📋 Bitkub API: Order History Checked").await;

    Ok(resp_json)
}

//  Ok(resp_json.result.unwrap_or(json!({})))

// Return Mock Data
//     let mock_result = json!({
//         "amt": amount_thb,
//         "ci": "",
//         "cre": 0,
//         "fee": 0,
//         "id": "697ef176132acb3b40521bd1m8a2qe",
//         "rat": 0,
//         "rec": 0,
//         "ts": ts,
//         "typ": "market"
//     });

//     Ok(mock_result)
