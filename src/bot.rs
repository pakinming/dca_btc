use sqlx::PgPool;
use tracing::info;
use std::time::Duration;
use tokio::time::sleep;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

use crate::db;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Display this text.")]
    Help,
    // #[command(description = "Buy BTC. Usage: /buy <amount_thb>")]
    // Buy(String),
    #[command(description = "Buy BTC with Limit Order. Usage: /buylimit <amount_thb>")]
    BuyLimit(String),
    #[command(description = "Buy BTC with Limit Order (500 THB).")]
    BuyLimit500,
    #[command(description = "Buy BTC with Limit Order (1000 THB).")]
    BuyLimit1000,
    #[command(description = "Show current status.")]
    Status,
    #[command(description = "Show my recent order history. Usage: /history <limit>")]
    History(String),
    #[command(description = "Show info for the last order.")]
    OrderInfo,
    #[command(description = "Show wallet balances.")]
    Balance,
    #[command(description = "Show my open orders. Usage: /openorders")]
    OpenOrders,
    #[command(description = "Cancel an order. Usage: /cancel <order_id>")]
    Cancel(String),
    #[command(description = "Show my BTC portfolio and profit/loss.")]
    Portfolio,
}

pub async fn run_bot(pool: Arc<PgPool>) {
    let bot = Bot::from_env();
    tracing::info!("Bot is running...");

    match bot.set_my_commands(Command::bot_commands()).await {
        Ok(_) => tracing::info!("Bot commands have been set successfully."),
        Err(e) => tracing::error!("Failed to set bot commands: {}", e),
    }

    Command::repl(bot, move |bot: Bot, msg: Message, cmd: Command| {
        let pool = pool.clone();
        async move {
            // Check Authorization
            let current_chat_id = msg.chat.id.to_string();
            let authorized_id_str = std::env::var("AUTHORIZED_USER_ID").ok();

            if let Some(auth_id) = authorized_id_str {
                if auth_id != current_chat_id {
                    bot.send_message(
                        msg.chat.id,
                        format!("⛔ Unauthorized. Your Chat ID is: `{}`", current_chat_id),
                    )
                    .await?;
                    return Ok(());
                }
            } else {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "⚠️ Security Warning: AUTHORIZED_USER_ID is not set.\nYour Chat ID is: `{}`\nPlease add `AUTHORIZED_USER_ID={}` to your .env file.",
                        current_chat_id, current_chat_id
                    ),
                )
                .await?;
                return Ok(());
            }

            match cmd {
                Command::Help => {
                    bot.send_message(msg.chat.id, Command::descriptions().to_string())
                        .await?;
                }
                
                Command::BuyLimit1000 => {
                    let amount = 1000;
                    match crate::bitkub::process_buy_limit(&pool, amount).await {
                        Ok(res) => {
                            if let Some(oid) = res.get("id").and_then(|v| v.as_str()) {
                                let pool_clone = pool.clone();
                                let oid_str = oid.to_string();
                                tokio::spawn(async move {
                                    wait_and_verify_order(pool_clone, oid_str).await;
                                });
                            }

                        }
                        Err(e) => {
                             bot.send_message(msg.chat.id, e).await?;
                        }
                    }
                }
                Command::BuyLimit500 => {
                    let amount = 500;
                    match crate::bitkub::process_buy_limit(&pool, amount).await {
                        Ok(res) => {
                            if let Some(oid) = res.get("id").and_then(|v| v.as_str()) {
                                let pool_clone = pool.clone();
                                let oid_str = oid.to_string();
                                tokio::spawn(async move {
                                    wait_and_verify_order(pool_clone, oid_str).await;
                                });
                            }

                        }
                        Err(e) => {
                             bot.send_message(msg.chat.id, e).await?;
                        }
                    }
                }
                Command::BuyLimit(amount_str) => {
                    let amount = amount_str.trim().parse::<i32>().unwrap_or(0);
                    
                    match crate::bitkub::process_buy_limit(&pool, amount).await {
                        Ok(res) => {
                               
                            if let Some(oid) = res.get("id").and_then(|v| v.as_str()) {
                                let pool_clone = pool.clone();
                                let oid_str = oid.to_string();
                                tokio::spawn(async move {
                                    wait_and_verify_order(pool_clone, oid_str).await;
                                });
                            }

                        }
                        Err(e) => {
                             bot.send_message(msg.chat.id, e).await?;
                        }
                    }
                }
                Command::Status => {
                    bot.send_message(
                        msg.chat.id,
                        "🤖 System Status: ONLINE\nMenu: Bitkub DCA Bot",
                    )
                    .await?;
                }
                Command::History(limit) => {
                    tracing::info!("Trades: get_my_order_history");
                    let limit = limit.trim().parse::<i32>().unwrap_or(0);
                    if limit <= 0 {
                        bot.send_message(
                            msg.chat.id,
                            "❌ Limit must be greater than 0. Usage: /history <limit>",
                        )
                        .await?;
                        return Ok(());
                    }
                    match crate::bitkub::get_my_order_history("BTC_THB", limit).await {
                        Ok(trades) => {
                             let mut message = "📊 My Recent Trades (BTC/THB):\n".to_string();
                             if let Some(result) = trades.get("result") {
                                 if let Some(arr) = result.as_array() {
                                     for (_i, trade) in arr.iter().enumerate() {
                                         // Structure of my-order-history might differ from public trades
                                         // Example: {"txn_id": "...", "order_id": "...", "hash": "...", "rate": 100, "amt": 1, "fee": 0, "cred": 0, "ts": 123...}
                                        //  let rate = trade.get("rate").and_then(|v| v.as_str()).unwrap_or("0");
                                        //  let amount = trade.get("amount").and_then(|v| v.as_str()).unwrap_or("0");
                                        //  let side = trade.get("side").and_then(|v| v.as_str()).unwrap_or("?");
                                        //  let fee = trade.get("fee").and_then(|v| v.as_str()).unwrap_or("0");

                                         let data = serde_json::to_string_pretty(trade).unwrap();
                                         message.push_str(data.as_str());
                                         message.push_str("\n");

                                         
                                       
                                     }
                                 }
                             } else {
                                  message.push_str("No trade data found.");
                             }
                             bot.send_message(msg.chat.id, message).await?;
                        }
                        Err(e) => {
                            bot.send_message(msg.chat.id, format!("❌ Error: {}", e)).await?;
                        }
                    }
                }
                Command::OrderInfo => {
                    match crate::db::get_latest_trade(&pool).await {
                         Ok(Some(trade)) => {
                             // Assuming order_id is in response_json["result"]["id"]
                             // Note: response_json structure depends on Bitkub response, check models.
                             let order_id = trade.response_json.get("id").and_then(|v| v.as_str());

                             tracing::info!("Order ID: {}", order_id.clone().unwrap());
                             
                             if let Some(oid) = order_id {
                                 match crate::bitkub::get_order_info(&trade.symbol, oid, "buy").await {
                                     Ok(info) => {

                                         let pretty_json = serde_json::to_string_pretty(&info).unwrap_or("Error serializing".into());
                                         bot.send_message(msg.chat.id, format!("📄 Order Info for ID: {}\n```json\n{}\n```", oid, pretty_json)).await?;
                                     }
                                     Err(e) => {
                                        tracing::error!(e);
                                        bot.send_message(msg.chat.id, format!("❌ Error fetching order info: {}", e)).await?;
                                     }
                                 }
                             } else {
                                tracing::error!("Could not extract Order ID from last trade.");
                                bot.send_message(msg.chat.id, "❌ Could not extract Order ID from last trade.").await?;
                             }
                         }
                         Ok(None) => {
                            tracing::error!("No trades found in database.");
                             bot.send_message(msg.chat.id, "❌ No trades found in database.").await?;
                         }
                         Err(e) => {
                            tracing::error!("Error fetching trade from database: {}", e);
                            bot.send_message(msg.chat.id, format!("❌ Database Error: {}", e)).await?;
                         }
                    }
                }
                Command::Balance => {
                    tracing::info!("Checking Balances...");
                    match crate::bitkub::get_balances().await {
                         Ok(balances) => {
                             if let Some(result) = balances.get("result") {
                                 let thb_avail = result["THB"]["available"].as_f64().unwrap_or(0.0);
                                 let thb_reserved = result["THB"]["reserved"].as_f64().unwrap_or(0.0);
                                 let btc_avail = result["BTC"]["available"].as_f64().unwrap_or(0.0);
                                 let btc_reserved = result["BTC"]["reserved"].as_f64().unwrap_or(0.0);

                                 let response_msg = format!(
                                     "💰 Wallet Balances:\n\n🇹🇭 THB:\nAvailable: {:.2}\nReserved: {:.2}\n\n₿ BTC:\nAvailable: {:.6}\nReserved: {:.6}",
                                     thb_avail, thb_reserved, btc_avail, btc_reserved
                                 );
                                 bot.send_message(msg.chat.id, response_msg).await?;
                             } else {
                                 bot.send_message(msg.chat.id, "❌ Error: Could not parse balance data.").await?;
                             }
                        }
                        Err(e) => {
                            bot.send_message(msg.chat.id, format!("❌ Error: {}", e)).await?;
                        }
                    }
                }
                Command::OpenOrders => {
                    match crate::bitkub::get_my_open_orders("BTC_THB").await {
                        Ok(orders) => {
                            if orders.is_empty() {
                                bot.send_message(msg.chat.id, "📭 No open orders found.").await?;
                            } else {
                                let mut message = "📋 My Open Orders:\n".to_string();
                                for o in orders {
                                    message.push_str(&format!("ID: `{}`\nType: {} {}\nRate: {}\nAmount: {}\n\n", o.id, o.side, o.order_type, o.rate, o.amount));
                                }
                                bot.send_message(msg.chat.id, message).await?;
                            }
                        }
                        Err(e) => {
                            bot.send_message(msg.chat.id, format!("❌ Error fetching open orders: {}", e)).await?;
                        }
                    }
                }
                Command::Cancel(order_id) => {
                    let oid = order_id.trim();
                    if oid.is_empty() {
                        bot.send_message(msg.chat.id, "❌ Usage: /cancel <order_id>").await?;
                        return Ok(());
                    }
                    match crate::bitkub::cancel_order("BTC_THB", oid, "buy").await {
                        Ok(_) => {
                            // Update DB immediately
                            let _ = crate::db::update_trade_status_by_order_id(&pool, oid, "cancelled").await;
                            bot.send_message(msg.chat.id, format!("✅ Cancelled order `{}` successfully.", oid)).await?;
                        }
                        Err(e) => {
                            bot.send_message(msg.chat.id, format!("❌ Failed to cancel order `{}`: {}", oid, e)).await?;
                        }
                    }
                }
                Command::Portfolio => {
                    tracing::info!("Calculating portfolio summary...");
                    match crate::db::get_portfolio_stats(&pool).await {
                        Ok(stats) => {
                            if stats.total_capital <= 0.0 || stats.total_btc <= 0.0 {
                                bot.send_message(
                                    msg.chat.id,
                                    "📭 ไม่พบประวัติการซื้อสำเร็จ (filled trades) ในฐานข้อมูล",
                                )
                                .await?;
                            } else {
                                match crate::bitkub::get_ticker(Some("BTC_THB")).await {
                                    Ok(ticker_map) => {
                                        if let Some(ticker) = ticker_map.get(0) {
                                            let current_price = ticker.last.parse::<f64>().unwrap_or(0.0);
                                            if current_price > 0.0 {
                                                let total_capital = stats.total_capital;
                                                let total_btc = stats.total_btc;
                                                let avg_price = total_capital / total_btc;
                                                let current_value = total_btc * current_price;
                                                let profit_loss = current_value - total_capital;
                                                let profit_loss_percent = (profit_loss / total_capital) * 100.0;

                                                let pnl_emoji = if profit_loss >= 0.0 { "🟢" } else { "🔴" };
                                                let pnl_prefix = if profit_loss >= 0.0 { "+" } else { "" };

                                                let message = format!(
                                                    "📊 สรุปข้อมูลพอร์ต BTC (DCA Bot)\n\n\
                                                     💰 ขนาดทุนทั้งหมด: {} THB\n\
                                                     🪙 BTC ที่ซื้อได้ทั้งหมด: {:.8} BTC\n\
                                                     🏷️ ราคาเฉลี่ย: {} THB/BTC\n\n\
                                                     📈 ราคาตลาดปัจจุบัน: {} THB/BTC\n\
                                                     💵 มูลค่าปัจจุบัน: {} THB\n\n\
                                                     {} กำไร/ขาดทุน: {}{} THB ({}{:.2}%)\n",
                                                    crate::bitkub::add_commas(&format!("{:.2}", total_capital)),
                                                    total_btc,
                                                    crate::bitkub::add_commas(&format!("{:.2}", avg_price)),
                                                    crate::bitkub::add_commas(&format!("{:.2}", current_price)),
                                                    crate::bitkub::add_commas(&format!("{:.2}", current_value)),
                                                    pnl_emoji,
                                                    pnl_prefix,
                                                    crate::bitkub::add_commas(&format!("{:.2}", profit_loss)),
                                                    pnl_prefix,
                                                    profit_loss_percent
                                                );

                                                bot.send_message(msg.chat.id, message).await?;
                                            } else {
                                                bot.send_message(msg.chat.id, "❌ ไม่สามารถอ่านราคา BTC ปัจจุบันจาก Bitkub ticker ได้").await?;
                                            }
                                        } else {
                                            bot.send_message(msg.chat.id, "❌ ไม่พบข้อมูลราคา BTC ใน Ticker").await?;
                                        }
                                    }
                                    Err(e) => {
                                        bot.send_message(msg.chat.id, format!("❌ ไม่สามารถดึงราคาล่าสุดได้: {}", e)).await?;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            bot.send_message(msg.chat.id, format!("❌ เกิดข้อผิดพลาดกับฐานข้อมูล: {}", e)).await?;
                        }
                    }
                }
            };
            Ok(())
        }
    })

    .await;
}

pub async fn send_alert(message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bot = Bot::from_env();
    // Use AUTHORIZED_USER_ID to send the alert
    let chat_id_str = std::env::var("AUTHORIZED_USER_ID").expect("AUTHORIZED_USER_ID must be set");
    let chat_id = chat_id_str.parse::<i64>().expect("AUTHORIZED_USER_ID must be a valid integer"); // Parsing to i64 for ChatId

    bot.send_message(teloxide::types::ChatId(chat_id), message).await?;
    Ok(())
}

pub fn calculate_total_receive(order_info: &crate::models::OrderInfoResult) -> (f64, bool) {
    let mut total_receive = 0.0;
    let mut found_history = false;
    
    if let Some(history) = &order_info.history {
        if !history.is_empty() {
            found_history = true;
            for h in history {
                let amt = h.amount.unwrap_or(0.0);
                let rate = h.rate.unwrap_or(0.0);
                if rate > 0.0 {
                    total_receive += amt / rate;
                }
            }
        }
    }
    
    // Bitkub truncates to 8 decimal places for crypto
    total_receive = (total_receive * 100_000_000.0).trunc() / 100_000_000.0;
    
    (total_receive, found_history)
}

pub fn build_telegram_msg_template(spent: f64, rate: f64, received: f64, time_str: &str) -> String {
    format!(
        "🤗🎉 You Spent : {:.2} THB\nPrice : {} THB/BTC\nYou Received : {:.8} BTC\n\nTime : {}",
        spent, rate, received, time_str
    )
}

pub async fn wait_and_verify_order(pool: std::sync::Arc<PgPool>, order_id: String) {
    let interval = Duration::from_secs(10);
    let status_alert_interval = 600; // 10 minutes (600s)
    let cancel_timeout = 3600; // 1 hour (3600s)
    let mut time_elapsed_since_last_alert = 0;
    let mut total_time_elapsed = 0;

    tracing::info!("⏳ Starting order verification task for ID: {} (infinite loop until matched)", order_id);

    // ดึงข้อมูล Trade จาก Database แค่ครั้งเดียว (ไม่ต้องทำซ้ำใน Loop)
    let trade = match db::get_trade_by_order_id(&pool, &order_id).await {
        Ok(Some(t)) => t,
        _ => {
            tracing::warn!("⚠️ Could not find trade in database for order_id: {}. Aborting task.", order_id);
            return;
        }
    };

    loop {
        sleep(interval).await;
        let secs = interval.as_secs();
        time_elapsed_since_last_alert += secs;
        total_time_elapsed += secs;

        match crate::bitkub::get_order_info("BTC_THB", &order_id, "buy").await {
            Ok(info_value) => {
                if let Ok(order_info) = serde_json::from_value::<crate::models::OrderInfoResult>(info_value) {
                    let status = order_info.status.clone().unwrap_or_default();
                    let filled = order_info.filled.unwrap_or(0.0);
                                
                    if status == "filled" || status == "cancelled" {
                        if status == "cancelled" && filled == 0.0 {
                            let _ = db::update_trade_status(&pool, trade.id, "cancelled").await;
                            let msg = format!("🚫 Order `{}` was cancelled.", order_id);
                            let _ = send_alert(&msg).await;
                        } 
                        let (total_receive, found_history) = calculate_total_receive(&order_info);
                        
                        // 1. อัปเดต total_receive ทันทีเมื่อซื้อขายสำเร็จ
                        if total_receive > 0.0 {
                            match db::update_trade_receive(&pool, trade.id, total_receive).await {
                                Ok(_) => tracing::info!("✅ Verified & Updated trade receive amount for ID: {} to {:.8}", trade.id, total_receive),
                                Err(e) => tracing::error!("❌ Failed to update trade receive amount for ID: {}: {}", trade.id, e),
                            }
                        } else {
                            tracing::warn!("⚠️ Order history showed 0 received.");
                        }

                        // 2. ทำการส่ง Alert และจัดการไม้ที่ถูก Split (ถ้ามี)
                        if found_history {
                            use chrono::{TimeZone, Utc, FixedOffset};
                            if let Some(history) = &order_info.history {
                                let mut total_history_amount = 0.0;
                                for h in history {
                                    total_history_amount += h.amount.unwrap_or(0.0) + h.fee.unwrap_or(0.0);
                                }
                                
                                let is_split = history.len() > 1 || (trade.amount_thb as f64 - total_history_amount).abs() >= 1.0;
                                tracing::info!("is_split: {} history.len(): {} total_history_amount: {} trade.amount_thb: {}", is_split, history.len(), total_history_amount, trade.amount_thb);
                                
                                for h in history {
                                    let amt = h.amount.unwrap_or(0.0);
                                    let fee = h.fee.unwrap_or(0.0);
                                    let spent = amt + fee;
                                    let rate = h.rate.unwrap_or(0.0);
                                    let mut received = 0.0;
                                    if rate > 0.0 {
                                        received = amt / rate;
                                        received = (received * 100_000_000.0).trunc() / 100_000_000.0;
                                    }

                                    let ts = h.timestamp.unwrap_or(0);
                                    if let chrono::LocalResult::Single(dt) = Utc.timestamp_millis_opt(ts) {
                                        let tz = FixedOffset::east_opt(7 * 3600).unwrap();
                                        let local_dt = dt.with_timezone(&tz);
                                        let time_str = local_dt.format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string();

                                        let msg = build_telegram_msg_template(spent, rate, received, &time_str);
                                        let _ = send_alert(&msg).await;
                                    }
                                    
                                    // Create a new record ONLY if it is split
                                    if is_split {
                                        if let Ok(h_val) = serde_json::to_value(h) {
                                            match db::save_trade(&pool, "btc_thb", amt as i32, rate as i32, "limit", &h_val).await {
                                                Ok(new_id) => {
                                                    tracing::info!("✅ Created new match record ID: {}", new_id);
                                                    if received > 0.0 {
                                                        let _ = db::update_trade_receive(&pool, new_id, received).await;
                                                    }
                                                }
                                                Err(e) => tracing::error!("❌ Failed to create match record: {}", e),
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        break;
                    } else {
                        tracing::info!("⏳ Order {} status: {}. Waiting...", order_id, status);
                        
                        if total_time_elapsed >= cancel_timeout {
                            tracing::warn!("⏳ Order {} timeout (1hr). Cancelling...", order_id);
                            match crate::bitkub::cancel_order("BTC_THB", &order_id, "buy").await {
                                Ok(_) => {
                                    let _ = db::update_trade_status(&pool, trade.id, "cancelled").await;
                                    let filled = order_info.filled.unwrap_or(0.0);
                                    let remaining_thb = trade.amount_thb as f64 - filled;
                                    let msg = format!("⏳ Order `{}` took too long (> 1 hr). Cancelled.\n🔄 Reopening new order for remaining {:.2} THB...", order_id, remaining_thb);
                                    let _ = send_alert(&msg).await;
                                    
                                    if remaining_thb >= 10.0 {
                                        let pool_clone = pool.clone();
                                        tokio::spawn(async move {
                                            if let Err(e) = crate::bitkub::process_buy_limit(&pool_clone, remaining_thb as i32).await {
                                                tracing::error!("❌ Failed to reopen order: {}", e);
                                                let _ = send_alert(&format!("❌ Failed to reopen order: {}", e)).await;
                                            }
                                        });
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("❌ Failed to cancel order automatically: {}", e);
                                }
                            }
                            break;
                        }
                        
                        if time_elapsed_since_last_alert >= status_alert_interval {
                            let msg = format!("⏳ Order `{}` status: {}. Still waiting for match...", order_id, status);
                            let _ = send_alert(&msg).await;
                            time_elapsed_since_last_alert = 0;
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("❌ Failed to get order info for ID: {}: {}", order_id, e);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_total_receive() {
        let json_payload = r#"{
            "id": "6a1ea61cf39c7bd33b38271fm8a2qe",
            "side": "buy",
            "amount": 500,
            "rate": 2258819,
            "fee": 1.26,
            "credit": 0,
            "filled": 498.74,
            "total": 500,
            "status": "filled",
            "history": [
                {
                    "id": "6a1ea61cf39c7bd33b38271fm8a2qe",
                    "amount": 489.99,
                    "credit": 0,
                    "fee": 1.23,
                    "rate": 2258819,
                    "timestamp": 1780393504219,
                    "txn_id": "6a1ea62022d997e15fdbd279m8a2qe"
                },
                {
                    "id": "6a1ea61cf39c7bd33b38271fm8a2qe",
                    "amount": 8.75,
                    "credit": 0,
                    "fee": 0.03,
                    "rate": 2258819,
                    "timestamp": 1780393510406,
                    "txn_id": "6a1ea62622d997e15fdbd282m8a2qe"
                }
            ]
        }"#;

        let order_info: crate::models::OrderInfoResult = serde_json::from_str(json_payload).unwrap();
        let (total_receive, found_history) = calculate_total_receive(&order_info);

        assert!(found_history);
        
        let expected_receive = 0.00022079;
        
        assert!((total_receive - expected_receive).abs() < 1e-8);
        assert!((total_receive - 0.00022079).abs() < 1e-8);
    }
}