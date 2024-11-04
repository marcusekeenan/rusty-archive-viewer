// session.rs

use crate::{
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
    pub refresh_interval: Duration,
    pub max_points_per_chart: usize,
    pub auto_reconnect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartPreferences {
    pub default_time_range: Duration,
    pub show_grid: bool,
    pub show_legend: bool,
    pub auto_scale: bool,
    pub default_operator: String,
    pub line_style: LineStyle,
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: Arc<DashMap<Uuid, Session>>,
    timeout: Duration,
    max_sessions: usize,
    metrics: Arc<ApiMetrics>,
    cleanup_interval: Duration,
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
                return Err(ArchiverError::SessionError {
                    message: "Maximum number of sessions reached".into(),
                    context: format!("max_sessions: {}", self.max_sessions),
                    source: None,
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
        self.metrics.record_session_created();
        
        debug!("Created new session: {}", session.id);
        Ok(session)
    }

    pub async fn get_session(&self, id: Uuid) -> Result<Session> {
        let mut session = self.sessions.get_mut(&id)
            .ok_or_else(|| ArchiverError::SessionError {
                message: "Session not found".into(),
                context: format!("session_id: {}", id),
                source: None,
            })?;

        if self.is_session_expired(&session) {
            self.sessions.remove(&id);
            return Err(ArchiverError::SessionError {
                message: "Session expired".into(),
                context: format!("session_id: {}", id),
                source: None,
            });
        }

        session.last_active = Utc::now();
        Ok(session.clone())
    }

    pub async fn update_session(&self, id: Uuid, preferences: UserPreferences) -> Result<Session> {
        let mut session = self.sessions.get_mut(&id)
            .ok_or_else(|| ArchiverError::SessionError {
                message: "Session not found".into(),
                context: format!("session_id: {}", id),
                source: None,
            })?;

        session.user_preferences = preferences;
        session.last_active = Utc::now();
        
        Ok(session.clone())
    }

    pub async fn add_chart_to_session(&self, session_id: Uuid, chart_id: Uuid) -> Result<()> {
        let mut session = self.sessions.get_mut(&session_id)
            .ok_or_else(|| ArchiverError::SessionError {
                message: "Session not found".into(),
                context: format!("session_id: {}", session_id),
                source: None,
            })?;

        session.charts.insert(chart_id);
        session.last_active = Utc::now();
        Ok(())
    }

    pub async fn remove_chart_from_session(&self, session_id: Uuid, chart_id: Uuid) -> Result<()> {
        let mut session = self.sessions.get_mut(&session_id)
            .ok_or_else(|| ArchiverError::SessionError {
                message: "Session not found".into(),
                context: format!("session_id: {}", session_id),
                source: None,
            })?;

        session.charts.remove(&chart_id);
        session.last_active = Utc::now();
        Ok(())
    }

    fn start_cleanup_task(&self) {
        let sessions = self.sessions.clone();
        let metrics = self.metrics.clone();
        let timeout = self.timeout;
        let interval = self.cleanup_interval;

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

    async fn cleanup_expired_sessions(&self) -> Result<()> {
        let expired: Vec<_> = self.sessions.iter()
            .filter(|session| self.is_session_expired(&session))
            .map(|session| session.id)
            .collect();

        for id in expired {
            if self.sessions.remove(&id).is_some() {
                self.metrics.record_session_expired();
                debug!("Cleaned up expired session: {}", id);
            }
        }

        Ok(())
    }

    fn is_session_expired(&self, session: &Session) -> bool {
        Utc::now() - session.last_active > self.timeout
    }

    pub fn get_active_session_count(&self) -> usize {
        self.sessions.len()
    }

    pub async fn get_session_metrics(&self) -> SessionMetrics {
        SessionMetrics {
            active_sessions: self.sessions.len(),
            expired_sessions: self.metrics.get_expired_session_count(),
            avg_session_duration: self.calculate_avg_session_duration(),
        }
    }

    fn calculate_avg_session_duration(&self) -> Duration {
        let total_duration: i64 = self.sessions.iter()
            .map(|session| {
                (session.last_active - session.created_at).num_seconds()
            })
            .sum();

        if self.sessions.is_empty() {
            Duration::seconds(0)
        } else {
            Duration::seconds(total_duration / self.sessions.len() as i64)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub active_sessions: usize,
    pub expired_sessions: u64,
    pub avg_session_duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
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

        // Add chart
        let chart_id = Uuid::new_v4();
        manager.add_chart_to_session(session.id, chart_id).await.unwrap();
        let session = manager.get_session(session.id).await.unwrap();
        assert!(session.charts.contains(&chart_id));
    }

    #[test]
    async fn test_session_expiration() {
        let manager = SessionManager::new(
            10,
            Duration::seconds(1) // Short timeout for testing
        );

        let session = manager.create_session(None).await.unwrap();
        
        // Wait for expiration
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        
        let result = manager.get_session(session.id).await;
        assert!(result.is_err());
    }
}