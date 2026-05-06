use std::cell::RefCell;

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

/// Wrapper around ratatui-image's StatefulProtocol for rendering cover art.
/// Uses RefCell because ratatui's render_stateful_widget needs &mut,
/// but all our render functions take &App.
pub struct CoverArt {
    pub protocol: RefCell<StatefulProtocol>,
}

/// Query the terminal for graphics protocol support and font size.
/// Must be called BEFORE enable_raw_mode().
pub fn create_picker() -> Option<Picker> {
    Picker::from_query_stdio().ok()
}

/// Fetch an image from a URL and create a StatefulProtocol using the picker.
pub async fn fetch_cover(url: &str, picker: &Picker) -> Option<CoverArt> {
    let bytes = reqwest::get(url).await.ok()?.bytes().await.ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let protocol = picker.new_resize_protocol(img);
    Some(CoverArt {
        protocol: RefCell::new(protocol),
    })
}
