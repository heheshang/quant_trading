use core::time::Duration;
use okx::websocket::{Args, ChannelType, OkxWebsocketClient};
use okx::Error;
use tokio::time::sleep;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::new().with_inst_id("BTC-USDT".to_string());
    let mut client = OkxWebsocketClient::new_public();
    let mut rx = client.connect().await.unwrap();
    client.subscribe(ChannelType::Tickers, args).await.unwrap();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            info!("收到公共频道消息: {:?}", msg);
        }
    });
    sleep(Duration::from_secs(100)).await;
    Ok(())
}
