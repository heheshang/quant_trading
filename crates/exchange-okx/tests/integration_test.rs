use exchange_okx::client::Client;
use exchange_okx::types::OkxEnvironment;

#[tokio::test]
async fn test_real_okx_client() {
    // Load environment variables
    dotenv::dotenv().ok();

    // Check if environment variables are set
    let api_key = match std::env::var("OKX_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("OKX_API_KEY not set, skipping test");
            return;
        }
    };

    let api_secret = match std::env::var("OKX_API_SECRET") {
        Ok(secret) => secret,
        Err(_) => {
            println!("OKX_API_SECRET not set, skipping test");
            return;
        }
    };

    let passphrase = match std::env::var("OKX_PASSPHRASE") {
        Ok(pass) => pass,
        Err(_) => {
            println!("OKX_PASSPHRASE not set, skipping test");
            return;
        }
    };

    // Create client
    let client = Client::new(api_key, api_secret, passphrase, OkxEnvironment::Demo);

    match client {
        Ok(client) => {
            println!("OKX client created successfully");

            // Test get account balance
            match client.get_account_balance(Some("USDT")).await {
                Ok(balances) => {
                    println!("Account balance retrieved: {} balances", balances.len());
                    for balance in balances {
                        println!(
                            "  {}: eq={}, avail={}",
                            balance.ccy, balance.eq, balance.avail_eq
                        );
                    }
                }
                Err(e) => {
                    println!("Failed to get account balance: {}", e);
                }
            }

            // Test get instruments
            match client.get_instruments("SPOT").await {
                Ok(instruments) => {
                    println!("Instruments retrieved: {:?}", instruments);
                }
                Err(e) => {
                    println!("Failed to get instruments: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Failed to create OKX client: {}", e);
        }
    }
}
