use okx::api::api_trait::OkxApiTrait;
use okx::config::Credentials;
use okx::{Error, OkxClient, OkxMarket, OkxTrade};
use serde_json::json;
#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    let credentials = Credentials::from_env().unwrap(); // 初始化客户端
    let client: OkxClient = OkxClient::new(credentials).unwrap();

    let trade = OkxTrade::new(client.clone());
    let okx_ord_id = "3055268924256178176";
    // 获取BTC-USDT的产品行情
    let ticker = trade
        .get_order_details("BTC-USDT-SWAP", Some(okx_ord_id), None)
        .await;
    println!("order 行情: {:#?}", ticker);

    let int_ord_id = "3055268924256178176";
    // 获取BTC-USDT的产品行情
    let ticker = trade
        .get_order_details("BTC-USDT-SWAP", None, Some(int_ord_id))
        .await?;
    println!("order 行情: {:#?}", ticker);

    let okx_order_id = "3055300031160799232";
    let ticker = trade
        .cancel_order("BTC-USDT-SWAP", Some(okx_order_id), None)
        .await;
    println!("order 行情: {:#?}", ticker);

    Ok(())
}
