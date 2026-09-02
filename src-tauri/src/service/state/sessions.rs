use super::*;

impl TransferServiceState {
    fn emit_session_upsert(&self, session: &SessionRecord) {
        self.emit(
            "sessions-changed",
            &SessionChangedEvent::Upsert {
                service_id: self.service_id.clone(),
                session: session.info(),
            },
        );
    }

    fn emit_sessions_removed(&self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }
        self.emit(
            "sessions-changed",
            &SessionChangedEvent::Remove {
                service_id: self.service_id.clone(),
                ids: ids.to_vec(),
            },
        );
    }

    pub fn rotate_code(&self) -> String {
        let mut throttle = self
            .auth_attempts
            .lock()
            .expect("auth attempt lock poisoned");
        *throttle = AuthAttemptState::new(Instant::now());
        let value = new_code();
        *self.access_code.write().expect("access code lock poisoned") = value.clone();
        value
    }

    pub fn verify_access_code(&self, address: IpAddr, supplied: &str) -> AuthDecision {
        self.verify_access_code_at(address, supplied, Instant::now())
    }

    pub(super) fn verify_access_code_at(
        &self,
        address: IpAddr,
        supplied: &str,
        now: Instant,
    ) -> AuthDecision {
        let mut throttle = self
            .auth_attempts
            .lock()
            .expect("auth attempt lock poisoned");
        throttle.attempts.retain(|_, record| {
            record.blocked_until.is_some_and(|until| until > now)
                || now.saturating_duration_since(record.last_seen) < AUTH_ATTEMPT_TTL
        });
        if now.saturating_duration_since(throttle.global_window_started) >= AUTH_WINDOW {
            throttle.global_failures = 0;
            throttle.global_window_started = now;
            throttle.global_blocked_until = None;
        }
        if throttle
            .global_blocked_until
            .is_some_and(|until| until > now)
        {
            return AuthDecision::GlobalBlocked;
        }
        if throttle.global_blocked_until.is_some() {
            throttle.global_blocked_until = None;
            throttle.global_failures = 0;
            throttle.global_window_started = now;
        }
        if throttle
            .attempts
            .get(&address)
            .and_then(|record| record.blocked_until)
            .is_some_and(|until| until > now)
        {
            return AuthDecision::AddressBlocked;
        }

        let expected = self
            .access_code
            .read()
            .expect("access code lock poisoned")
            .clone();
        let valid_shape = supplied.len() == ACCESS_CODE_DIGITS
            && supplied.bytes().all(|value| value.is_ascii_digit());
        let correct =
            valid_shape && supplied.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() == 1;
        if correct {
            throttle.attempts.remove(&address);
            return AuthDecision::Accepted;
        }

        throttle.global_failures = throttle.global_failures.saturating_add(1);
        if throttle.global_failures >= AUTH_FAILURES_GLOBAL {
            throttle.global_blocked_until = Some(now + AUTH_BLOCK_DURATION);
            throttle.global_window_started = now;
            return AuthDecision::GlobalBlocked;
        }

        if let Some(record) = throttle.attempts.get_mut(&address) {
            if record.blocked_until.is_some() {
                record.failures = 0;
                record.blocked_until = None;
            }
            record.failures = record.failures.saturating_add(1);
            record.last_seen = now;
            if record.failures >= AUTH_FAILURES_PER_ADDRESS {
                record.failures = 0;
                record.blocked_until = Some(now + AUTH_BLOCK_DURATION);
                return AuthDecision::AddressBlocked;
            }
        } else if throttle.attempts.len() < MAX_AUTH_ATTEMPT_RECORDS {
            throttle.attempts.insert(
                address,
                AttemptRecord {
                    failures: 1,
                    blocked_until: None,
                    last_seen: now,
                },
            );
        }
        AuthDecision::Invalid
    }

    #[cfg(test)]
    pub async fn create_session(
        &self,
        address: IpAddr,
        user_agent: String,
    ) -> Result<SessionRecord, SessionCreateError> {
        self.create_named_session(address, &user_agent, None).await
    }

    pub async fn create_named_session(
        &self,
        address: IpAddr,
        user_agent: &str,
        device_name: Option<&str>,
    ) -> Result<SessionRecord, SessionCreateError> {
        let now_instant = Instant::now();
        let now = Utc::now().to_rfc3339();
        let record = SessionRecord {
            id: Uuid::new_v4().to_string(),
            token: random_token(32),
            csrf: random_token(24),
            address,
            device_name: normalize_device_name(device_name)?,
            client_name: describe_user_agent(user_agent),
            created_at: now.clone(),
            last_activity: now,
            created_at_instant: now_instant,
            last_activity_instant: now_instant,
        };
        let (expired, result) = {
            let mut sessions = self.sessions.lock().await;
            let expired: Vec<_> = sessions
                .values()
                .filter(|session| session.expired_at(now_instant))
                .map(|session| session.id.clone())
                .collect();
            sessions.retain(|_, session| !session.expired_at(now_instant));
            let result = if sessions.len() >= MAX_SESSIONS_GLOBAL {
                Err(SessionCreateError::GlobalLimit)
            } else if sessions
                .values()
                .filter(|session| session.address == address)
                .count()
                >= MAX_SESSIONS_PER_ADDRESS
            {
                Err(SessionCreateError::AddressLimit)
            } else {
                sessions.insert(record.token.clone(), record.clone());
                Ok(record.clone())
            };
            (expired, result)
        };
        self.cleanup_expired_sessions(&expired).await;
        self.emit_sessions_removed(&expired);
        if let Ok(session) = &result {
            self.emit_session_upsert(session);
        }
        result
    }

    pub async fn authenticate(&self, token: &str, address: IpAddr) -> Option<SessionRecord> {
        let now = Instant::now();
        let (session, expired) = {
            let mut sessions = self.sessions.lock().await;
            let current = sessions.get(token)?;
            if current.expired_at(now) {
                let id = current.id.clone();
                sessions.remove(token);
                (None, Some(id))
            } else if current.address != address {
                return None;
            } else {
                let current = sessions.get_mut(token).expect("session remains present");
                current.last_activity = Utc::now().to_rfc3339();
                current.last_activity_instant = now;
                (Some(current.clone()), None)
            }
        };
        if let Some(expired) = expired {
            self.cleanup_expired_sessions(std::slice::from_ref(&expired))
                .await;
            self.emit_sessions_removed(std::slice::from_ref(&expired));
            return None;
        }
        self.touch();
        session
    }

    pub async fn session_token_is_active(&self, token: &str, address: IpAddr) -> bool {
        let now = Instant::now();
        self.sessions
            .lock()
            .await
            .get(token)
            .is_some_and(|session| session.address == address && !session.expired_at(now))
    }

    pub async fn session_is_active(&self, expected: &SessionRecord) -> bool {
        let now = Instant::now();
        let (active, expired) = {
            let mut sessions = self.sessions.lock().await;
            let expired = sessions
                .get(&expected.token)
                .and_then(|current| current.expired_at(now).then(|| current.id.clone()));
            if expired.is_some() {
                sessions.remove(&expected.token);
            }
            let active = sessions.get(&expected.token).is_some_and(|current| {
                current.id == expected.id && current.address == expected.address
            });
            (active, expired)
        };
        if let Some(expired) = expired {
            self.cleanup_expired_sessions(std::slice::from_ref(&expired))
                .await;
            self.emit_sessions_removed(std::slice::from_ref(&expired));
        }
        active
    }

    async fn cleanup_expired_sessions(&self, session_ids: &[String]) {
        for id in session_ids {
            self.cancel_downloads(Some(id)).await;
            self.cancel_uploads(Some(id)).await;
            self.remove_directory_listings(Some(id)).await;
        }
    }

    pub async fn expire_stale_sessions(&self) {
        let now = Instant::now();
        let expired = {
            let mut sessions = self.sessions.lock().await;
            let expired: Vec<_> = sessions
                .values()
                .filter(|session| session.expired_at(now))
                .map(|session| session.id.clone())
                .collect();
            sessions.retain(|_, session| !session.expired_at(now));
            expired
        };
        if !expired.is_empty() {
            self.cleanup_expired_sessions(&expired).await;
            self.emit_sessions_removed(&expired);
        }
    }

    pub async fn revoke_session(&self, id: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        let before = sessions.len();
        sessions.retain(|_, item| item.id != id);
        let changed = before != sessions.len();
        drop(sessions);
        if changed {
            self.cancel_downloads(Some(id)).await;
            self.cancel_uploads(Some(id)).await;
            self.remove_directory_listings(Some(id)).await;
            self.emit_sessions_removed(&[id.into()]);
        }
        changed
    }

    pub async fn revoke_all(&self) {
        let revoked: Vec<_> = {
            let mut sessions = self.sessions.lock().await;
            let revoked = sessions
                .values()
                .map(|session| session.id.clone())
                .collect();
            sessions.clear();
            revoked
        };
        self.cleanup_expired_sessions(&revoked).await;
        self.emit(
            "sessions-changed",
            &SessionChangedEvent::Reset {
                service_id: self.service_id.clone(),
            },
        );
    }
}
