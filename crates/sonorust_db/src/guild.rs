use std::{collections::HashMap, sync::LazyLock};

use rusqlite::{params, OptionalExtension};
use serenity::all::GuildId;
use tokio::sync::{RwLock as TokioRwLock, RwLockWriteGuard as TokioRwLockWriteGuard};

use super::DATABASE_CONN;
use crate::errors::SonorustDBError;

static DB_CACHE: LazyLock<TokioRwLock<HashMap<GuildId, Option<GuildData>>>> =
    LazyLock::new(|| TokioRwLock::new(HashMap::new()));

struct GuildDatabase;

impl GuildDatabase {
    async fn from(guild_id: GuildId) -> Result<Option<GuildData>, SonorustDBError> {
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = DATABASE_CONN.lock().unwrap();
            let txn = conn.transaction()?;

            // guildtable_id の取り出し
            let sql = "SELECT id FROM guild WHERE discord_id = ?1;";
            let result: Option<u64> = txn
                .query_row(sql, [guild_id.get()], |row| row.get(0))
                .optional()?;

            // データベースになければ返す
            let Some(guildtable_id) = result else {
                return Ok(None);
            };

            // サーバー辞書
            let mut dict: HashMap<String, String> = HashMap::new();
            {
                let mut stmt = txn.prepare(
                    "SELECT before_text, after_text FROM guild_dict WHERE guild_table_id = ?1",
                )?;

                let mut rows = stmt.query([guildtable_id])?;

                while let Some(row) = rows.next()? {
                    dict.insert(row.get(0)?, row.get(1)?);
                }
            }

            // サーバーオプション
            let mut options = GuildOptions::default();
            let option_pairs = [
                (&mut options.is_auto_join, "is_auto_join"),
                (&mut options.is_dic_onlyadmin, "is_dic_onlyadmin"),
                (&mut options.is_entrance_exit_log, "is_entrance_exit_log"),
                (&mut options.is_entrance_exit_play, "is_entrance_exit_play"),
                (&mut options.is_if_long_fastread, "is_if_long_fastread"),
                (&mut options.is_notice_attachment, "is_notice_attachment"),
            ];

            {
                let mut stmt = txn.prepare(
                    "
                    SELECT id FROM guild_guild_options
                    WHERE guild_table_id = ?1
                      AND guild_option_table_id = (SELECT id FROM guild_option WHERE option_name = ?2);
                    ",
                )?;

                for (option_refm, option_name) in option_pairs {
                    let result: Option<u64> = stmt
                        .query_row(params![guildtable_id, option_name], |row| row.get(0))
                        .optional()?;

                    match result {
                        Some(_) => *option_refm = true,
                        None => *option_refm = false,
                    }
                }
            }

            txn.commit()?;

            Ok(Some(GuildData {
                guild_id,
                dict: dict,
                options: options,
            }))
        })
        .await;

        match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(err)) => Err(SonorustDBError::UpdateGuildData(err)),
            Err(err) => Err(SonorustDBError::Unknown(err.into())),
        }
    }

    async fn update(guild_data: GuildData) -> Result<(), SonorustDBError> {
        let result = tokio::task::spawn_blocking(move || {
            let mut conn = DATABASE_CONN.lock().unwrap();
            let txn = conn.transaction()?;

            // guildtable_id の取り出し
            let sql = "SELECT id FROM guild WHERE discord_id = ?1;";
            let result: Option<u64> = txn
                .query_row(sql, [guild_data.guild_id.get()], |row| row.get(0))
                .optional()?;

            let guildtable_id = match result {
                Some(id) => id,
                None => {
                    txn.execute(
                        "INSERT INTO guild (discord_id) VALUES (?1);",
                        [guild_data.guild_id.get()],
                    )?;

                    txn.query_row(sql, [guild_data.guild_id.get()], |row| row.get(0))?
                }
            };

            // サーバー辞書のアップデート
            {
                txn.execute(
                    "DELETE FROM guild_dict WHERE guild_table_id = ?1",
                    [guildtable_id],
                )?;

                let mut stmt = txn
                    .prepare("
                    INSERT INTO guild_dict (guild_table_id, before_text, after_text)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT (guild_table_id, before_text) DO NOTHING;
                            ")?;

                for (before_text, after_text) in guild_data.dict {
                    stmt.execute(params![guildtable_id, before_text, after_text])?;
                }
            }


            // オプションのアップデート
            {
                let mut insert_stmt = txn.prepare(
                    "
                    INSERT INTO guild_guild_options (guild_table_id, guild_option_table_id)
                    VALUES (?1,
                            (SELECT (id) FROM guild_option WHERE option_name = ?2))
                    ON CONFLICT (guild_table_id, guild_option_table_id) DO NOTHING;
                    ",
                )?;
                let mut delete_stmt = txn.prepare(
                    "
                    DELETE FROM guild_guild_options
                        WHERE guild_table_id = ?1
                        AND guild_option_table_id = (SELECT (id) FROM guild_option WHERE option_name = ?2);
                    ",
                )?;

                let options = guild_data.options;
                let option_pairs = [
                    (options.is_auto_join, "is_auto_join"),
                    (options.is_dic_onlyadmin, "is_dic_onlyadmin"),
                    (options.is_entrance_exit_log, "is_entrance_exit_log"),
                    (options.is_entrance_exit_play, "is_entrance_exit_play"),
                    (options.is_if_long_fastread, "is_if_long_fastread"),
                    (options.is_notice_attachment, "is_notice_attachment"),
                ];

                for (option_bool, option_name) in option_pairs {
                    let params = params![guildtable_id, option_name];

                    if option_bool {
                        insert_stmt.execute(params)?;
                    } else {
                        delete_stmt.execute(params)?;
                    }
                }
            }

            txn.commit()?;
            Ok(())
        })
        .await;

        match result {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(err)) => Err(SonorustDBError::UpdateGuildData(err)),
            Err(err) => Err(SonorustDBError::Unknown(err.into())),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GuildOptions {
    pub is_dic_onlyadmin: bool,
    pub is_auto_join: bool,
    pub is_entrance_exit_log: bool,
    pub is_entrance_exit_play: bool,
    pub is_notice_attachment: bool,
    pub is_if_long_fastread: bool,
}

impl Default for GuildOptions {
    fn default() -> Self {
        Self {
            is_dic_onlyadmin: true,
            is_auto_join: false,
            is_entrance_exit_log: false,
            is_entrance_exit_play: false,
            is_notice_attachment: false,
            is_if_long_fastread: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GuildData {
    pub guild_id: GuildId,
    pub dict: HashMap<String, String>,
    pub options: GuildOptions,
}

impl GuildData {
    pub async fn from(guild_id: GuildId) -> Result<GuildData, SonorustDBError> {
        let cache_data = {
            let db_cache = DB_CACHE.read().await;
            db_cache.get(&guild_id).map(|i| i.clone())
        };

        // cacheにあったなら取り出し、なければデータベースから取り出してキャッシュに入れる
        let result = match cache_data {
            Some(data) => data,
            None => {
                let data = GuildDatabase::from(guild_id).await?;

                let mut db_cache = DB_CACHE.write().await;

                match &data {
                    Some(data) => {
                        db_cache.insert(guild_id, Some(data.clone()));
                    }
                    None => {
                        db_cache.insert(guild_id, None);
                    }
                }

                data
            }
        };

        // データベースになければ初期設定を使用
        let guild_data = match result {
            Some(data) => data,
            None => Self::default_settings(guild_id),
        };

        Ok(guild_data)
    }

    pub fn default_settings(guild_id: GuildId) -> GuildData {
        Self {
            guild_id,
            dict: HashMap::new(),
            options: GuildOptions::default(),
        }
    }
}

#[derive(Debug)]
pub struct GuildDataMut<'a> {
    pub guild_id: GuildId,
    pub dict: HashMap<String, String>,
    pub options: GuildOptions,

    cache_lock: TokioRwLockWriteGuard<'a, HashMap<GuildId, Option<GuildData>>>,
}

impl GuildDataMut<'_> {
    pub async fn from<'a>(guild_id: GuildId) -> Result<GuildDataMut<'a>, SonorustDBError> {
        let guild_data = GuildData::from(guild_id).await?;

        Ok(GuildDataMut {
            guild_id,
            dict: guild_data.dict,
            options: guild_data.options,
            cache_lock: DB_CACHE.write().await,
        })
    }

    pub async fn update(self) -> Result<(), SonorustDBError> {
        let mut db_cache = self.cache_lock;

        let guild_data = GuildData {
            guild_id: self.guild_id,
            dict: self.dict,
            options: self.options,
        };

        GuildDatabase::update(guild_data.clone()).await?;
        db_cache.insert(self.guild_id, Some(guild_data));

        Ok(())
    }
}

#[cfg(test)]
mod tests_guild_data_base {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_update() {
        let dict = [("A1", "B1"), ("A2", "B2"), ("A3", "B3")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        GuildDatabase::update(GuildData {
            guild_id: GuildId::new(123),
            dict,
            options: GuildOptions {
                is_dic_onlyadmin: true,
                is_auto_join: false,
                is_entrance_exit_log: true,
                is_entrance_exit_play: false,
                is_notice_attachment: true,
                is_if_long_fastread: false,
            },
        })
        .await
        .unwrap();

        let dict = [("A4", "B4"), ("A5", "B5"), ("A6", "B6")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        GuildDatabase::update(GuildData {
            guild_id: GuildId::new(456),
            dict,
            options: GuildOptions {
                is_dic_onlyadmin: false,
                is_auto_join: false,
                is_entrance_exit_log: false,
                is_entrance_exit_play: true,
                is_notice_attachment: true,
                is_if_long_fastread: true,
            },
        })
        .await
        .unwrap()
    }

    #[ignore]
    #[tokio::test]
    async fn test_from() {
        for i in [123, 456] {
            let guild_data = GuildDatabase::from(GuildId::new(i)).await.unwrap();
            dbg!(guild_data);
        }
    }
}

#[cfg(test)]
mod tests_guild_data {
    use serenity::all::GuildId;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_from() {
        for i in [1, 123, 456, 1, 123, 456] {
            dbg!(GuildData::from(GuildId::new(i)).await.unwrap());
            {
                let db_cache = DB_CACHE.read().await;
                dbg!(&db_cache);
            }
        }
    }
}

#[cfg(test)]
mod tests_guilddata_mut {
    use serenity::all::GuildId;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_from() {
        for i in [1, 123, 456] {
            dbg!(GuildDataMut::from(GuildId::new(i)).await.unwrap());
        }
    }

    #[ignore]
    #[tokio::test]
    async fn test_update() -> anyhow::Result<()> {
        let guild_id = GuildId::from(123);

        for i in [true, false, true, false] {
            {
                let mut guilddata_mut = GuildDataMut::from(guild_id).await?;
                dbg!(guilddata_mut.options.is_auto_join);

                guilddata_mut.options.is_auto_join = i;
                guilddata_mut.update().await?;
            }

            let guild_data = GuildData::from(guild_id).await?;
            dbg!(guild_data.options.is_auto_join);
        }

        Ok(())
    }
}
