pub mod client;
pub mod types;
pub mod websocket;

pub use client::{Client, ClientInterface};
pub use websocket::OkxWebSocket;

// Re-export OKX SDK types used in trait signatures so external crates (e.g. src-tauri tests) can construct them.
pub use okx::api::announcements::announcements_api::{AnnouncementDetail, AnnouncementPage};

#[cfg(test)]
pub mod mock_data;
#[cfg(any(test, feature = "test-utils"))]
pub use client::MockClientInterface as MockOkxClient;
