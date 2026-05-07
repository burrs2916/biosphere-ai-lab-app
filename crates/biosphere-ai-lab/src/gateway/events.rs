use crate::core::EventBus;

#[cfg(feature = "tauri")]
pub fn setup_event_forwarding(app: &tauri::AppHandle, event_bus: &EventBus) {
    use tauri::Emitter;

    let mut rx = event_bus.subscribe();
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let _ = app_handle.emit("lab-event", &event);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Event bus lagged, skipped {} events", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });
}
