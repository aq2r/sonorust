use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use serenity::all::{ChannelId, GuildId};
use sqlx::Row;
use tokio::sync::{RwLock as TokioRwLock, RwLockWriteGuard as TokioRwLockWriteGuard};

use crate::DB_POOL;

pub(crate) enum GuildOptionsStr {
    IsDicOnlyAdmin,
    IsEntranceExitLog,
    IsEntranceExitPlay,
    IsNoticeAttachment,
    IsIfLongFastRead,
}

impl GuildOptionsStr {
    pub fn as_str(&self) -> &'static str {
        match self {
            GuildOptionsStr::IsDicOnlyAdmin => "is_dic_onlyadmin",
            GuildOptionsStr::IsEntranceExitLog => "is_entrance_exit_log",
            GuildOptionsStr::IsEntranceExitPlay => "is_entrance_exit_play",
            GuildOptionsStr::IsNoticeAttachment => "is_notice_attachment",
            GuildOptionsStr::IsIfLongFastRead => "is_if_long_fastread",
        }
    }
}

struct GuildDatabase;
impl GuildDatabase {
    async fn from<T>(guild_id: T) -> Result<Option<GuildData>, sqlx::Error>
    where
        T: Into<GuildId>,
    {
        let guild_id: GuildId = guild_id.into();
        log::debug!("Access Guild Database Get - ID: {guild_id}");

        let pool = DB_POOL.get().expect("Not initialaized DB_POOL");
        let mut tx = pool.begin().await?;

        // guild table id
        let result: Option<u64> = sqlx::query("SELECT id FROM guild WHERE discord_id = ?1;")
            .bind(guild_id.to_string())
            .fetch_optional(&mut *tx)
            .await?
            .map(|row| row.get("id"));

        let Some(guild_table_id) = result else {
            return Ok(None);
        };

        let guild_table_id_string = guild_table_id.to_string();

        // サーバー辞書
        let dict: HashMap<String, String> =
            sqlx::query("SELECT before_text, after_text FROM guild_dict WHERE guild_table_id = ?1")
                .bind(&guild_table_id_string)
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .map(|row| (row.get("before_text"), row.get("after_text")))
                .collect();

        // サーバーオプション
        let mut options = GuildOptions::default();
        let option_pairs = [
            (
                &mut options.is_dic_onlyadmin,
                GuildOptionsStr::IsDicOnlyAdmin,
            ),
            (
                &mut options.is_entrance_exit_log,
                GuildOptionsStr::IsEntranceExitLog,
            ),
            (
                &mut options.is_entrance_exit_play,
                GuildOptionsStr::IsEntranceExitPlay,
            ),
            (
                &mut options.is_if_long_fastread,
                GuildOptionsStr::IsIfLongFastRead,
            ),
            (
                &mut options.is_notice_attachment,
                GuildOptionsStr::IsNoticeAttachment,
            ),
        ];

        for (option_refm, option_name) in option_pairs {
            let result = sqlx::query(
                "
                SELECT id FROM guild_guild_options
                WHERE guild_table_id = ?1
                    AND guild_option_table_id = (SELECT id FROM guild_option WHERE option_name = ?2);
                ",
            )
                .bind(&guild_table_id_string)
                .bind(option_name.as_str())
                .fetch_optional(&mut *tx)
                .await?;

            *option_refm = result.is_some();
        }

        // autojoin_channels
        let mut autojoin_channels = HashMap::new();
        let result = sqlx::query(
            "
            SELECT voice_channel_id, text_channel_id FROM guild_auto_join
            WHERE guild_table_id = ?1
        ",
        )
        .bind(&guild_table_id_string)
        .fetch_all(&mut *tx)
        .await?;

        for row in result {
            let voice_channel_id: u64 = row.get("voice_channel_id");
            let text_channel_id: u64 = row.get("text_channel_id");

            let voice_channel_id = ChannelId::new(voice_channel_id);
            let text_channel_id = ChannelId::new(text_channel_id);

            // 下コメントと同じ
            autojoin_channels
                .entry(voice_channel_id)
                .or_insert_with(|| HashSet::new())
                .insert(text_channel_id);

            // if let None = autojoin_channels.get_mut(&voice_channel_id) {
            //     autojoin_channels.insert(voice_channel_id, HashSet::new());
            // }

            // if let Some(hashset) = autojoin_channels.get_mut(&voice_channel_id) {
            //     hashset.insert(text_channel_id);
            // }
        }

        tx.commit().await?;

        Ok(Some(GuildData {
            guild_id,
            dict,
            autojoin_channels,
            options,
        }))
    }

    async fn update(guilddata: GuildData) -> Result<(), sqlx::Error> {
        log::debug!("Access Guild Database Update - ID: {}", guilddata.guild_id);

        let pool = DB_POOL.get().expect("Not initialaized DB_POOL");
        let mut tx = pool.begin().await?;

        // guild_table_id 取得
        let sql = "SELECT id FROM guild WHERE discord_id = ?1;";
        let guild_id_string = guilddata.guild_id.to_string();
        let result: Option<u64> = sqlx::query(sql)
            .bind(&guild_id_string)
            .fetch_optional(&mut *tx)
            .await?
            .map(|row| row.get("id"));

        let guild_table_id = match result {
            Some(id) => id,
            None => {
                sqlx::query("INSERT INTO guild (discord_id) VALUES (?1)")
                    .bind(&guild_id_string)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query(sql)
                    .bind(&guild_id_string)
                    .fetch_one(&mut *tx)
                    .await?
                    .get("id")
            }
        };
        let guild_table_id_string = guild_table_id.to_string();

        // サーバー辞書更新
        sqlx::query("DELETE FROM guild_dict WHERE guild_table_id = ?1")
            .bind(&guild_table_id_string)
            .execute(&mut *tx)
            .await?;

        for (before_text, after_text) in guilddata.dict {
            sqlx::query(
                "
                INSERT INTO guild_dict (guild_table_id, before_text, after_text)
                VALUES (?1, ?2, ?3)
                ON CONFLICT (guild_table_id, before_text) DO NOTHING;
                ",
            )
            .bind(&guild_table_id_string)
            .bind(before_text)
            .bind(after_text)
            .execute(&mut *tx)
            .await?;
        }

        // ギルドオプション更新
        let insert_sql = "
            INSERT INTO guild_guild_options (guild_table_id, guild_option_table_id)
            VALUES (?1,
                    (SELECT (id) FROM guild_option WHERE option_name = ?2))
            ON CONFLICT (guild_table_id, guild_option_table_id) DO NOTHING;
            ";
        let delete_sql = "
        DELETE FROM guild_guild_options
            WHERE guild_table_id = ?1
            AND guild_option_table_id = (SELECT (id) FROM guild_option WHERE option_name = ?2);
        ";

        let options = guilddata.options;
        let option_pairs = [
            (options.is_dic_onlyadmin, GuildOptionsStr::IsDicOnlyAdmin),
            (
                options.is_entrance_exit_log,
                GuildOptionsStr::IsEntranceExitLog,
            ),
            (
                options.is_entrance_exit_play,
                GuildOptionsStr::IsEntranceExitPlay,
            ),
            (
                options.is_if_long_fastread,
                GuildOptionsStr::IsIfLongFastRead,
            ),
            (
                options.is_notice_attachment,
                GuildOptionsStr::IsNoticeAttachment,
            ),
        ];

        for (option_bool, option_name) in option_pairs {
            let sql = if option_bool { insert_sql } else { delete_sql };

            sqlx::query(sql)
                .bind(&guild_table_id_string)
                .bind(option_name.as_str())
                .execute(&mut *tx)
                .await?;
        }

        // autojoin 更新
        sqlx::query("DELETE FROM guild_auto_join WHERE guild_table_id = ?1")
            .bind(&guild_table_id_string)
            .execute(&mut *tx)
            .await?;

        for (voice_channel_id, hashset) in guilddata.autojoin_channels {
            let voice_channel_id_string = voice_channel_id.to_string();

            for text_channel_id in hashset {
                sqlx::query(
                    "
                    INSERT INTO guild_auto_join (guild_table_id, voice_channel_id, text_channel_id)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT (voice_channel_id, text_channel_id) DO NOTHING;
                ",
                )
                .bind(&guild_table_id_string)
                .bind(&voice_channel_id_string)
                .bind(text_channel_id.to_string())
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

static DB_CACHE: LazyLock<TokioRwLock<HashMap<GuildId, Option<GuildData>>>> =
    LazyLock::new(|| TokioRwLock::new(HashMap::new()));

#[derive(Debug, Clone)]
pub struct GuildData {
    pub guild_id: GuildId,
    pub dict: HashMap<String, String>,

    /// HashMap<VoiceChannelId, HashSet<読み上げるチャンネル>>
    pub autojoin_channels: HashMap<ChannelId, HashSet<ChannelId>>,
    pub options: GuildOptions,
}

impl GuildData {
    pub async fn from(guild_id: GuildId) -> Result<GuildData, sqlx::Error> {
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
        GuildData {
            guild_id,
            dict: HashMap::new(),
            options: GuildOptions::default(),
            autojoin_channels: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct GuildDataMut<'a> {
    pub guild_id: GuildId,
    pub dict: HashMap<String, String>,

    /// HashMap<VoiceChannelId, HashSet<読み上げるチャンネル>>
    pub autojoin_channels: HashMap<ChannelId, HashSet<ChannelId>>,
    pub options: GuildOptions,

    cache_lock: TokioRwLockWriteGuard<'a, HashMap<GuildId, Option<GuildData>>>,
}

impl GuildDataMut<'_> {
    pub async fn from<'a>(guild_id: GuildId) -> Result<GuildDataMut<'a>, sqlx::Error> {
        let guilddata = GuildData::from(guild_id).await?;

        Ok(GuildDataMut {
            guild_id,
            dict: guilddata.dict,
            autojoin_channels: guilddata.autojoin_channels,
            options: guilddata.options,
            cache_lock: DB_CACHE.write().await,
        })
    }

    pub async fn update(self) -> Result<(), sqlx::Error> {
        let mut db_cache = self.cache_lock;

        let guild_data = GuildData {
            guild_id: self.guild_id,
            dict: self.dict,
            options: self.options,
            autojoin_channels: self.autojoin_channels,
        };

        GuildDatabase::update(guild_data.clone()).await?;
        db_cache.insert(self.guild_id, Some(guild_data));

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GuildOptions {
    pub is_dic_onlyadmin: bool,
    pub is_entrance_exit_log: bool,
    pub is_entrance_exit_play: bool,
    pub is_notice_attachment: bool,
    pub is_if_long_fastread: bool,
}

impl Default for GuildOptions {
    fn default() -> Self {
        Self {
            is_dic_onlyadmin: true,
            is_entrance_exit_log: false,
            is_entrance_exit_play: false,
            is_notice_attachment: false,
            is_if_long_fastread: false,
        }
    }
}

#[cfg(test)]
mod tests_guild_data_base {
    use tokio::fs::create_dir_all;

    use crate::init_database;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_update() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        let dict: HashMap<_, _> = [("A1", "B1"), ("A2", "B2"), ("A3", "B3")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let autojoin_channels: HashMap<ChannelId, HashSet<ChannelId>> = HashMap::from([
            (
                12345.into(),
                HashSet::from([1.into(), 2.into(), 3.into(), 4.into(), 5.into()]),
            ),
            (
                678910.into(),
                HashSet::from([6.into(), 7.into(), 8.into(), 9.into(), 10.into()]),
            ),
        ]);

        GuildDatabase::update(GuildData {
            guild_id: GuildId::new(123),
            dict,
            options: GuildOptions {
                is_dic_onlyadmin: true,
                is_entrance_exit_log: true,
                is_entrance_exit_play: false,
                is_notice_attachment: true,
                is_if_long_fastread: false,
            },
            autojoin_channels,
        })
        .await?;

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_from() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        for i in [123, 456] {
            let guild_data = GuildDatabase::from(GuildId::new(i)).await.unwrap();
            dbg!(guild_data);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests_guild_data {
    use serenity::all::GuildId;
    use tokio::fs::create_dir_all;

    use crate::init_database;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_from() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        for i in [1, 123, 456, 1, 123, 456] {
            dbg!(GuildData::from(GuildId::new(i)).await.unwrap());
            {
                let db_cache = DB_CACHE.read().await;
                dbg!(&db_cache);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests_guilddata_mut {
    use serenity::all::GuildId;
    use tokio::fs::create_dir_all;

    use crate::init_database;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_from() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        for i in [1, 123, 456] {
            dbg!(GuildDataMut::from(GuildId::new(i)).await.unwrap());
        }

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_update() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        let guild_id = GuildId::from(123);

        for i in [true, false, true, false] {
            {
                let mut guilddata_mut = GuildDataMut::from(guild_id).await?;
                dbg!(guilddata_mut.options.is_dic_onlyadmin);

                guilddata_mut.options.is_dic_onlyadmin = i;
                guilddata_mut.update().await?;
            }

            let guild_data = GuildData::from(guild_id).await?;
            dbg!(guild_data.options.is_dic_onlyadmin);
        }

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_update_dict() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        let guild_id = GuildId::from(123);
        {
            let mut guilddata_mut = GuildDataMut::from(guild_id).await?;
            dbg!(guilddata_mut.options.is_dic_onlyadmin);

            guilddata_mut.dict = HashMap::from([]);

            guilddata_mut.update().await?;
        }

        Ok(())
    }
}
