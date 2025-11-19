use okx::config::Credentials;
use okx::{Error, OkxAsset, OkxClient};
use okx::api::api_trait::OkxApiTrait;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Error> {

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(&"info")
        }))
        .init();

    info!("Starting asset example");
    let credentials = Credentials::from_env().unwrap();// 初始化客户端
    info!("Credentials initialized {:?}", credentials);
    let client: OkxClient = OkxClient::new(credentials).unwrap();
    //获取asset账户余额
    let balances = OkxAsset::new(client).get_balances(None).await?;
    println!("账户余额: {:?}", balances);

    Ok(())
}