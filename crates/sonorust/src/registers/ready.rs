use std::sync::{LazyLock, RwLock};

use serenity::all::{Command, Context, Ready, UserId};

use crate::registers::slash_commands;

/// BOT の所有者のユーザーID
pub static APP_OWNER_ID: LazyLock<RwLock<Option<UserId>>> = LazyLock::new(|| RwLock::new(None));

pub async fn ready(ctx: &Context, ready: &Ready) {
    log::info!("{} is connected!", ready.user.name);

    // BOT の所有者のユーザーIDを変数に保存
    let app_owner = ctx
        .http
        .get_current_application_info()
        .await
        .and_then(|i| Ok(i.owner));

    if let Ok(Some(owner)) = app_owner {
        let mut app_owner_id = APP_OWNER_ID.write().unwrap();
        *app_owner_id = Some(owner.id);
        log::debug!("Updated app_owner_id: {:?}", app_owner_id);
    }

    // テスト用: 環境変数 (IS_SYNC_SLASH) が false なら同期しない
    let is_sync_slash: bool = match std::env::var("IS_SYNC_SLASH").map(|i| i.parse()) {
        Ok(Ok(b)) => b,
        _ => true,
    };

    if !is_sync_slash {
        return;
    }

    // スラッシュコマンドの登録
    log::info!("Registering SlashCommands...");

    let commands = slash_commands::registers();
    match Command::set_global_commands(&ctx.http, commands).await {
        Ok(_) => log::info!("Slash command has been registered."),
        Err(_) => log::error!("Failed to register slash command."),
    }
}
