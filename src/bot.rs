use sqlx::PgPool;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Display this text.")]
    Help,
    #[command(description = "Buy BTC. Usage: /buy <amount_thb>")]
    Buy(String),
    #[command(description = "Show current status.")]
    Status,
    #[command(description = "Show my recent order history.")]
    History,
}

pub async fn run_bot(pool: Arc<PgPool>) {
    let bot = Bot::from_env();
    println!("Bot is running...");

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
                Command::Buy(amount_str) => {
                    let amount = amount_str.trim().parse::<i32>().unwrap_or(0);
                    if amount <= 0 {
                        bot.send_message(
                            msg.chat.id,
                            "❌ Amount must be greater than 0. Usage: /buy <amount_thb>",
                        )
                        .await?;
                        return Ok(());
                    }

                match crate::bitkub::place_bid(&pool, amount).await {
                        Ok(result_json) => {
                            let response_msg = format!(
                                "✅ Success!\nAmount: {} THB\nBitkub ID: {}",
                                amount,
                                result_json["id"].as_str().unwrap_or("?")
                            );
                            bot.send_message(msg.chat.id, response_msg).await?;
                        }
                        Err(e) => {
                            bot.send_message(msg.chat.id, format!("❌ Error: {}", e)).await?;
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
                Command::History => {
                    match crate::bitkub::get_my_order_history("BTC_THB", 3).await {
                        Ok(trades) => {
                             let mut message = "📊 My Recent Trades (BTC/THB):\n".to_string();
                             if let Some(result) = trades.get("result") {
                                 if let Some(arr) = result.as_array() {
                                     for (i, trade) in arr.iter().enumerate() {
                                         // Structure of my-order-history might differ from public trades
                                         // Example: {"txn_id": "...", "order_id": "...", "hash": "...", "rate": 100, "amt": 1, "fee": 0, "cred": 0, "ts": 123...}
                                         let rate = trade.get("rate").and_then(|v| v.as_str()).unwrap_or("0");
                                         let amount = trade.get("amount").and_then(|v| v.as_str()).unwrap_or("0");
                                         let side = trade.get("side").and_then(|v| v.as_str()).unwrap_or("?");
                                         let fee = trade.get("fee").and_then(|v| v.as_str()).unwrap_or("0");

                                         
                                         // Simple formatting
                                         message.push_str(format!("{} {} btc {} ฿{} fee {}", i+1, side, rate, amount, fee).as_str());
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
