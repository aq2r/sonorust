mod guild;
mod user;

pub use guild::GuildData;
pub use guild::GuildDataMut;
pub use guild::GuildOptions;
pub use user::UserData;
pub use user::UserDataMut;

use guild::GuildOptionsStr;

use std::{fs::File, path::PathBuf, str::FromStr, sync::OnceLock};

use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

static DB_POOL: OnceLock<SqlitePool> = OnceLock::new();

pub async fn init_database(database_path: &str) -> Result<(), sqlx::Error> {
    let database_pathbuf: PathBuf = database_path.into();

    if !database_pathbuf.exists() {
        File::create(&database_pathbuf)?;
    }

    let options = SqliteConnectOptions::from_str(database_path)?.pragma("foreign_keys", "ON");
    let pool = SqlitePool::connect_with(options).await?;

    let mut tx = pool.begin().await?;

    let sqls = [
        // user table
        "
        CREATE TABLE IF NOT EXISTS user (
            id INTEGER PRIMARY KEY,
            discord_id INTEGER NOT NULL UNIQUE,
            model_name TEXT NOT NULL,
            speaker_name TEXT NOT NULL,
            style_name TEXT NOT NULL,
            length REAL NOT NULL
        );
        ",
        // guild table
        "
        CREATE TABLE IF NOT EXISTS guild (
            id INTEGER PRIMARY KEY,
            discord_id INTEGER NOT NULL UNIQUE
        );
        ",
        // guild_option table
        "
        CREATE TABLE IF NOT EXISTS guild_option (
            id INTEGER PRIMARY KEY,
            option_name TEXT NOT NULL UNIQUE
        );
        ",
        // guild と guild_option の中間テーブル
        "
        CREATE TABLE IF NOT EXISTS guild_guild_options (
            id INTEGER PRIMARY KEY,
            guild_table_id INTEGER NOT NULL,
            guild_option_table_id INTEGER NOT NULL,

            FOREIGN KEY (guild_table_id) REFERENCES guild(id),
            FOREIGN KEY (guild_option_table_id) REFERENCES guild_option(id),
            UNIQUE (guild_table_id, guild_option_table_id)
        );
        ",
        // guild_dict table
        "
        CREATE TABLE IF NOT EXISTS guild_dict (
            id INTEGER PRIMARY KEY,
            guild_table_id INTEGER NOT NULL,
            before_text TEXT NOT NULL,
            after_text TEXT NOT NULL,

            FOREIGN KEY (guild_table_id) REFERENCES guild(id),
            UNIQUE (guild_table_id, before_text)
        );
        ",
        // guild_auto_join
        "
        CREATE TABLE IF NOT EXISTS guild_auto_join (
            id INTEGER PRIMARY KEY,
            guild_table_id INTEGER NOT NULL,
            voice_channel_id INTEGER NOT NULL,
            text_channel_id INTEGER NOT NULL,

            FOREIGN KEY (guild_table_id) REFERENCES guild(id)
            UNIQUE (voice_channel_id, text_channel_id)
        );
        ",
    ];

    for i in sqls {
        sqlx::query(i).execute(&mut *tx).await?;
    }

    // ギルドオプションの追加
    let guild_options = [
        GuildOptionsStr::IsDicOnlyAdmin,
        GuildOptionsStr::IsEntranceExitLog,
        GuildOptionsStr::IsEntranceExitPlay,
        GuildOptionsStr::IsIfLongFastRead,
        GuildOptionsStr::IsNoticeAttachment,
    ];

    for i in guild_options {
        sqlx::query("INSERT INTO guild_option (option_name) VALUES (?1) ON CONFLICT DO NOTHING")
            .bind(i.as_str())
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    DB_POOL.set(pool).expect("Failed to set DB_POOL");
    Ok(())
}

mod tests {
    #[ignore]
    #[tokio::test]
    async fn test_init_database() -> anyhow::Result<()> {
        use super::*;
        use tokio::fs::create_dir_all;

        create_dir_all("appdata").await?;
        init_database("appdata/database.db").await?;

        Ok(())
    }
}
