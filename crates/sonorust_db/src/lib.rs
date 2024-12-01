mod errors;
mod guild;
mod user;

pub use errors::SonorustDBError;
pub use guild::{GuildData, GuildDataMut, GuildOptions};
pub use user::{UserData, UserDataMut};

use std::{
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use rusqlite::{params, Connection};

static DATABASE_PATH: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("./appdata/database.db"));

static DATABASE_CONN: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    std::fs::create_dir_all(DATABASE_PATH.ancestors().nth(1).unwrap()).expect("Can't open file.");

    let mut conn = Connection::open(DATABASE_PATH.as_path()).expect("Can't open file");
    init_database(&mut conn).expect("Failed to init database.");
    Mutex::new(conn)
});

pub fn init_database(conn: &mut Connection) -> Result<(), SonorustDBError> {
    let mut result = || {
        let txn = conn.transaction()?;

        // 設定の変更
        txn.execute("PRAGMA foreign_keys = ON;", ())?;

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
        ];

        for i in sqls {
            txn.execute(i, ())?;
        }

        // ギルドオプションの追加
        let guild_options = [
            "is_dic_onlyadmin",
            "is_auto_join",
            "is_entrance_exit_log",
            "is_entrance_exit_play",
            "is_notice_attachment",
            "is_if_long_fastread",
        ];

        for i in guild_options {
            txn.execute(
                "INSERT INTO guild_option (option_name) VALUES (?1) ON CONFLICT DO NOTHING",
                params![i],
            )?;
        }

        txn.commit()?;
        Ok(())
    };

    result().map_err(|err| SonorustDBError::InitDatabase(err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[test]
    fn test_init_database() {
        let _ = *DATABASE_CONN;
    }
}
