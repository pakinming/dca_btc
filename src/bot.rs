use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;
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
    #[command(description = "Show current status.")]
    Status,
    #[command(description = "Show my recent order history. Usage: /history <limit>")]
    History(String),
    #[command(description = "Show info for the last order.")]
    OrderInfo,
    #[command(description = "Show wallet balances.")]
    Balance,
}

pub async fn run_bot(pool: Arc<PgPool>) {
    let bot = Bot::from_env();
    tracing::info!("Bot is running...");

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
                // Command::Buy(amount_str) => {
                //     let amount = amount_str.trim().parse::<i32>().unwrap_or(0);
                //     if amount <= 0 {
                //         bot.send_message(
                //             msg.chat.id,
                //             "❌ Amount must be greater than 0. Usage: /buy <amount_thb>",
                //         )
                //         .await?;
                //         return Ok(());
                //     }

                // match crate::bitkub::place_bid(&pool, amount).await {
                //         Ok(result_json) => {
                //             let response_msg = format!(
                //                 "✅ Success!\nAmount: {} THB\nBitkub ID: {}",
                //                 amount,
                //                 result_json["id"].as_str().unwrap_or("?")
                //             );
                //             bot.send_message(msg.chat.id, response_msg).await?;
                //         }
                //         Err(e) => {
                //             bot.send_message(msg.chat.id, format!("❌ Error: {}", e)).await?;
                //         }
                //     }
                // }
            
                Command::BuyLimit(amount_str) => {
                    let amount = amount_str.trim().parse::<i32>().unwrap_or(0);
                    
                    match crate::bitkub::process_buy_limit(&pool, amount).await {
                        Ok((msg_text, res)) => {
                             bot.send_message(msg.chat.id, msg_text).await?;
                               
                            // Wait for 10 seconds to let the order process
                            sleep(Duration::from_secs(10)).await;

                            // extract id from res
                            let order_id = res.get("id").and_then(|v| v.as_str());
                            // We need to fetch the ID from the database or the response. 
                            // The place_bid returns the result from Bitkub.
                                    
                            if let Some(oid) = order_id {
                                         
                                match db::get_trade_by_order_id(&pool, oid).await {
                                    Ok(Some(trade)) => {
                                        if let Some(trade_rec) = trade.response_json.get("rec").and_then(|v| v.as_f64()) {
                                            if trade_rec > 0.0 {
                                                match db::update_trade_receive(&pool, trade.id, trade_rec).await {
                                                    Ok(_) => tracing::info!("✅ COMMAND: /buylimit Updated trade receive amount for ID: {}", trade.id),
                                                    Err(e) => tracing::error!("❌ COMMAND: /buylimit Failed to update trade receive amount for ID: {}", trade.id),
                                                }                                                        
                                            }
                                        }
                                    }
                                    _ => tracing::warn!("⚠️ COMMAND: /buylimit Could not match latest trade to update receive amount."),
                                }
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
