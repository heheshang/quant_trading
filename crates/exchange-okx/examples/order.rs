use okx::api::api_trait::OkxApiTrait;
use okx::config::Credentials;
use okx::OkxClient;
use okx::{Error, OkxTrade};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let credentials = Credentials::from_env().unwrap();
    let client: OkxClient = OkxClient::new(credentials).unwrap();

    let trade = OkxTrade::new(client.clone());
    let okx_ord_id = "3055268924256178176";
    let ticker = trade
        .get_order_details("BTC-USDT-SWAP", Some(okx_ord_id), None)
        .await;
    info!("order 行情: {:#?}", ticker);

    let int_ord_id = "3055268924256178176";
    let ticker = trade
        .get_order_details("BTC-USDT-SWAP", None, Some(int_ord_id))
        .await?;
    info!("order 行情: {:#?}", ticker);

    let okx_order_id = "3055300031160799232";
    let ticker = trade
        .cancel_order("BTC-USDT-SWAP", Some(okx_order_id), None)
        .await;
    info!("order 行情: {:#?}", ticker);

    Ok(())
}
