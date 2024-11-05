// session.rs

use crate::archiver::{
    error::{ArchiverError, Result},
    metrics::ApiMetrics,
    types::*,
};

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::collections::{HashMap, HashSet};
use tracing::{debug, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub user_preferences: UserPreferences,
    pub charts: HashSet<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub timezone: String,
    pub default_chart_settings: ChartPreferences,
    #[serde(with = "duration_serde")]
    pub refresh_interval: Duration,
    pub max_points_per_chart: usize,
    pub auto_reconnect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartPreferences {
    #[serde(with = "duration_serde")]
    pub default_time_range: Duration,
    pub show_grid: bool,
    pub show_legend: bool,
    pub auto_scale: bool,
    pub default_operator: String,
    pub line_style: LineStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: Arc<DashMap<Uuid, Session>>,
    timeout: Duration,
    max_sessions: usize,
    metrics: Arc<ApiMetrics>,
    cleanup_interval: Duration,
}

mod duration_serde {
    use chrono::Duration;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.num_seconds().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = i64::deserialize(deserializer)?;
        Ok(Duration::seconds(seconds))
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
            default_chart_settings: ChartPreferences::default(),
            refresh_interval: Duration::seconds(1),
            max_points_per_chart: 10000,
            auto_reconnect: true,
        }
    }
}

impl Default for ChartPreferences {
    fn default() -> Self {
        Self {
            default_time_range: Duration::hours(1),
            show_grid: true,
            show_legend: true,
            auto_scale: true,
            default_operator: "raw".to_string(),
            line_style: LineStyle::Solid,
        }
    }
}

impl SessionManager {
    pub fn new(max_sessions: usize, timeout: Duration) -> Self {
        let manager = Self {
            sessions: Arc::new(DashMap::new()),
            timeout,
            max_sessions,
            metrics: Arc::new(ApiMetrics::new()),
            cleanup_interval: Duration::minutes(5),
        };

        manager.start_cleanup_task();
        manager
    }

    pub async fn create_session(&self, preferences: Option<UserPreferences>) -> Result<Session> {
        if self.sessions.len() >= self.max_sessions {
            self.cleanup_expired_sessions().await?;
            
            if self.sessions.len() >= self.max_sessions {
                return Err(ArchiverError::ServerError {
                    message: "Maximum number of sessions reached".into(),
                    status: 503,
                    body: None,
                    retry_after: Some(std::time::Duration::from_secs(60)),
                });
            }
        }

        let session = Session {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            last_active: Utc::now(),
            user_preferences: preferences.unwrap_or_default(),
            charts: HashSet::new(),
        };

        self.sessions.insert(session.id, session.clone());
        Ok(session)
    }

    pub async fn get_session(&self, id: Uuid) -> Result<Session> {
        let mut entry = self.sessions.get_mut(&id)
            .ok_or_else(|| ArchiverError::ServerError {
                message: "Session not found".into(),
                status: 404,
                body: None,
                retry_after: None,
            })?;

        if self.is_session_expired(&entry) {
            self.sessions.remove(&id);
            return Err(ArchiverError::ServerError {
                message: "Session expired".into(),
                status: 401,
                body: None,
                retry_after: None,
            });
        }

        entry.last_active = Utc::now();
        Ok(entry.clone())
    }

    pub async fn update_session(&self, id: Uuid, preferences: UserPreferences) -> Result<Session> {
        let mut entry = self.sessions.get_mut(&id)
            .ok_or_else(|| ArchiverError::ServerError {
                message: "Session not found".into(),
                status: 404,
                body: None,
                retry_after: None,
            })?;

        entry.user_preferences = preferences;
        entry.last_active = Utc::now();
        Ok(entry.clone())
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<()> {
        let expired: Vec<_> = self.sessions.iter()
            .filter(|ref_multi| self.is_session_expired(ref_multi))
            .map(|ref_multi| ref_multi.id)
            .collect();

        for id in expired {
            self.sessions.remove(&id);
            debug!("Cleaned up expired session: {}", id);
        }

        Ok(())
    }

    fn is_session_expired(&self, session: &Session) -> bool {
        Utc::now() - session.last_active > self.timeout
    }

    fn start_cleanup_task(&self) {
        let sessions = self.sessions.clone();
        let timeout = self.timeout;
        let interval = self.cleanup_interval;
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(
                std::time::Duration::from_secs(interval.num_seconds() as u64)
            );

            loop {
                interval_timer.tick().await;
                let expired: Vec<_> = sessions.iter()
                    .filter(|session| {
                        Utc::now() - session.last_active > timeout
                    })
                    .map(|session| session.id)
                    .collect();

                for id in expired {
                    if sessions.remove(&id).is_some() {
                        metrics.record_session_expired();
                        debug!("Removed expired session: {}", id);
                    }
                }
            }
        });
    }

    pub fn get_active_session_count(&self) -> usize {
        self.sessions.len()
    }

    pub async fn destroy_session(&self, id: Uuid) -> Result<()> {
        self.sessions.remove(&id)
            .ok_or_else(|| ArchiverError::ServerError {
                message: "Session not found".into(),
                status: 404,
                body: None,
                retry_after: None,
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let manager = SessionManager::new(
            10,
            Duration::minutes(30)
        );

        // Create session
        let session = manager.create_session(None).await.unwrap();
        assert!(manager.sessions.contains_key(&session.id));

        // Get session
        let retrieved = manager.get_session(session.id).await.unwrap();
        assert_eq!(retrieved.id, session.id);

        // Update preferences
        let new_prefs = UserPreferences {
            timezone: "America/Los_Angeles".to_string(),
            ..Default::default()
        };
        let updated = manager.update_session(session.id, new_prefs.clone()).await.unwrap();
        assert_eq!(updated.user_preferences.timezone, "America/Los_Angeles");
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let manager = SessionManager::new(
            10,
            Duration::seconds(1)
        );

        let session = manager.create_session(None).await.unwrap();
        tokio::time::sleep(StdDuration::from_secs(2)).await;
        
        let result = manager.get_session(session.id).await;
        assert!(result.is_err());
    }
}