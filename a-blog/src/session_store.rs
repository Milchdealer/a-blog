use actix_session::storage::{LoadError, SaveError, SessionKey, SessionStore, UpdateError};
use actix_web::cookie::time::Duration;
use chrono::prelude::*;
use entity::session;
use entity::session::Entity as Session;
use log::error;
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng as _};
use sea_orm::{entity::*, query::*};
use sea_orm::{DatabaseConnection, EntityTrait};
use std::collections::HashMap;
use std::sync::Arc;

type SessionState = HashMap<String, String>;

#[derive(Clone)]
pub struct SqliteSessionStore {
    conn: DatabaseConnection,
    config: CacheConfiguration,
}

impl SqliteSessionStore {
    pub async fn new<S: Into<String>>(conn: S) -> SqliteSessionStore {
        SqliteSessionStore {
            conn: sea_orm::Database::connect(conn.into())
                .await
                .map_err(|e| panic!("Failed to initiate SessionStore {}", e))
                .unwrap(),
            config: CacheConfiguration::default(),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl SessionStore for SqliteSessionStore {
    async fn load(&self, session_key: &SessionKey) -> Result<Option<SessionState>, LoadError> {
        let session_key = (self.config.cache_keygen)(session_key.as_ref());

        match Session::find()
            .filter(session::Column::SessionKey.eq(session_key.clone()))
            .one(&self.conn)
            .await
        {
            Ok(sess) => match sess {
                Some(sess) => {
                    let session_state: Option<SessionState> =
                        serde_json::from_str(&sess.session_state)
                            .map_err(Into::into)
                            .map_err(LoadError::Deserialization)?;

                    if Utc::now() > sess.death_date {
                        Ok(None)
                    } else {
                        Ok(session_state)
                    }
                }
                None => Ok(None),
            },
            Err(e) => {
                error!("Failed to find session {}: {}", session_key, e);
                Err(LoadError::Other(e.into()))
            }
        }
    }

    async fn save(
        &self,
        session_state: SessionState,
        ttl: &Duration,
    ) -> Result<SessionKey, SaveError> {
        let session_key = generate_session_key();
        let cache_key = (self.config.cache_keygen)(session_key.as_ref());
        let body = serde_json::to_string(&session_state)
            .map_err(Into::into)
            .map_err(SaveError::Serialization)?;
        let duration = chrono::Duration::seconds(ttl.whole_seconds());

        session::ActiveModel {
            session_key: Set(cache_key.into()),
            session_state: Set(body.to_owned()),
            death_date: Set(Utc::now() + duration),
            ..Default::default()
        }
        .insert(&self.conn)
        .await
        .map_err(Into::into)
        .map_err(SaveError::Other)
        .unwrap();

        Ok(session_key
            .try_into()
            .map_err(Into::into)
            .map_err(SaveError::Other)?)
    }

    async fn update(
        &self,
        session_key: SessionKey,
        session_state: SessionState,
        ttl: &Duration,
    ) -> Result<SessionKey, UpdateError> {
        let session_key_str = (self.config.cache_keygen)(session_key.as_ref());
        let duration = chrono::Duration::seconds(ttl.whole_seconds());

        match Session::find()
            .filter(session::Column::SessionKey.eq(session_key_str.clone()))
            .one(&self.conn)
            .await
        {
            Ok(sess) => {
                let mut sess = match sess {
                    Some(sess) => sess.into_active_model(),
                    None => {
                        // Re-create session if it was deleted in between calling load() and update()
                        let body = serde_json::to_string(&session_state)
                            .map_err(Into::into)
                            .map_err(UpdateError::Serialization)?;

                        session::ActiveModel {
                            session_key: Set(session_key_str.into()),
                            session_state: Set(body.to_owned()),
                            ..Default::default()
                        }
                    }
                };

                sess.death_date = Set(Utc::now() + duration);
                sess.update(&self.conn)
                    .await
                    .map_err(|e| UpdateError::Other(e.into()))
                    .unwrap();

                Ok(session_key)
            }
            Err(e) => {
                error!("Failed to find session {}: {}", session_key_str, e);
                Err(UpdateError::Other(e.into()))
            }
        }
    }

    async fn update_ttl(
        &self,
        session_key: &SessionKey,
        ttl: &Duration,
    ) -> Result<(), anyhow::Error> {
        let session_key = session_key.as_ref();

        match Session::find()
            .filter(session::Column::SessionKey.eq(session_key))
            .one(&self.conn)
            .await
        {
            Ok(sess) => match sess {
                Some(sess) => {
                    let duration = chrono::Duration::seconds(ttl.whole_seconds());
                    let mut sess = sess.into_active_model();
                    sess.death_date = Set(Utc::now() + duration);
                    sess.update(&self.conn).await.unwrap();

                    Ok(())
                }
                None => Ok(()),
            },
            Err(e) => {
                error!("Failed to find session {}: {}", session_key, e);
                Err(e.into())
            }
        }
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<(), anyhow::Error> {
        let session_key = session_key.as_ref();

        match Session::find()
            .filter(session::Column::SessionKey.eq(session_key))
            .one(&self.conn)
            .await
        {
            Ok(sess) => {
                let sess = sess.unwrap();
                sess.delete(&self.conn).await.unwrap();
                Ok(())
            }
            Err(e) => {
                error!("Failed to find session {}: {}", session_key, e);
                Err(e.into())
            }
        }
    }
}

#[derive(Clone)]
struct CacheConfiguration {
    cache_keygen: Arc<dyn Fn(&str) -> String + Send + Sync>,
}

impl Default for CacheConfiguration {
    fn default() -> Self {
        Self {
            cache_keygen: Arc::new(str::to_owned),
        }
    }
}

/// Session key generation routine that follows [OWASP recommendations].
///
/// [OWASP recommendations]: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#session-id-entropy
fn generate_session_key() -> SessionKey {
    let value = std::iter::repeat(())
        .map(|()| OsRng.sample(Alphanumeric))
        .take(64)
        .collect::<Vec<_>>();

    // These unwraps will never panic because pre-conditions are always verified
    // (i.e. length and character set)
    String::from_utf8(value).unwrap().try_into().unwrap()
}
