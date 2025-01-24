use std::collections::{HashMap, VecDeque};

use langrustang::{format_t, lang_t};
use serenity::all::{ChannelId, Context, GuildId, UserId, VoiceState};
use setting_inputter::SettingsJson;
use sonorust_db::GuildData;

use crate::{
    crate_extensions::{
        play_on_voice_channel,
        sbv2_api::{CHANNEL_QUEUES, READ_CHANNELS},
        SettingsJsonExtension,
    },
    errors::SonorustError,
};

use super::message::TextReplace;

#[derive(Debug, Clone, Copy)]
enum UserAction {
    Entrance,
    Exit,
}

pub async fn voice_state_update(
    ctx: Context,
    old: Option<VoiceState>,
    new: VoiceState,
) -> Result<(), SonorustError> {
    auto_join(&ctx, new.guild_id, new.user_id).await?;

    let fn_log_play = |channel_id, user_action| {
        entrance_exit_log_play(&ctx, new.guild_id, channel_id, new.user_id, user_action)
    };

    if let Some(old) = old {
        if new.channel_id != old.channel_id {
            if let Some(channel_id) = old.channel_id {
                fn_log_play(channel_id, UserAction::Exit).await?;
            }
            if let Some(channel_id) = new.channel_id {
                fn_log_play(channel_id, UserAction::Entrance).await?;
            }

            auto_exit(&ctx, new.guild_id).await;
        }
    } else {
        if let Some(channel_id) = new.channel_id {
            fn_log_play(channel_id, UserAction::Entrance).await?;
        }
    }

    Ok(())
}

/// もし自分だけになったら自動退出する処理
async fn auto_exit(ctx: &Context, guild_id: Option<GuildId>) {
    // サーバー内ではない場合何もしない
    let Some(guild_id) = guild_id else {
        return;
    };

    let manager = songbird::get(ctx).await.unwrap();

    // 自分が vc に参加していないなら何もしない
    if let None = manager.get(guild_id) {
        return;
    }

    let in_vc_users = {
        let guild = match guild_id.to_guild_cached(&ctx.cache) {
            Some(guild) => guild,
            None => return,
        };

        let voice_states: &HashMap<UserId, VoiceState> = &guild.voice_states;

        // 自分自身がいるボイスチャンネルの id を取得
        let self_in_vc = match voice_states.get(&ctx.cache.current_user().id) {
            Some(ch) => ch.channel_id,
            None => return,
        };

        // そのチャンネル id にいるユーザーリストを取得
        voice_states
            .iter()
            .filter(|(_, v)| v.channel_id == self_in_vc) // 同じVCにいるユーザーだけ取得
            .map(|(k, _)| *k) // UserId だけ取得
            .collect::<Vec<_>>()
    };

    // 自分だけではないなら何もしない
    if in_vc_users.len() != 1 {
        return;
    }

    // ボイスチャンネルから切断する
    let _ = manager.remove(guild_id).await;

    log::debug!("Auto exited: {{ GuildID: {} }}", guild_id);
}

/// サーバー設定で自動参加が ON になっているなら自動参加する処理
async fn auto_join(
    ctx: &Context,
    guild_id: Option<GuildId>,
    user_id: UserId,
) -> Result<(), SonorustError> {
    // サーバー内ではない場合何もしない
    let Some(guild_id) = guild_id else {
        return Ok(());
    };

    let client = SettingsJson::get_sbv2_client();

    // Api が起動していない場合何もしない
    if !client.is_api_activation().await {
        return Ok(());
    }

    // もしそのサーバーのVCにすでにいる場合何もしない
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(_) = manager.get(guild_id) {
        return Ok(());
    }

    // そのサーバーで VC 自動参加が ON になっていないなら何もしない
    let guild_data = GuildData::from(guild_id).await?;

    if !guild_data.options.is_auto_join {
        return Ok(());
    }

    // ユーザーがいるVCを取得する処理
    let user_vchannel = {
        let guild = guild_id.to_guild_cached(&ctx.cache).unwrap();
        guild.voice_states.get(&user_id).and_then(|v| v.channel_id)
    };

    // 使用したユーザーがVCに参加していない場合返す
    let connect_channel = match user_vchannel {
        Some(user_vc) => user_vc,
        None => return Ok(()),
    };

    // もしVCに参加できなかったら返す
    if let Err(_) = manager.join(guild_id, connect_channel).await {
        log::error!(lang_t!("log.fail_join_vc"));
        return Ok(());
    }

    // サーバーIDと読み上げるチャンネルIDのペアを登録
    {
        let mut read_channels = READ_CHANNELS.write().unwrap();
        read_channels.insert(guild_id, connect_channel);
    }

    // 読み上げ queue を初期化
    {
        let mut channel_queues = CHANNEL_QUEUES.write().unwrap();
        channel_queues.insert(connect_channel, VecDeque::new());
    }

    // メッセージを送信して VC で再生
    let lang = SettingsJson::get_bot_lang();

    let _ = connect_channel
        .say(&ctx.http, lang_t!("join.connected", lang))
        .await;

    let _ = play_on_voice_channel(
        ctx,
        Some(guild_id),
        connect_channel,
        user_id,
        lang_t!("join.connected", lang),
    )
    .await;

    log::debug!(
        "Auto joined: {{ GuildID: {}, ChannelID: {} }}",
        guild_id,
        connect_channel
    );

    Ok(())
}

/// 入退出のログを残す処理
async fn entrance_exit_log_play(
    ctx: &Context,
    guild_id: Option<GuildId>,
    channel_id: ChannelId,
    user_id: UserId,
    user_action: UserAction,
) -> Result<(), SonorustError> {
    let Some(guild_id) = guild_id else {
        return Ok(());
    };

    // 自分自身の変更の場合何もしない
    if user_id == ctx.cache.current_user().id {
        return Ok(());
    };

    // もしそのサーバーのVCにいない場合何もしない
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let None = manager.get(guild_id) {
        return Ok(());
    }

    // 自分自身がいるボイスチャンネルの id を取得してそのチャンネルでの変更か確認
    let self_in_vc = {
        let guild = match guild_id.to_guild_cached(&ctx.cache) {
            Some(guild) => guild,
            None => return Ok(()),
        };

        let voice_states: &HashMap<UserId, VoiceState> = &guild.voice_states;
        match voice_states.get(&ctx.cache.current_user().id) {
            Some(ch) => ch.channel_id,
            None => return Ok(()),
        }
    };

    if Some(channel_id) != self_in_vc {
        return Ok(());
    }

    // 変更があったユーザー名を取得 できなかった場合リターン
    let Ok(user) = user_id.to_user(&ctx.http).await else {
        return Ok(());
    };

    let user_name = user
        .nick_in(&ctx.http, guild_id)
        .await
        .unwrap_or_else(|| user.global_name.unwrap_or_else(|| user.name));

    // 読み上げているチャンネルを取得 取得できなかった場合リターン
    let log_channel = {
        let read_channels = READ_CHANNELS.read().unwrap();

        match read_channels.get(&guild_id) {
            Some(ch) => *ch,
            None => return Ok(()),
        }
    };

    // サーバーデータの取得
    let guild_data = GuildData::from(guild_id).await?;
    if guild_data.options.is_entrance_exit_log {
        // メッセージを送信
        let msg = match user_action {
            UserAction::Entrance => format!("> **{}** さんが参加しました。", user_name),
            UserAction::Exit => format!("> **{}** さんが退席しました。", user_name),
        };

        log_channel.say(&ctx.http, msg).await?;
    };

    // チャンネルで読み上げ
    if guild_data.options.is_entrance_exit_play {
        let lang = SettingsJson::get_bot_lang();

        let user_name_r = {
            use crate::_langrustang_autogen::Lang::*;

            match lang {
                Ja => {
                    let mut text_replace = TextReplace::new(user_name);
                    text_replace.eng_to_kana();

                    text_replace.as_string()
                }
                _ => user_name,
            }
        };

        let msg = match user_action {
            UserAction::Entrance => format_t!("msg.vc_joined", lang, user_name_r),
            UserAction::Exit => format_t!("msg.vc_leaved", lang, user_name_r),
        };

        if let Err(why) =
            play_on_voice_channel(ctx, Some(guild_id), log_channel, user_id, &msg).await
        {
            log::error!("{}: {}", lang_t!("log.err_send_msg"), why);
        };
    }

    Ok(())
}
