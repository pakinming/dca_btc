mod bitkub;
mod bot;
mod db;
mod models;

use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    println!(
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

        if amount > 0 {
            let pool_scheduler = pool.clone();
            let _ = sched
                .add(Job::new_async(cron_str.as_str(), move |_uuid, _l| {
                    let pool = pool_scheduler.clone();
                    Box::pin(async move {
                        println!("⏰ Executing Scheduled Buy Alert...");
                        match bitkub::place_bid(&pool, amount).await {
                            Ok(res) => println!("✅ Scheduled Buy Success: {:?}", res),
                            Err(e) => eprintln!("❌ Scheduled Buy Failed: {}", e),
                        }
                    })
                })?)
                .await?;
            sched.start().await?;
            println!(
                "📅 Schedule started with cron: {} for amount: {} THB",
                cron_str, amount
            );
        } else {
            println!("⚠️ Schedule enabled but amount is invalid: {}", amount);
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
