use crate::db;
use crate::models::{BitkubResponse, OrderInfoResult, PlaceBidPayload, ResultPlaceBid, TickerData};
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
    let path = "/api/v3/market/place-bid";

    // Prepare Data
    let payload = PlaceBidPayload {
        sym: "btc_thb".to_string(),
        amt: amount_thb,
        rat: 0,
        typ: "market".to_string(),
    };

    // Prepare Timestamp
    let _ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .to_string();

    // Serialize Payload
    let payload_str = serde_json::to_string(&payload)?;

    let resp_text = call_bitkub_api("POST", path, Some(payload_str), None).await?;
    tracing::info!("Response: {}", resp_text);
    let resp_json: BitkubResponse<ResultPlaceBid> = serde_json::from_str(&resp_text)?;

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
    let path = "/api/v3/market/my-order-history";

    let query = format!("?sym={}&lmt={}", sym, limit);
    let resp_text = call_bitkub_api("GET", path, None, Some(query.clone())).await?;

    let resp_json: serde_json::Value = serde_json::from_str(&resp_text)?;
    // tracing::info!("Query: {}", &query);
    // tracing::info!("Response: {:#?}", resp_json);

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

pub async fn get_order_info(
    sym: &str,
    order_id: &str,
    side: &str,
) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
    let path = "/api/v3/market/order-info";

    let query = format!("?sym={}&id={}&sd={}", sym, order_id, side);
    let resp_text = call_bitkub_api("GET", path, None, Some(query)).await?;

    tracing::info!("Order Info: {}", resp_text);
    let resp_json: BitkubResponse<OrderInfoResult> = serde_json::from_str(&resp_text)?;

    if resp_json.error != 0 {
        return Err(format!("Bitkub Error (order-info): {}", resp_json.error).into());
    }
    // info!("Order Info: {:#?}", resp_json.result);

    Ok(serde_json::to_value(resp_json.result)?)
}

pub async fn get_balances() -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
    let path = "/api/v3/market/balances";
    let resp_text = call_bitkub_api("POST", path, Some("{}".to_string()), None).await?;

    // Check for error in response
    let resp_json: serde_json::Value = serde_json::from_str(&resp_text)?;
    if let Some(error) = resp_json.get("error") {
        if error.as_i64().unwrap_or(0) != 0 {
            return Err(format!("Bitkub Error (balances): {}", error).into());
        }
    }

    Ok(resp_json)
}

pub async fn get_ticker(
    sym: Option<&str>,
) -> Result<Vec<TickerData>, Box<dyn Error + Send + Sync>> {
    let path = "/api/v3/market/ticker";
    let query = sym.map(|s| format!("?sym={}", s));
    let resp_text = call_bitkub_api("GET", path, None, query).await?;
    tracing::info!("Response: {}", resp_text);

    // Bitkub returns { "BTC_THB": { ... } }
    let resp_json: Vec<TickerData> = serde_json::from_str(&resp_text)?;
    Ok(resp_json)
}

pub async fn place_bid_limit(
    pool: &PgPool,
    amount_thb: i32,
    rate: i32,
) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
    // Config
    let path = "/api/v3/market/place-bid";

    // Prepare Data
    let payload = PlaceBidPayload {
        sym: "btc_thb".to_string(), // fixed symbol for now or pass as arg if needed
        amt: amount_thb,
        rat: rate,
        typ: "limit".to_string(),
    };

    // Serialize Payload
    let payload_str = serde_json::to_string(&payload)?;

    let resp_text = call_bitkub_api("POST", path, Some(payload_str), None).await?;
    tracing::info!("Response: {}", resp_text);
    let resp_json: BitkubResponse<ResultPlaceBid> = serde_json::from_str(&resp_text)?;

    if resp_json.error != 0 {
        let err_msg = format!("Bitkub Error (place_bid_limit): {}", resp_json.error);
        let _ = crate::bot::send_alert(&err_msg).await;
        //save error to db
        let _ = db::save_trade(
            pool,
            "btc_thb",
            amount_thb,
            rate,
            "limit",
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
    let _ = db::save_trade(pool, "btc_thb", amount_thb, rate, "limit", &result_json).await?;

    let amount_formatted = add_commas(&amount_thb.to_string());
    let rate_formatted = add_commas(&rate.to_string());
    let _ = crate::bot::send_alert(&format!(
        "✅ Bitkub Buy Limit: {} THB @ {} Rate",
        amount_formatted, rate_formatted
    ))
    .await;

    Ok(result_json)
}

pub async fn process_buy_limit(
    pool: &PgPool,
    amount: i32,
) -> Result<serde_json::Value, String> {
    if amount <= 0 {
        return Err("❌ Amount must be greater than 0. Usage: /buylimit <amount_thb>".to_string());
    }
    if amount > 1000 {
        return Err("❌ Error: Amount is too high. Max 1000 THB".to_string());
    }

    // 1. Get Ticker to find the best rate (Lowest Ask)
    let ticker_map = get_ticker(Some("BTC_THB"))
        .await
        .map_err(|e| format!("❌ Error fetching ticker: {}", e))?;

    let ticker = ticker_map
        .get(0)
        .ok_or_else(|| "❌ Error: BTC_THB ticker not found.".to_string())?;

    let rate_lowest_ask = ticker.lowest_ask.parse::<f64>().unwrap_or(0.0) as i32;
    tracing::info!("Picked Rate (Lowest Ask): {}", rate_lowest_ask);

    if rate_lowest_ask <= 0 {
        return Err("❌ Error: Invalid price from ticker.".to_string());
    }

    // 2. Place Limit Bid
    place_bid_limit(pool, amount, rate_lowest_ask)
        .await
        .map_err(|e| format!("❌ Error placing order: {}", e))
}

pub async fn get_my_open_orders(
    sym: &str,
) -> Result<Vec<crate::models::OpenOrder>, Box<dyn Error + Send + Sync>> {
    let path = "/api/v3/market/my-open-orders";
    let query = format!("?sym={}", sym);
    let resp_text = call_bitkub_api("GET", path, None, Some(query)).await?;
    
    let resp_json: crate::models::BitkubResponse<Vec<crate::models::OpenOrder>> = serde_json::from_str(&resp_text)?;
    
    if resp_json.error != 0 {
        return Err(format!("Bitkub Error (my-open-orders): {}", resp_json.error).into());
    }
    
    Ok(resp_json.result.unwrap_or_default())
}

pub async fn cancel_order(
    sym: &str,
    id: &str,
    side: &str,
) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
    let path = "/api/v3/market/cancel-order";
    let payload = crate::models::CancelOrderPayload {
        sym: sym.to_string(),
        id: id.to_string(),
        sd: side.to_string(),
    };
    
    let payload_str = serde_json::to_string(&payload)?;
    let resp_text = call_bitkub_api("POST", path, Some(payload_str), None).await?;
    
    let resp_json: serde_json::Value = serde_json::from_str(&resp_text)?;
    if let Some(error) = resp_json.get("error") {
        if error.as_i64().unwrap_or(0) != 0 {
            return Err(format!("Bitkub Error (cancel-order): {}", error).into());
        }
    }
    
    Ok(resp_json)
}

async fn call_bitkub_api(
    method: &str,
    path: &str,
    payload: Option<String>,
    query: Option<String>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let api_key = env::var("API_KEY").expect("API_KEY must be set");
    let api_secret = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
    let host = "https://api.bitkub.com";
   
    
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .to_string();
    tracing::info!("Call: {}{} {:?}",host,path,query);

    let mut data_to_sign = format!("{}{}{}", ts, method, path);

    if method == "GET" {
        if let Some(q) = &query {
            data_to_sign.push_str(q);
        }
    } else if let Some(p) = &payload {
        data_to_sign.push_str(p);
    }

    let mut mac = Hmac::<Sha256>::new_from_slice(api_secret.as_bytes())?;
    mac.update(data_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let client = reqwest::Client::new();
    let url = if let Some(q) = &query {
        format!("{}{}{}", host, path, q)
    } else {
        format!("{}{}", host, path)
    };

    let builder = if method == "POST" {
        client.post(url)
    } else {
        client.get(url)
    };

    let mut builder = builder
        .header("Content-Type", "application/json")
        .header("X-BTK-APIKEY", api_key)
        .header("X-BTK-TIMESTAMP", ts)
        .header("X-BTK-SIGN", signature);

    if let Some(p) = payload {
        builder = builder.body(p);
    }

    let res = builder.send().await?;
    let resp_text = res.text().await?;

    Ok(resp_text)
}
