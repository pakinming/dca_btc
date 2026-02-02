mod bitkub;
mod bot;
mod db;
mod models;

use chrono_tz::Asia;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio_cron_scheduler::{Job, JobScheduler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!(
        "APP VERSION: {}",
        env::var("APP_VERSION").unwrap_or("0.0.0".to_string())
    );

    // Initialize DB Pool
    let pool = db::init_pool().await?;
    let pool = Arc::new(pool);

    // Schedule
    // Keep scheduler alive
    let _sched = if env::var("SCHEDULE_ENABLED").unwrap_or("false".to_string()) == "true" {
        let sched = JobScheduler::new().await?;
        let amount_str = env::var("SCHEDULE_AMOUNT").unwrap_or("0".to_string());
        let amount = amount_str.parse::<i32>().unwrap_or(0);
        // Default to every day at 08:00:00
        let cron_str = env::var("SCHEDULE_CRON").unwrap_or("0 0 8 * * *".to_string());
        tracing::debug!("cron_str = '{}'", &cron_str);
        tracing::info!("DateTime Local: {}", chrono::Local::now());
        tracing::info!("DateTime UTC: {}", chrono::Utc::now());

        if amount > 0 {
            let pool_scheduler = pool.clone();
            let _ = sched
                .add(Job::new_async_tz(
                    cron_str.as_str(),
                    Asia::Bangkok,
                    move |_uuid, _l| {
                        let pool = pool_scheduler.clone();
                        Box::pin(async move {
                            tracing::info!("⏰ Executing Scheduled Buy Alert...");
                            match bitkub::process_buy_limit(&pool, amount).await {
                                Ok((_msg, res)) => {
                                    tracing::info!("✅ Scheduled Buy Success: {:?}", res);
                                    
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
                                                            Ok(_) => tracing::info!("✅ SCHEDULED: Updated trade receive amount for ID: {}", trade.id),
                                                            Err(e) => tracing::error!("❌ SCHEDULED: Failed to update trade receive amount for ID: {}", trade.id),
                                                        }                                                        
                                                     }
                                                 }
                                             }
                                             _ => tracing::warn!("⚠️ SCHEDULED: Could not match latest trade to update receive amount."),
                                         }
                                    }
                                }
                                Err(e) => tracing::error!("❌ SCHEDULED: Scheduled Buy Failed: {}", e),
                            }
                        })
                    },
                )?)
                .await?;
            sched.start().await?;
            tracing::info!(
                "📅 Schedule started with cron: {} for amount: {} THB",
                cron_str,
                amount
            );
        } else {
            tracing::warn!("⚠️ Schedule enabled but amount is invalid: {}", amount);
        }
        Some(sched)
    } else {
        None
    };

    // Run Bot
    bot::run_bot(pool).await;

    Ok(())
}

// ✅ Success! Order Details: Some(Object {"amt": Number(20), "ci": String(""), "cre": Number(0), "fee": Number(0), "id": String("697ef1453f0fe25d5eada1ffm8a2qe"), "rat": Number(0), "rec": Number(0), "ts": String("1769926981"), "typ": String("market")})
