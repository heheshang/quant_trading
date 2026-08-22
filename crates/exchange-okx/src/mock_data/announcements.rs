//! Mock data for announcement types.

use okx::api::announcements::announcements_api::{AnnouncementDetail, AnnouncementPage};

/// Create a default [`AnnouncementDetail`] for testing.
pub fn mock_announcement_detail() -> AnnouncementDetail {
    AnnouncementDetail {
        ann_type: "delisting".to_string(),
        p_time: "1597026383085".to_string(),
        title: "Test Announcement".to_string(),
        url: "https://www.okx.com/support/announcement/test".to_string(),
    }
}

/// Create a default [`AnnouncementPage`] for testing.
pub fn mock_announcement_page() -> AnnouncementPage {
    AnnouncementPage {
        details: vec![
            mock_announcement_detail(),
            AnnouncementDetail {
                ann_type: "listing".to_string(),
                p_time: "1597026383086".to_string(),
                title: "New Token Listing".to_string(),
                url: "https://www.okx.com/support/announcement/listing".to_string(),
            },
        ],
        total_page: "1".to_string(),
    }
}
