use okx::api::api_trait::OkxApiTrait;
use okx::config::Credentials;
use okx::{Error, OkxAsset, OkxClient};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting asset example");
    let credentials = Credentials::from_env().unwrap();
    info!("Credentials initialized {:?}", credentials);
    let client: OkxClient = OkxClient::new(credentials).unwrap();
    let balances = OkxAsset::new(client)
        .get_balances(Some(&vec!["BTC".to_string()]))
        .await?;
    info!("账户余额: {:?}", balances);

    Ok(())
}
