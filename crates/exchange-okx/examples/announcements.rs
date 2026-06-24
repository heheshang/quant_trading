use okx::api::announcements::announcements_api::OkxAnnouncements;
use okx::Error;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let announcements = OkxAnnouncements::from_env()
        .unwrap()
        .get_announcements(None, None, None)
        .await?;
    info!("公告: {:?}", announcements);

    Ok(())
}
