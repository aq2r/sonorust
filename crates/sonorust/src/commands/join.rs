use std::collections::{HashSet, VecDeque};

use langrustang::lang_t;
use serenity::all::{ChannelId, Context, CreateCommand, GuildId, UserId};

use crate::{Handler, _langrustang_autogen::Lang, crate_extensions::rwlock::RwLockExt};

pub async fn join(
    handler: &Handler,
    ctx: &Context,
    lang: Lang,
    guild_id: Option<GuildId>,
    channel_id: ChannelId,
    user_id: UserId,
) -> Result<&'static str, &'static str> {
    // guild_id を取得、DM などの場合返す
    let Some(guild_id) = guild_id else {
        return Err(lang_t!("msg.only_use_guild_2", lang));
    };

    // もしそのサーバーの VC にすでにいる場合返す
    let manager = songbird::get(ctx).await.unwrap();
    if let Some(_) = manager.get(guild_id) {
        return Err(lang_t!("join.already", lang));
    }

    // ユーザーがいる VC を取得する
    let user_in_vc = {
        let Some(guild) = guild_id.to_guild_cached(&ctx.cache) else {
            log::error!(lang_t!("log.fail_get_guild"));
            return Err(lang_t!("join.cannot_connect", lang));
        };

        guild.voice_states.get(&user_id).and_then(|v| v.channel_id)
    };

    // 使用したユーザーが VC に参加していない場合返す
    let Some(connect_ch) = user_in_vc else {
        return Err(lang_t!("join.after_connecting", lang));
    };

    // もし VC に参加できなかったら返す
    match manager.join(guild_id, connect_ch).await {
        Ok(handler) => {
            log::debug!(
                "Joined voice channel (name: {} id: {})",
                guild_id
                    .name(&ctx.cache)
                    .unwrap_or_else(|| "Unknown".to_string()),
                guild_id,
            );

            let mut lock = handler.lock().await;
            let _ = lock.deafen(true).await;
        }
        Err(err) => {
            log::error!("{}: {err}", lang_t!("log.fail_join_vc"));
            return Err(lang_t!("join.cannot_connect", lang));
        }
    }

    // サーバーIDと読み上げるチャンネルIDのペアを登録
    handler
        .read_channels
        .with_write(|lock| lock.insert(guild_id, HashSet::from([channel_id])));

    // 読み上げ queue を初期化
    handler
        .channel_queues
        .with_write(|lock| lock.insert(guild_id, VecDeque::new()));

    Ok(lang_t!("join.connected", lang))
}

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("join").description(lang_t!("join.command.description", lang))
}
