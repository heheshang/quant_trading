use exchange_okx::client::Client;
use exchange_okx::types::OkxEnvironment;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Get OKX credentials from environment variables
    let api_key = std::env::var("OKX_API_KEY").expect("OKX_API_KEY not set");
    let api_secret = std::env::var("OKX_API_SECRET").expect("OKX_API_SECRET not set");
    let passphrase = std::env::var("OKX_PASSPHRASE").expect("OKX_PASSPHRASE not set");

    // Create OKX client
    let client = Client::new(
        api_key,
        api_secret,
        passphrase,
        OkxEnvironment::Demo, // Use demo environment for testing
    )?;

    println!("OKX client created successfully");

    // Get account balance
    match client.get_account_balance(Some("USDT")).await {
        Ok(balances) => {
            println!("Account balances:");
            for balance in balances {
                println!(
                    "  {}: {} (available: {})",
                    balance.ccy, balance.eq, balance.avail_eq
                );
            }
        }
        Err(e) => println!("Failed to get account balance: {}", e),
    }

    // Get positions
    match client.get_positions(None).await {
        Ok(positions) => {
            println!("Current positions:");
            for position in positions {
                println!(
                    "  {}: {} (avg price: {})",
                    position.inst_id, position.pos, position.avg_px
                );
            }
        }
        Err(e) => println!("Failed to get positions: {}", e),
    }

    // Get market instruments
    match client.get_instruments("SPOT").await {
        Ok(instruments) => {
            println!("Available instruments: {:?}", instruments);
        }
        Err(e) => println!("Failed to get instruments: {}", e),
    }

    // Example: Place a limit order (commented out to avoid actual trading)
    /*
    let order_request = OkxPlaceOrderRequest {
        inst_id: "BTC-USDT".to_string(),
        td_mode: "cash".to_string(),
        side: "buy".to_string(),
        ord_type: "limit".to_string(),
        sz: "0.001".to_string(),
        px: Some("10000".to_string()),
        cl_ord_id: None,
        tag: None,
        pos_side: None,
        ccy: None,
        px_usd: None,
        px_vol: None,
        reduce_only: None,
        tgt_ccy: None,
    };

    match client.place_order(order_request).await {
        Ok(order) => {
            println!("Order placed successfully: {:?}", order);
        }
        Err(e) => println!("Failed to place order: {}", e),
    }
    */

    Ok(())
}
