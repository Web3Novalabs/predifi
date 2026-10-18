//! Hardened session management for authenticated users.
//!
//! Features:
//! - Session fixation prevention (rotate session id on authenticate)
//! - Idle timeout enforcement
//! - Concurrent session limits per user
//! - Session invalidation on password/key changes
//! - Session activity logging for audit trails

use std::collections::{hash_map::DefaultHasher, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, StatusCode};
use http::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};

/// Default idle timeout (30 minutes).
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 30 * 60;
/// Default maximum concurrent sessions per user.
pub const DEFAULT_MAX_SESSIONS_PER_USER: usize = 5;
/// Cap on retained activity log entries.
const ACTIVITY_LOG_CAPACITY: usize = 2_000;

/// Configuration for the session store.
#[derive(Debug, Clone, Copy)]
pub struct SessionConfig {
    pub idle_timeout: Duration,
    /// Values below one are treated as one so session creation cannot loop indefinitely.
    pub max_sessions_per_user: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS),
            max_sessions_per_user: DEFAULT_MAX_SESSIONS_PER_USER,
        }
    }
}

/// Represents an authenticated user session.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct UserSession {
    /// Opaque session identifier (rotated on login to prevent fixation).
    pub session_id: String,
    /// The principal identifier (wallet address, e.g. `G...`).
    pub user_address: String,
}

/// Internal stored session state.
#[derive(Debug, Clone)]
struct StoredSession {
    session_id: String,
    user_address: String,
    /// Credential/key version at creation time; bumping invalidates sessions.
    key_version: u64,
    created_at: Instant,
    last_activity: Instant,
}

/// Audit event kinds for session activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionActivityKind {
    Created,
    Authenticated,
    Renewed,
    IdleTimeout,
    ConcurrentLimitEvicted,
    InvalidatedKeyChange,
    Logout,
    RejectedFixation,
    RejectedMissing,
    RejectedExpired,
}

/// Single activity log entry.
#[derive(Debug, Clone, Serialize)]
pub struct SessionActivity {
    pub kind: SessionActivityKind,
    pub user_address: String,
    /// Non-secret fingerprint for correlating events; never the bearer token.
    pub session_id: Option<String>,
    pub detail: String,
    pub at_unix_ms: u64,
}

/// In-memory session store with hardening controls.
#[derive(Clone)]
pub struct SessionStore {
    inner: Arc<SessionStoreInner>,
}

struct SessionStoreInner {
    config: SessionConfig,
    state: Mutex<SessionState>,
    activity: Mutex<VecDeque<SessionActivity>>,
}

#[derive(Default)]
struct SessionState {
    /// session_id → session
    sessions: HashMap<String, StoredSession>,
    /// user_address → ordered session ids (oldest first)
    by_user: HashMap<String, VecDeque<String>>,
    /// user_address → credential/key version
    key_versions: HashMap<String, u64>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new(SessionConfig::default())
    }
}

impl SessionStore {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            inner: Arc::new(SessionStoreInner {
                config,
                state: Mutex::new(SessionState::default()),
                activity: Mutex::new(VecDeque::new()),
            }),
        }
    }

    /// Create a new session, rotating the session id (fixation prevention).
    ///
    /// If `previous_session_id` is provided (e.g. pre-auth cookie), it is
    /// discarded and a fresh id is issued. It is never accepted as the new
    /// session or used to revoke a possibly unrelated authenticated session.
    pub fn create_session(
        &self,
        user_address: &str,
        previous_session_id: Option<&str>,
    ) -> UserSession {
        if let Some(old) = previous_session_id {
            self.log(
                SessionActivityKind::RejectedFixation,
                user_address,
                Some(old),
                "pre-auth session id discarded; new id issued",
            );
        }

        let session_id = generate_session_id();
        let now = Instant::now();
        let mut evicted_ids = Vec::new();

        {
            let mut state = self.inner.state.lock().expect("session state");
            let stale_ids: Vec<_> = state
                .by_user
                .get(user_address)
                .into_iter()
                .flatten()
                .filter(|id| {
                    state.sessions.get(*id).map_or(true, |session| {
                        session.last_activity.elapsed() >= self.inner.config.idle_timeout
                    })
                })
                .cloned()
                .collect();
            for stale_id in stale_ids {
                remove_session(&mut state, &stale_id);
                evicted_ids.push((SessionActivityKind::IdleTimeout, stale_id));
            }

            // Enforce concurrent session limit (evict oldest).
            let limit = self.inner.config.max_sessions_per_user.max(1);
            loop {
                let evicted = state
                    .by_user
                    .get_mut(user_address)
                    .filter(|sessions| sessions.len() >= limit)
                    .and_then(VecDeque::pop_front);
                let Some(evicted_id) = evicted else {
                    break;
                };
                state.sessions.remove(&evicted_id);
                evicted_ids.push((SessionActivityKind::ConcurrentLimitEvicted, evicted_id));
            }

            let key_version = *state.key_versions.get(user_address).unwrap_or(&0);
            state.sessions.insert(
                session_id.clone(),
                StoredSession {
                    session_id: session_id.clone(),
                    user_address: user_address.to_string(),
                    key_version,
                    created_at: now,
                    last_activity: now,
                },
            );
            state
                .by_user
                .entry(user_address.to_string())
                .or_default()
                .push_back(session_id.clone());
        }

        for (kind, evicted_id) in evicted_ids {
            let detail = match kind {
                SessionActivityKind::IdleTimeout => "purged idle session before login",
                _ => "evicted oldest session due to concurrent limit",
            };
            self.log(kind, user_address, Some(&evicted_id), detail);
        }
        self.log(
            SessionActivityKind::Created,
            user_address,
            Some(&session_id),
            "session created",
        );
        self.log(
            SessionActivityKind::Authenticated,
            user_address,
            Some(&session_id),
            "user authenticated",
        );

        UserSession {
            session_id,
            user_address: user_address.to_string(),
        }
    }

    /// Validate a session token and refresh idle timer. Returns `None` if invalid/expired.
    pub fn validate_and_touch(&self, session_id: &str) -> Option<UserSession> {
        let mut state = self.inner.state.lock().expect("session state");
        let (user_address, stored_session_id, key_version, last_activity) =
            match state.sessions.get(session_id) {
                Some(session) => (
                    session.user_address.clone(),
                    session.session_id.clone(),
                    session.key_version,
                    session.last_activity,
                ),
                None => {
                    drop(state);
                    self.log(
                        SessionActivityKind::RejectedMissing,
                        "",
                        Some(session_id),
                        "unknown session id",
                    );
                    return None;
                }
            };

        let idle = last_activity.elapsed();
        if idle >= self.inner.config.idle_timeout {
            remove_session(&mut state, &stored_session_id);
            drop(state);
            self.log(
                SessionActivityKind::IdleTimeout,
                &user_address,
                Some(&stored_session_id),
                &format!("idle for {}s", idle.as_secs()),
            );
            self.log(
                SessionActivityKind::RejectedExpired,
                &user_address,
                Some(&stored_session_id),
                "session expired due to idle timeout",
            );
            return None;
        }

        let current_key_version = *state.key_versions.get(&user_address).unwrap_or(&0);
        if key_version != current_key_version {
            remove_session(&mut state, &stored_session_id);
            drop(state);
            self.log(
                SessionActivityKind::InvalidatedKeyChange,
                &user_address,
                Some(&stored_session_id),
                "session key_version mismatch",
            );
            return None;
        }

        let session = state
            .sessions
            .get_mut(session_id)
            .expect("session remains present while state is locked");
        session.last_activity = Instant::now();
        let out = UserSession {
            session_id: session.session_id.clone(),
            user_address: session.user_address.clone(),
        };
        drop(state);
        self.log(
            SessionActivityKind::Renewed,
            &out.user_address,
            Some(&out.session_id),
            "activity renewed",
        );
        Some(out)
    }

    /// Invalidate all sessions for a user after a password/key change.
    pub fn invalidate_on_key_change(&self, user_address: &str) {
        let ids = {
            let mut state = self.inner.state.lock().expect("session state");
            let entry = state
                .key_versions
                .entry(user_address.to_string())
                .or_insert(0);
            *entry = entry.wrapping_add(1);
            let ids: Vec<String> = state
                .by_user
                .get(user_address)
                .map(|q| q.iter().cloned().collect())
                .unwrap_or_default();
            for id in &ids {
                remove_session(&mut state, id);
            }
            ids
        };

        for id in &ids {
            self.log(
                SessionActivityKind::InvalidatedKeyChange,
                user_address,
                Some(id),
                "invalidated after password/key change",
            );
        }
    }

    /// Explicit logout — destroy a single session.
    pub fn logout(&self, session_id: &str) -> bool {
        let user = {
            let mut state = self.inner.state.lock().expect("session state");
            let Some(user) = state
                .sessions
                .get(session_id)
                .map(|session| session.user_address.clone())
            else {
                return false;
            };
            remove_session(&mut state, session_id);
            user
        };
        self.log(
            SessionActivityKind::Logout,
            &user,
            Some(session_id),
            "user logged out",
        );
        true
    }

    /// Count active sessions for a user (after purging idle ones).
    pub fn active_session_count(&self, user_address: &str) -> usize {
        self.purge_idle_for_user(user_address);
        self.inner
            .state
            .lock()
            .expect("session state")
            .by_user
            .get(user_address)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    /// Recent activity log (newest last).
    pub fn activity_log(&self) -> Vec<SessionActivity> {
        self.inner
            .activity
            .lock()
            .expect("activity")
            .iter()
            .cloned()
            .collect()
    }

    fn purge_idle_for_user(&self, user_address: &str) {
        let stale = {
            let mut state = self.inner.state.lock().expect("session state");
            let stale: Vec<String> = state
                .by_user
                .get(user_address)
                .map(|q| {
                    q.iter()
                        .filter(|id| {
                            state
                                .sessions
                                .get(*id)
                                .map(|s| {
                                    s.last_activity.elapsed() >= self.inner.config.idle_timeout
                                })
                                .unwrap_or(true)
                        })
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            for id in &stale {
                remove_session(&mut state, id);
            }
            stale
        };
        for id in &stale {
            self.log(
                SessionActivityKind::IdleTimeout,
                user_address,
                Some(&id),
                "purged idle session",
            );
        }
    }

    fn log(
        &self,
        kind: SessionActivityKind,
        user_address: &str,
        session_id: Option<&str>,
        detail: &str,
    ) {
        let entry = SessionActivity {
            kind,
            user_address: user_address.to_string(),
            session_id: session_id.map(session_fingerprint),
            detail: detail.to_string(),
            at_unix_ms: unix_ms_now(),
        };
        tracing::info!(
            kind = ?entry.kind,
            user = %entry.user_address,
            session_id = ?entry.session_id,
            detail = %entry.detail,
            "session.activity"
        );
        let mut log = self.inner.activity.lock().expect("activity");
        if log.len() >= ACTIVITY_LOG_CAPACITY {
            log.pop_front();
        }
        log.push_back(entry);
    }
}

fn remove_session(state: &mut SessionState, session_id: &str) -> bool {
    let Some(session) = state.sessions.remove(session_id) else {
        return false;
    };
    if let Some(user_sessions) = state.by_user.get_mut(&session.user_address) {
        user_sessions.retain(|id| id != session_id);
        if user_sessions.is_empty() {
            state.by_user.remove(&session.user_address);
        }
    }
    true
}

fn session_fingerprint(session_id: &str) -> String {
    let mut hasher = DefaultHasher::new();
    session_id.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn generate_session_id() -> String {
    format!("sess_{}", uuid::Uuid::new_v4().simple())
}

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Process-wide default store used by the Axum extractor when no state is injected.
static GLOBAL_STORE: std::sync::OnceLock<SessionStore> = std::sync::OnceLock::new();

/// Access (and lazily init) the global session store.
pub fn global_session_store() -> &'static SessionStore {
    GLOBAL_STORE.get_or_init(SessionStore::default)
}

/// Axum extractor: `Authorization: Bearer <session_id>`.
///
/// Validates against [`global_session_store`], enforces idle timeout and
/// key-version checks, and renews last-activity on success.
#[async_trait]
impl<S> FromRequestParts<S> for UserSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header_value = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "missing authorization header"))?;

        let mut hdr = header_value.splitn(2, ' ');
        let scheme = hdr.next().unwrap_or("");
        let token = hdr.next().unwrap_or("");

        if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() {
            return Err((StatusCode::UNAUTHORIZED, "invalid authorization header"));
        }

        global_session_store()
            .validate_and_touch(token)
            .ok_or((StatusCode::UNAUTHORIZED, "invalid or expired session"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_session_rotates_away_from_pre_auth_id() {
        let store = SessionStore::new(SessionConfig {
            idle_timeout: Duration::from_secs(60),
            max_sessions_per_user: 3,
        });
        let pre = "pre_auth_fixation_id";
        let session = store.create_session("GUSER", Some(pre));
        assert_ne!(session.session_id, pre);
        assert_eq!(session.session_id.len(), 37);
        assert!(store.validate_and_touch(&session.session_id).is_some());
        assert!(store.validate_and_touch(pre).is_none());
        let activity = store.activity_log();
        let fixation = activity
            .iter()
            .find(|entry| entry.kind == SessionActivityKind::RejectedFixation)
            .expect("fixation attempt is logged");
        assert_ne!(fixation.session_id.as_deref(), Some(pre));
    }

    #[test]
    fn supplied_pre_auth_id_does_not_revoke_another_users_session() {
        let store = SessionStore::default();
        let victim = store.create_session("GVictim", None);
        store.create_session("GAttacker", Some(&victim.session_id));
        assert!(store.validate_and_touch(&victim.session_id).is_some());
    }

    #[test]
    fn idle_timeout_rejects_stale_session() {
        let store = SessionStore::new(SessionConfig {
            idle_timeout: Duration::from_millis(30),
            max_sessions_per_user: 3,
        });
        let session = store.create_session("GUSER", None);
        std::thread::sleep(Duration::from_millis(50));
        assert!(store.validate_and_touch(&session.session_id).is_none());
        let kinds: Vec<_> = store.activity_log().into_iter().map(|a| a.kind).collect();
        assert!(kinds.contains(&SessionActivityKind::IdleTimeout));
    }

    #[test]
    fn concurrent_session_limit_evicts_oldest() {
        let store = SessionStore::new(SessionConfig {
            idle_timeout: Duration::from_secs(60),
            max_sessions_per_user: 2,
        });
        let s1 = store.create_session("GUSER", None);
        let s2 = store.create_session("GUSER", None);
        let s3 = store.create_session("GUSER", None);
        assert!(store.validate_and_touch(&s1.session_id).is_none());
        assert!(store.validate_and_touch(&s2.session_id).is_some());
        assert!(store.validate_and_touch(&s3.session_id).is_some());
        assert_eq!(store.active_session_count("GUSER"), 2);
    }

    #[test]
    fn expired_sessions_do_not_evict_active_sessions_at_limit() {
        let store = SessionStore::new(SessionConfig {
            idle_timeout: Duration::from_secs(60),
            max_sessions_per_user: 2,
        });
        let expired = store.create_session("GUSER", None);
        let active = store.create_session("GUSER", None);
        {
            let mut state = store.inner.state.lock().unwrap();
            state
                .sessions
                .get_mut(&expired.session_id)
                .unwrap()
                .last_activity = Instant::now() - Duration::from_secs(61);
        }

        let newest = store.create_session("GUSER", None);
        assert!(store.validate_and_touch(&expired.session_id).is_none());
        assert!(store.validate_and_touch(&active.session_id).is_some());
        assert!(store.validate_and_touch(&newest.session_id).is_some());
        assert_eq!(store.active_session_count("GUSER"), 2);
    }

    #[test]
    fn zero_session_limit_still_allows_exactly_one_session() {
        let store = SessionStore::new(SessionConfig {
            idle_timeout: Duration::from_secs(60),
            max_sessions_per_user: 0,
        });
        let first = store.create_session("GUSER", None);
        let second = store.create_session("GUSER", None);
        assert!(store.validate_and_touch(&first.session_id).is_none());
        assert!(store.validate_and_touch(&second.session_id).is_some());
        assert_eq!(store.active_session_count("GUSER"), 1);
    }

    #[test]
    fn key_change_invalidates_all_sessions() {
        let store = SessionStore::default();
        let s1 = store.create_session("GUSER", None);
        let s2 = store.create_session("GUSER", None);
        store.invalidate_on_key_change("GUSER");
        assert!(store.validate_and_touch(&s1.session_id).is_none());
        assert!(store.validate_and_touch(&s2.session_id).is_none());
        assert_eq!(store.active_session_count("GUSER"), 0);
        let kinds: Vec<_> = store.activity_log().into_iter().map(|a| a.kind).collect();
        assert!(kinds.contains(&SessionActivityKind::InvalidatedKeyChange));
    }

    #[test]
    fn logout_destroys_session_and_logs() {
        let store = SessionStore::default();
        let s = store.create_session("GUSER", None);
        assert!(store.logout(&s.session_id));
        assert!(store.validate_and_touch(&s.session_id).is_none());
    }

    #[test]
    fn activity_log_records_create() {
        let store = SessionStore::default();
        store.create_session("GABC", None);
        let log = store.activity_log();
        assert!(log.iter().any(|a| a.kind == SessionActivityKind::Created));
        assert!(log
            .iter()
            .any(|a| a.kind == SessionActivityKind::Authenticated));
    }
}
