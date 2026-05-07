use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::core::{LabError, Plugin, PluginKind, Result};
use crate::core::config::TrainingConfig;
use crate::core::event::EventBus;
use crate::types::{ComputeBackend, PluginId, SessionId, SessionStatus};
use super::{Engine, SessionHandle};
use super::burn_training::{run_training, run_training_from_checkpoint, TrainControl};

#[allow(dead_code)]
struct SessionState {
    status: SessionStatus,
    config: TrainingConfig,
    current_epoch: usize,
    train_control: Arc<TrainControl>,
}

pub struct BurnEngine {
    id: PluginId,
    event_bus: Option<Arc<EventBus>>,
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
}

impl BurnEngine {
    pub fn new() -> Self {
        Self {
            id: PluginId::new("burn"),
            event_bus: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_event_bus(mut self, event_bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub fn set_event_bus(&mut self, event_bus: Arc<EventBus>) {
        self.event_bus = Some(event_bus);
    }
}

impl Plugin for BurnEngine {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Burn Engine"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Burn deep learning framework engine with WGPU and Metal backend support"
    }

    fn plugin_kind(&self) -> PluginKind {
        PluginKind::Engine
    }
}

#[async_trait]
impl Engine for BurnEngine {
    fn supported_backends(&self) -> Vec<ComputeBackend> {
        vec![
            ComputeBackend::Cpu,
            ComputeBackend::Wgpu,
            #[cfg(target_os = "macos")]
            ComputeBackend::Metal,
        ]
    }

    async fn create_session(&self, config: &TrainingConfig) -> Result<SessionHandle> {
        let session_id = SessionId::new();
        let handle = SessionHandle {
            session_id: session_id.clone(),
        };

        let train_control = Arc::new(TrainControl::new());

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), SessionState {
            status: SessionStatus::Created,
            config: config.clone(),
            current_epoch: 0,
            train_control,
        });

        Ok(handle)
    }

    async fn start(&self, handle: SessionHandle) -> Result<()> {
        let sid = handle.session_id.to_string();

        let (config, train_control) = {
            let mut sessions = self.sessions.write().await;
            let state = sessions.get_mut(&sid)
                .ok_or_else(|| LabError::Custom(format!("Session not found: {}", sid)))?;

            if state.status != SessionStatus::Created && state.status != SessionStatus::Paused {
                return Err(LabError::Custom(format!(
                    "Cannot start session in {} state", state.status
                )));
            }

            state.status = SessionStatus::Training;
            (state.config.clone(), state.train_control.clone())
        };

        if let Some(ref event_bus) = self.event_bus {
            event_bus.emit(crate::core::event::LabEvent::SessionStarted {
                session_id: handle.session_id.clone(),
            });
        }

        let sessions = self.sessions.clone();
        let event_bus = self.event_bus.clone();
        let session_id = handle.session_id.clone();

        tokio::spawn(async move {
            let result = Self::run_training(
                &session_id,
                &config,
                &sessions,
                event_bus.clone(),
                train_control,
            ).await;

            let final_status = match &result {
                Ok(()) => SessionStatus::Completed,
                Err(LabError::TrainingFailed(msg)) if msg.contains("cancelled") => SessionStatus::Cancelled,
                Err(_) => SessionStatus::Failed,
            };

            {
                let mut sessions_guard = sessions.write().await;
                if let Some(state) = sessions_guard.get_mut(&session_id.to_string()) {
                    if state.status != SessionStatus::Cancelled {
                        state.status = final_status;
                    }
                }
            }

            if let Some(ref event_bus) = event_bus {
                let current_status = {
                    let sessions_guard = sessions.read().await;
                    sessions_guard.get(&session_id.to_string())
                        .map(|s| s.status)
                        .unwrap_or(SessionStatus::Failed)
                };

                match current_status {
                    SessionStatus::Cancelled => {
                        event_bus.emit(crate::core::event::LabEvent::SessionCancelled {
                            session_id: session_id.clone(),
                        });
                    }
                    SessionStatus::Completed => {
                        event_bus.emit(crate::core::event::LabEvent::SessionCompleted {
                            session_id: session_id.clone(),
                            final_metrics: serde_json::json!({}),
                        });
                    }
                    _ => {
                        event_bus.emit(crate::core::event::LabEvent::SessionFailed {
                            session_id: session_id.clone(),
                            error: result.err().map(|e| e.to_string()).unwrap_or_default(),
                        });
                    }
                }
            }
        });

        Ok(())
    }

    async fn pause(&self, handle: SessionHandle) -> Result<()> {
        let sid = handle.session_id.to_string();
        let mut sessions = self.sessions.write().await;
        let state = sessions.get_mut(&sid)
            .ok_or_else(|| LabError::Custom(format!("Session not found: {}", sid)))?;

        if state.status != SessionStatus::Training {
            return Err(LabError::Custom(format!(
                "Cannot pause session in {} state", state.status
            )));
        }

        state.train_control.pause();
        state.status = SessionStatus::Paused;

        if let Some(ref event_bus) = self.event_bus {
            event_bus.emit(crate::core::event::LabEvent::SessionPaused {
                session_id: handle.session_id.clone(),
            });
        }

        Ok(())
    }

    async fn resume(&self, handle: SessionHandle) -> Result<()> {
        let sid = handle.session_id.to_string();
        let mut sessions = self.sessions.write().await;
        let state = sessions.get_mut(&sid)
            .ok_or_else(|| LabError::Custom(format!("Session not found: {}", sid)))?;

        if state.status != SessionStatus::Paused {
            return Err(LabError::Custom(format!(
                "Cannot resume session in {} state", state.status
            )));
        }

        state.train_control.resume();
        state.status = SessionStatus::Training;

        if let Some(ref event_bus) = self.event_bus {
            event_bus.emit(crate::core::event::LabEvent::SessionResumed {
                session_id: handle.session_id.clone(),
            });
        }

        Ok(())
    }

    async fn stop(&self, handle: SessionHandle) -> Result<()> {
        let sid = handle.session_id.to_string();
        let mut sessions = self.sessions.write().await;
        if let Some(state) = sessions.get_mut(&sid) {
            if state.status == SessionStatus::Cancelled {
                return Ok(());
            }
            state.train_control.cancel();
            state.status = SessionStatus::Cancelled;
        }
        Ok(())
    }

    fn get_status(&self, handle: &SessionHandle) -> SessionStatus {
        let sid = handle.session_id.to_string();
        match self.sessions.try_read() {
            Ok(sessions) => {
                sessions.get(&sid)
                    .map(|s| s.status)
                    .unwrap_or(SessionStatus::Created)
            }
            Err(_) => SessionStatus::Created,
        }
    }

    async fn start_from_checkpoint(&self, handle: SessionHandle, checkpoint_epoch: usize, artifact_dir: &str) -> Result<()> {
        let sid = handle.session_id.to_string();

        let (config, train_control) = {
            let mut sessions = self.sessions.write().await;
            let state = sessions.get_mut(&sid)
                .ok_or_else(|| LabError::Custom(format!("Session not found: {}", sid)))?;

            state.status = SessionStatus::Training;
            (state.config.clone(), state.train_control.clone())
        };

        if let Some(ref event_bus) = self.event_bus {
            event_bus.emit(crate::core::event::LabEvent::SessionStarted {
                session_id: handle.session_id.clone(),
            });
        }

        let sessions = self.sessions.clone();
        let event_bus = self.event_bus.clone();
        let session_id = handle.session_id.clone();
        let artifact_dir = artifact_dir.to_string();

        tokio::spawn(async move {
            let result = Self::run_training_from_checkpoint_task(
                &session_id,
                &config,
                &sessions,
                event_bus.clone(),
                train_control,
                checkpoint_epoch,
                artifact_dir.clone(),
            ).await;

            let final_status = match &result {
                Ok(()) => SessionStatus::Completed,
                Err(LabError::TrainingFailed(msg)) if msg.contains("cancelled") => SessionStatus::Cancelled,
                Err(_) => SessionStatus::Failed,
            };

            {
                let mut sessions_guard = sessions.write().await;
                if let Some(state) = sessions_guard.get_mut(&session_id.to_string()) {
                    if state.status != SessionStatus::Cancelled {
                        state.status = final_status;
                    }
                }
            }

            if let Some(ref event_bus) = event_bus {
                let current_status = {
                    let sessions_guard = sessions.read().await;
                    sessions_guard.get(&session_id.to_string())
                        .map(|s| s.status)
                        .unwrap_or(SessionStatus::Failed)
                };

                match current_status {
                    SessionStatus::Cancelled => {
                        event_bus.emit(crate::core::event::LabEvent::SessionCancelled {
                            session_id: session_id.clone(),
                        });
                    }
                    SessionStatus::Completed => {
                        event_bus.emit(crate::core::event::LabEvent::SessionCompleted {
                            session_id: session_id.clone(),
                            final_metrics: serde_json::json!({}),
                        });
                    }
                    _ => {
                        event_bus.emit(crate::core::event::LabEvent::SessionFailed {
                            session_id: session_id.clone(),
                            error: result.err().map(|e| e.to_string()).unwrap_or_default(),
                        });
                    }
                }
            }
        });

        Ok(())
    }

    async fn run_inference(&self, config: &TrainingConfig, artifact_dir: &str, input_data: &[Vec<f32>]) -> Result<super::InferenceOutput> {
        let result = super::burn_training::run_inference(config, artifact_dir, input_data)?;
        Ok(super::InferenceOutput {
            predictions: result.predictions,
            predicted_classes: result.predicted_classes,
            probabilities: result.probabilities,
        })
    }
}

impl BurnEngine {
    async fn run_training(
        session_id: &SessionId,
        config: &TrainingConfig,
        sessions: &Arc<RwLock<HashMap<String, SessionState>>>,
        event_bus: Option<Arc<EventBus>>,
        train_control: Arc<TrainControl>,
    ) -> crate::core::Result<()> {
        let event_bus = match event_bus {
            Some(eb) => eb,
            None => return Err(LabError::Custom("No event bus configured".to_string())),
        };

        {
            let mut sessions_guard = sessions.write().await;
            if let Some(state) = sessions_guard.get_mut(&session_id.to_string()) {
                state.status = SessionStatus::LoadingData;
            }
        }

        let artifact_dir = crate::core::config::get_artifact_dir(&session_id.to_string());

        let eb = event_bus.clone();
        let sid = session_id.clone();
        let config_clone = config.clone();
        let tc = train_control.clone();

        let result = tokio::task::spawn_blocking(move || {
            if tc.is_cancelled() {
                return Err(LabError::TrainingFailed("Training cancelled".to_string()));
            }

            run_training(
                eb,
                sid,
                &config_clone,
                &artifact_dir,
                tc,
            )
        })
        .await
        .map_err(|e| LabError::TrainingFailed(format!("Training task panicked: {}", e)))?
        .map_err(|e| LabError::TrainingFailed(e.to_string()));

        if train_control.is_cancelled() {
            return Err(LabError::TrainingFailed("Training cancelled".to_string()));
        }

        result
    }

    async fn run_training_from_checkpoint_task(
        session_id: &SessionId,
        config: &TrainingConfig,
        sessions: &Arc<RwLock<HashMap<String, SessionState>>>,
        event_bus: Option<Arc<EventBus>>,
        train_control: Arc<TrainControl>,
        checkpoint_epoch: usize,
        artifact_dir: String,
    ) -> crate::core::Result<()> {
        let event_bus = match event_bus {
            Some(eb) => eb,
            None => return Err(LabError::Custom("No event bus configured".to_string())),
        };

        {
            let mut sessions_guard = sessions.write().await;
            if let Some(state) = sessions_guard.get_mut(&session_id.to_string()) {
                state.status = SessionStatus::LoadingData;
            }
        }

        let eb = event_bus.clone();
        let sid = session_id.clone();
        let config_clone = config.clone();
        let tc = train_control.clone();

        let result = tokio::task::spawn_blocking(move || {
            if tc.is_cancelled() {
                return Err(LabError::TrainingFailed("Training cancelled".to_string()));
            }

            run_training_from_checkpoint(
                eb,
                sid,
                &config_clone,
                &artifact_dir,
                checkpoint_epoch,
                tc,
            )
        })
        .await
        .map_err(|e| LabError::TrainingFailed(format!("Training task panicked: {}", e)))?
        .map_err(|e| LabError::TrainingFailed(e.to_string()));

        if train_control.is_cancelled() {
            return Err(LabError::TrainingFailed("Training cancelled".to_string()));
        }

        result
    }
}

impl Default for BurnEngine {
    fn default() -> Self {
        Self::new()
    }
}
