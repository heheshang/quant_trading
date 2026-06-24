use okx::api::api_trait::OkxApiTrait;
use okx::config::Credentials;
use okx::{Error, OkxClient, OkxMarket};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let credentials = Credentials::from_env().unwrap();

    let client: OkxClient = OkxClient::new(credentials).unwrap();

    let market = OkxMarket::new(client.clone());
    let ticker = market.get_ticker("BTC-USDT-SWAP").await?;
    info!("BTC-USDT 行情: {:?}", ticker);
    Ok(())
}
