use std::sync::Arc;

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

#[cfg(feature = "tauri")]
pub fn setup_app(app: &tauri::App, app_state: &Arc<crate::gateway::AppState>) {
    crate::infrastructure::log("SYSTEM", "Biosphere AI Lab 启动", None);

    let event_bus = app_state.event_bus.clone();
    setup_event_forwarding(app.handle(), &event_bus);
    crate::infrastructure::log("SYSTEM", "事件转发已设置", None);

    let state = app_state.clone();
    tauri::async_runtime::spawn(async move {
        state.recover_orphan_experiments().await;

        state.register_default_plugins().await;
        crate::infrastructure::log("SYSTEM", "默认插件注册完成", None);

        state.start_resource_monitor().await;
        crate::infrastructure::log("SYSTEM", "资源监控已启动", None);
    });
}
