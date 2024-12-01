use std::{collections::HashMap, sync::LazyLock};

use rusqlite::{params, OptionalExtension};
use serenity::all::UserId;
use tokio::sync::{RwLock as TokioRwLock, RwLockWriteGuard as TokioRwLockWriteGuard};

use super::DATABASE_CONN;
use crate::errors::SonorustDBError;

static DB_CACHE: LazyLock<TokioRwLock<HashMap<UserId, Option<UserData>>>> =
    LazyLock::new(|| TokioRwLock::new(HashMap::new()));

struct UserDatabase;

impl UserDatabase {
    async fn from(user_id: UserId) -> Result<Option<UserData>, SonorustDBError> {
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = DATABASE_CONN.lock().unwrap();
            let txn = conn.transaction()?;

            let result = txn
                .query_row(
                    "SELECT model_name, speaker_name, style_name, length 
                     FROM user WHERE discord_id = ?1;",
                    [user_id.get()],
                    |row| {
                        Ok(UserData {
                            user_id,
                            model_name: row.get(0)?,
                            speaker_name: row.get(1)?,
                            style_name: row.get(2)?,
                            length: row.get(3)?,
                        })
                    },
                )
                .optional()?;

            Ok(result)
        })
        .await;

        match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(err)) => Err(SonorustDBError::GetUserData(err)),
            Err(err) => Err(SonorustDBError::Unknown(err.into())),
        }
    }

    async fn update(user_data: UserData) -> Result<(), SonorustDBError> {
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = DATABASE_CONN.lock().unwrap();
            let txn = conn.transaction()?;

            txn.execute(
                "INSERT OR REPLACE INTO
                 user (discord_id, model_name, speaker_name, style_name, length)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    user_data.user_id.get(),
                    user_data.model_name,
                    user_data.speaker_name,
                    user_data.style_name,
                    user_data.length
                ],
            )?;

            txn.commit()?;
            Ok(())
        })
        .await;

        match result {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(err)) => Err(SonorustDBError::UpdateUserData(err)),
            Err(err) => Err(SonorustDBError::Unknown(err.into())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserData {
    pub user_id: UserId,
    pub model_name: String,
    pub speaker_name: String,
    pub style_name: String,
    pub length: f64,
}

impl UserData {
    pub async fn from(user_id: UserId) -> Result<UserData, SonorustDBError> {
        let cache_data = {
            let db_cache = DB_CACHE.read().await;
            db_cache.get(&user_id).map(|i| i.clone())
        };

        // cacheにあったなら取り出し、なければデータベースから取り出してキャッシュに入れる
        let result = match cache_data {
            Some(data) => data,
            None => {
                let data = UserDatabase::from(user_id).await?;

                let mut db_cache = DB_CACHE.write().await;

                match &data {
                    Some(data) => {
                        db_cache.insert(user_id, Some(data.clone()));
                    }
                    None => {
                        db_cache.insert(user_id, None);
                    }
                }

                data
            }
        };

        // データベースになければ初期設定を使用
        let user_data = match result {
            Some(data) => data,
            None => Self::default_settings(user_id),
        };

        Ok(user_data)
    }

    pub fn default_settings(user_id: UserId) -> UserData {
        Self {
            user_id,
            model_name: "None".to_string(),
            speaker_name: "None".to_string(),
            style_name: "None".to_string(),
            length: 1.0,
        }
    }
}

#[derive(Debug)]
pub struct UserDataMut<'a> {
    pub user_id: UserId,
    pub model_name: String,
    pub speaker_name: String,
    pub style_name: String,
    pub length: f64,

    cache_lock: TokioRwLockWriteGuard<'a, HashMap<UserId, Option<UserData>>>,
}

impl UserDataMut<'_> {
    pub async fn from<'a>(user_id: UserId) -> Result<UserDataMut<'a>, SonorustDBError> {
        let user_data = UserData::from(user_id).await?;

        Ok(UserDataMut {
            user_id,
            model_name: user_data.model_name,
            speaker_name: user_data.speaker_name,
            style_name: user_data.style_name,
            length: user_data.length,
            cache_lock: DB_CACHE.write().await,
        })
    }

    pub async fn update(self) -> Result<(), SonorustDBError> {
        let mut db_cache = self.cache_lock;

        let user_data = UserData {
            user_id: self.user_id,
            model_name: self.model_name,
            speaker_name: self.speaker_name,
            style_name: self.style_name,
            length: self.length,
        };

        UserDatabase::update(user_data.clone()).await?;
        db_cache.insert(self.user_id, Some(user_data));

        Ok(())
    }
}

#[cfg(test)]
mod tests_user_data_base {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_update() {
        UserDatabase::update(UserData {
            user_id: 1.into(),
            model_name: "model_name1".to_string(),
            speaker_name: "speaker_name2".to_string(),
            style_name: "style_name3".to_string(),
            length: 1.5,
        })
        .await
        .unwrap();
    }

    #[ignore]
    #[tokio::test]
    async fn test_from() {
        let user_data = UserDatabase::from(1.into()).await.unwrap();
        dbg!(user_data);

        let user_data = UserDatabase::from(2.into()).await.unwrap();
        dbg!(user_data);
    }
}

#[cfg(test)]
mod tests_user_data {
    use serenity::all::UserId;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_from() {
        for i in [1, 123, 456, 1, 123, 456] {
            dbg!(UserData::from(UserId::new(i)).await.unwrap());
            {
                let db_cache = DB_CACHE.read().await;
                dbg!(&db_cache);
            }
        }
    }
}

#[cfg(test)]
mod tests_userdata_mut {
    use serenity::all::UserId;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_from() {
        for i in [1, 123, 456, 1, 123, 456] {
            dbg!(UserDataMut::from(UserId::new(i)).await.unwrap());
            {
                let db_cache = DB_CACHE.read().await;
                dbg!(&db_cache);
            }
        }
    }

    #[ignore]
    #[tokio::test]
    async fn test_update() -> anyhow::Result<()> {
        let user_id = UserId::new(123);

        for i in ["Name1", "Name2", "Name3", "Name4"] {
            {
                let mut userdata_mut = UserDataMut::from(user_id).await?;
                dbg!(&userdata_mut.model_name);

                userdata_mut.model_name = i.to_string();
                userdata_mut.update().await?;
            }

            let user_data = UserData::from(user_id).await?;
            dbg!(&user_data.model_name);
        }

        Ok(())
    }
}
