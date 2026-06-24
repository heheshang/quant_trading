use exchange_okx::client::Client;
use exchange_okx::types::OkxEnvironment;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    dotenv::dotenv().ok();

    let api_key = std::env::var("OKX_API_KEY").expect("OKX_API_KEY not set");
    let api_secret = std::env::var("OKX_API_SECRET").expect("OKX_API_SECRET not set");
    let passphrase = std::env::var("OKX_PASSPHRASE").expect("OKX_PASSPHRASE not set");

    let client = Client::new(
        api_key,
        api_secret,
        passphrase,
        OkxEnvironment::Demo,
    )?;

    info!("OKX client created successfully");

    match client.get_account_balance(Some("USDT")).await {
        Ok(balances) => {
            info!("Account balances:");
            for balance in balances {
                info!(
                    "  {}: {} (available: {})",
                    balance.ccy, balance.eq, balance.avail_eq
                );
            }
        }
        Err(e) => error!("Failed to get account balance: {}", e),
    }

    match client.get_positions(None).await {
        Ok(positions) => {
            info!("Current positions:");
            for position in positions {
                info!(
                    "  {}: {} (avg price: {})",
                    position.inst_id, position.pos, position.avg_px
                );
            }
        }
        Err(e) => error!("Failed to get positions: {}", e),
    }

    match client.get_instruments("SPOT").await {
        Ok(instruments) => {
            info!("Available instruments: {:?}", instruments);
        }
        Err(e) => error!("Failed to get instruments: {}", e),
    }

    Ok(())
}
