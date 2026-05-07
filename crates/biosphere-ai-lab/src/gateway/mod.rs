pub mod tauri_adapter;
pub mod state;
pub mod init;

pub use state::AppState;

#[cfg(feature = "tauri")]
pub use init::setup_event_forwarding;

#[cfg(feature = "tauri")]
pub use init::setup_app;
