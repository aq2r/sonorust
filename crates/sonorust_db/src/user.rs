use std::{collections::HashMap, sync::LazyLock};

use serenity::all::UserId;
use sqlx::Row;
use tokio::sync::{RwLock as TokioRwLock, RwLockWriteGuard as TokioRwLockWriteGuard};

use crate::DB_POOL;

struct UserDatabase;
impl UserDatabase {
    async fn from<T>(user_id: T) -> Result<Option<UserData>, sqlx::Error>
    where
        T: Into<UserId>,
    {
        let user_id: UserId = user_id.into();
        log::debug!("Access User Database Get - ID: {user_id}");

        let pool = DB_POOL.get().expect("Not initialaized DB_POOL");
        let mut tx = pool.begin().await?;

        let result = sqlx::query(
            "SELECT model_name, speaker_name, style_name, length 
                     FROM user WHERE discord_id = ?1;",
        )
        .bind(user_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;

        let userdata = result.map(|row| UserData {
            user_id,
            model_name: row.get("model_name"),
            speaker_name: row.get("speaker_name"),
            style_name: row.get("style_name"),
            length: row.get("length"),
        });

        tx.commit().await?;
        Ok(userdata)
    }

    async fn update(userdata: UserData) -> Result<(), sqlx::Error> {
        log::debug!("Access User Database Update - ID: {}", userdata.user_id);

        let pool = DB_POOL.get().expect("Not initialaized DB_POOL");
        let mut tx = pool.begin().await?;

        sqlx::query(
            "INSERT OR REPLACE INTO
                 user (discord_id, model_name, speaker_name, style_name, length)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(userdata.user_id.to_string())
        .bind(userdata.model_name)
        .bind(userdata.speaker_name)
        .bind(userdata.style_name)
        .bind(userdata.length)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

static DB_CACHE: LazyLock<TokioRwLock<HashMap<UserId, Option<UserData>>>> =
    LazyLock::new(|| TokioRwLock::new(HashMap::new()));

#[derive(Debug, Clone)]
pub struct UserData {
    pub user_id: UserId,
    pub model_name: String,
    pub speaker_name: String,
    pub style_name: String,
    pub length: f64,
}

impl UserData {
    pub async fn from<T>(user_id: T) -> Result<UserData, sqlx::Error>
    where
        T: Into<UserId>,
    {
        let user_id: UserId = user_id.into();

        let cache_data = {
            let db_cache = DB_CACHE.read().await;
            db_cache.get(&user_id).map(|i| i.clone())
        };

        let userdata = match cache_data {
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
        let userdata = match userdata {
            Some(data) => data,
            None => Self::default_settings(user_id),
        };

        Ok(userdata)
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
    pub async fn from<'a>(user_id: UserId) -> Result<UserDataMut<'a>, sqlx::Error> {
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

    pub async fn update(self) -> Result<(), sqlx::Error> {
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

    use tokio::fs::create_dir_all;

    use crate::init_database;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_update() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        UserDatabase::update(UserData {
            user_id: 1_u64.into(),
            model_name: "model_name1".to_string(),
            speaker_name: "speaker_name2".to_string(),
            style_name: "style_name3".to_string(),
            length: 1.5,
        })
        .await?;

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_from() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        let user_data = UserDatabase::from(1).await.unwrap();
        dbg!(user_data);

        let user_data = UserDatabase::from(2).await.unwrap();
        dbg!(user_data);

        Ok(())
    }
}

#[cfg(test)]
mod tests_user_data {
    use serenity::all::UserId;
    use tokio::fs::create_dir_all;

    use crate::init_database;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_from() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        for i in [1, 123, 456, 1, 123, 456] {
            dbg!(UserData::from(UserId::new(i)).await.unwrap());
            {
                let db_cache = DB_CACHE.read().await;
                dbg!(&db_cache);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests_userdata_mut {
    use serenity::all::UserId;
    use tokio::fs::create_dir_all;

    use crate::init_database;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_from() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        for i in [1, 123, 456, 1, 123, 456] {
            dbg!(UserDataMut::from(UserId::new(i)).await.unwrap());
            {
                let db_cache = DB_CACHE.read().await;
                dbg!(&db_cache);
            }
        }

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_update() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

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
