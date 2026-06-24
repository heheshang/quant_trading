use exchange_okx::client::Client;
use exchange_okx::types::OkxEnvironment;
use tracing::info;

#[tokio::test]
async fn test_real_okx_client() {
    dotenv::dotenv().ok();

    let api_key = match std::env::var("OKX_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            info!("OKX_API_KEY not set, skipping test");
            return;
        }
    };

    let api_secret = match std::env::var("OKX_API_SECRET") {
        Ok(secret) => secret,
        Err(_) => {
            info!("OKX_API_SECRET not set, skipping test");
            return;
        }
    };

    let passphrase = match std::env::var("OKX_PASSPHRASE") {
        Ok(pass) => pass,
        Err(_) => {
            info!("OKX_PASSPHRASE not set, skipping test");
            return;
        }
    };

    let client = Client::new(api_key, api_secret, passphrase, OkxEnvironment::Demo);

    match client {
        Ok(client) => {
            info!("OKX client created successfully");

            match client.get_account_balance(Some("USDT")).await {
                Ok(balances) => {
                    info!("Account balance retrieved: {} balances", balances.len());
                    for balance in balances {
                        info!(
                            "  {}: eq={}, avail={}",
                            balance.ccy, balance.eq, balance.avail_eq
                        );
                    }
                }
                Err(e) => {
                    info!("Failed to get account balance: {}", e);
                }
            }

            match client.get_instruments("SPOT").await {
                Ok(instruments) => {
                    info!("Instruments retrieved: {:?}", instruments);
                }
                Err(e) => {
                    info!("Failed to get instruments: {}", e);
                }
            }
        }
        Err(e) => {
            info!("Failed to create OKX client: {}", e);
        }
    }
}
