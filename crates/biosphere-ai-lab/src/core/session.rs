use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::event::EventBus;
use crate::core::config::TrainingConfig;
use crate::types::{SessionId, SessionStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub name: String,
    pub status: SessionStatus,
    pub config: TrainingConfig,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub current_epoch: usize,
    pub total_epochs: usize,
    pub error_message: Option<String>,
}

pub struct Session {
    id: SessionId,
    status: Arc<RwLock<SessionStatus>>,
    config: TrainingConfig,
    event_bus: Arc<EventBus>,
    created_at: DateTime<Utc>,
    started_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    completed_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    current_epoch: Arc<RwLock<usize>>,
    error_message: Arc<RwLock<Option<String>>>,
}

impl Session {
    pub fn new(config: TrainingConfig, event_bus: Arc<EventBus>) -> Self {
        let id = SessionId::new();
        let session_id_for_event = id.clone();

        let session = Self {
            id,
            status: Arc::new(RwLock::new(SessionStatus::Created)),
            config,
            event_bus,
            created_at: Utc::now(),
            started_at: Arc::new(RwLock::new(None)),
            completed_at: Arc::new(RwLock::new(None)),
            current_epoch: Arc::new(RwLock::new(0)),
            error_message: Arc::new(RwLock::new(None)),
        };

        session.event_bus.emit(LabEvent::SessionCreated {
            session_id: session_id_for_event,
        });

        session
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub async fn status(&self) -> SessionStatus {
        *self.status.read().await
    }

    pub fn config(&self) -> &TrainingConfig {
        &self.config
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub async fn set_status(&self, new_status: SessionStatus) {
        let mut status = self.status.write().await;
        *status = new_status;
    }

    pub async fn start(&self) {
        let mut started = self.started_at.write().await;
        *started = Some(Utc::now());
        drop(started);

        let mut status = self.status.write().await;
        *status = SessionStatus::Training;
        drop(status);

        self.event_bus.emit(LabEvent::SessionStarted {
            session_id: self.id.clone(),
        });
    }

    pub async fn pause(&self) {
        let mut status = self.status.write().await;
        *status = SessionStatus::Paused;
        drop(status);

        self.event_bus.emit(LabEvent::SessionPaused {
            session_id: self.id.clone(),
        });
    }

    pub async fn resume(&self) {
        let mut status = self.status.write().await;
        *status = SessionStatus::Training;
        drop(status);

        self.event_bus.emit(LabEvent::SessionResumed {
            session_id: self.id.clone(),
        });
    }

    pub async fn complete(&self, final_metrics: serde_json::Value) {
        let mut completed = self.completed_at.write().await;
        *completed = Some(Utc::now());
        drop(completed);

        let mut status = self.status.write().await;
        *status = SessionStatus::Completed;
        drop(status);

        self.event_bus.emit(LabEvent::SessionCompleted {
            session_id: self.id.clone(),
            final_metrics,
        });
    }

    pub async fn fail(&self, error: String) {
        let mut err = self.error_message.write().await;
        *err = Some(error.clone());
        drop(err);

        let mut status = self.status.write().await;
        *status = SessionStatus::Failed;
        drop(status);

        self.event_bus.emit(LabEvent::SessionFailed {
            session_id: self.id.clone(),
            error,
        });
    }

    pub async fn cancel(&self) {
        let mut status = self.status.write().await;
        *status = SessionStatus::Cancelled;
        drop(status);

        self.event_bus.emit(LabEvent::SessionCancelled {
            session_id: self.id.clone(),
        });
    }

    pub async fn set_epoch(&self, epoch: usize) {
        let mut current = self.current_epoch.write().await;
        *current = epoch;
    }

    pub async fn current_epoch(&self) -> usize {
        *self.current_epoch.read().await
    }

    pub async fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            name: self.config.session_name.clone(),
            status: *self.status.read().await,
            config: self.config.clone(),
            created_at: self.created_at,
            started_at: *self.started_at.read().await,
            completed_at: *self.completed_at.read().await,
            current_epoch: *self.current_epoch.read().await,
            total_epochs: self.config.epochs,
            error_message: self.error_message.read().await.clone(),
        }
    }
}

use crate::core::event::LabEvent;
