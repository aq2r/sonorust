use std::collections::VecDeque;

use langrustang::lang_t;
use serenity::all::{ChannelId, Context, CreateCommand, GuildId, UserId};
use setting_inputter::SettingsJson;

use crate::crate_extensions::{
    sbv2_api::{CHANNEL_QUEUES, READ_CHANNELS},
    SettingsJsonExtension,
};

pub async fn join(
    ctx: &Context,
    guild_id: Option<GuildId>,
    channel_id: ChannelId,
    user_id: UserId,
) -> Result<&'static str, &'static str> {
    let lang = SettingsJson::get_bot_lang();

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
    let user_vc = {
        let Some(guild) = guild_id.to_guild_cached(&ctx.cache) else {
            log::error!(lang_t!("log.fail_get_guild"));
            return Err(lang_t!("join.cannot_connect", lang));
        };

        guild.voice_states.get(&user_id).and_then(|v| v.channel_id)
    };

    // 使用したユーザーが VC に参加していない場合返す
    let Some(connect_ch) = user_vc else {
        return Err(lang_t!("join.after_connecting", lang));
    };

    // もし VC に参加できなかったら返す
    if let Err(err) = manager.join(guild_id, connect_ch).await {
        log::error!("{}: {err}", lang_t!("log.fail_join_vc"));
        return Err(lang_t!("join.cannot_connect", lang));
    } else {
        log::debug!(
            "Joined voice channel (name: {} id: {})",
            guild_id
                .name(&ctx.cache)
                .unwrap_or_else(|| "Unknown".to_string()),
            guild_id,
        );
    }

    // サーバーIDと読み上げるチャンネルIDのペアを登録
    {
        let mut read_channels = READ_CHANNELS.write().unwrap();
        read_channels.insert(guild_id, channel_id);
    }

    // 読み上げ queue を初期化
    {
        let mut channel_queues = CHANNEL_QUEUES.write().unwrap();
        channel_queues.insert(channel_id, VecDeque::new());
    }

    Ok(lang_t!("join.connected", lang))
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("join").description(lang_t!("join.command.description", lang))
}
