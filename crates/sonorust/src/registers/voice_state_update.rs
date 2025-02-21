use std::collections::{HashMap, VecDeque};

use langrustang::lang_t;
use serenity::{
    all::{ChannelId, Context, GuildId, UserId, VoiceState},
    futures::{stream::FuturesUnordered, StreamExt},
};
use sonorust_db::GuildData;

use crate::{
    crate_extensions::{
        infer_api::InferApiExt, rwlock::RwLockExt, sonorust_setting::SettingJsonExt,
    },
    errors::SonorustError,
    Handler,
};

pub async fn voice_state_update(
    handler: &Handler,
    ctx: &Context,
    old: &Option<VoiceState>,
    new: &VoiceState,
) -> Result<(), SonorustError> {
    let mut autojoin_future = None;
    let mut auto_leave_future = None;
    let mut log_play_futures = FuturesUnordered::new();

    let fn_log_play = |channel_id, user_action| {
        entrance_exit_log_play(
            handler,
            ctx,
            new.guild_id,
            channel_id,
            new.user_id,
            user_action,
        )
    };

    if let Some(old) = old {
        if new.channel_id != old.channel_id {
            if let Some(channel_id) = old.channel_id {
                log_play_futures.push(fn_log_play(channel_id, UserAction::Exit));
            }
            if let Some(channel_id) = new.channel_id {
                log_play_futures.push(fn_log_play(channel_id, UserAction::Entrance));
                autojoin_future = Some(auto_join(handler, &ctx, new.guild_id, new.user_id));
            }

            auto_leave_future = Some(auto_leave(&ctx, new.guild_id));
        }
    } else {
        if let Some(channel_id) = new.channel_id {
            log_play_futures.push(fn_log_play(channel_id, UserAction::Entrance));
            autojoin_future = Some(auto_join(handler, &ctx, new.guild_id, new.user_id));
        }
    }

    // それぞれを同時実行
    let log_play_futures = async {
        while let Some(result) = log_play_futures.next().await {
            result?;
        }
        Ok::<(), SonorustError>(())
    };

    let autojoin_future = async {
        if let Some(future) = autojoin_future {
            future.await?;
        }
        Ok::<(), SonorustError>(())
    };

    let autoleave_future = async {
        if let Some(future) = auto_leave_future {
            future.await;
        }
        Ok::<(), SonorustError>(())
    };

    let (r1, r2, r3) = tokio::join!(autojoin_future, log_play_futures, autoleave_future);

    r1?;
    r2?;
    r3?;

    Ok(())
}

async fn auto_join(
    handler: &Handler,
    ctx: &Context,
    guild_id: Option<GuildId>,
    user_id: UserId,
) -> Result<(), SonorustError> {
    let lang = handler.setting_json.get_bot_lang();

    // サーバー内ではない場合何もしない
    let Some(guild_id) = guild_id else {
        return Ok(());
    };

    // もしそのサーバーのVCにすでにいる場合何もしない
    let manager = songbird::get(ctx).await.unwrap().clone();
    if let Some(_) = manager.get(guild_id) {
        return Ok(());
    }

    // ユーザーがいるVCを取得する処理
    // 使用したユーザーがVCに参加していない場合返す
    let in_user_channel = {
        let guild = guild_id.to_guild_cached(&ctx.cache).unwrap();
        let ch = guild.voice_states.get(&user_id).and_then(|v| v.channel_id);

        // そのチャンネル id にいるユーザーリストを取得
        let voice_states: &HashMap<UserId, VoiceState> = &guild.voice_states;
        let in_vc_users = voice_states
            .iter()
            .filter(|(_, v)| v.channel_id == ch)
            .map(|(k, _)| *k)
            .collect::<Vec<_>>();

        // ユーザーが1人になった時のみ
        if in_vc_users.len() != 1 {
            return Ok(());
        }

        match ch {
            Some(user_vc) => user_vc,
            None => return Ok(()),
        }
    };

    // そのサーバーの参加リストに含まれるか
    let guilddata = GuildData::from(guild_id).await?;

    let join_set = match guilddata.autojoin_channels.get(&in_user_channel) {
        Some(set) => set,
        None => return Ok(()),
    };

    // サーバーIDと読み上げるチャンネルIDのペアを登録
    handler
        .read_channels
        .with_write(|lock| lock.insert(guild_id, join_set.clone()));

    // 読み上げ queue を初期化
    handler
        .channel_queues
        .with_write(|lock| lock.insert(guild_id, VecDeque::new()));

    // 参加
    match manager.join(guild_id, in_user_channel).await {
        Ok(handler) => {
            let mut lock = handler.lock().await;
            let _ = lock.deafen(true).await;
        }
        Err(_) => {
            log::error!(lang_t!("log.fail_join_vc"));
            return Ok(());
        }
    }

    // メッセージ送信や音声再生を同時実行
    let mut tasks = FuturesUnordered::new();

    for i in join_set.iter() {
        tasks.push(i.say(&ctx.http, lang_t!("join.connected", lang)));
    }

    let send_connected_future = async {
        while let Some(item) = tasks.next().await {
            item?;
        }
        Ok::<(), SonorustError>(())
    };
    let mut play_connected = None;

    // 接続しました 音声
    if let Some(ch) = join_set.iter().nth(0) {
        play_connected = Some(handler.infer_client.play_on_vc(
            handler,
            ctx,
            Some(guild_id),
            *ch,
            user_id,
            lang_t!("join.connected", lang),
        ));
    }

    let play_connected_future = async {
        if let Some(future) = play_connected {
            future.await?;
        }
        Ok::<(), SonorustError>(())
    };

    let (r1, r2) = tokio::join!(send_connected_future, play_connected_future);
    r1?;
    r2?;

    log::debug!(
        "Auto joined: {{ GuildID: {}, ChannelID: {} }}",
        guild_id,
        in_user_channel
    );

    Ok(())
}

async fn auto_leave(ctx: &Context, guild_id: Option<GuildId>) {
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

#[derive(Debug, Clone, Copy)]
enum UserAction {
    Entrance,
    Exit,
}

/// 入退出のログを残す処理
async fn entrance_exit_log_play(
    handler: &Handler,
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
    let log_channels = {
        let read_channels = handler.read_channels.write().unwrap();

        match read_channels.get(&guild_id) {
            Some(ch) => ch.clone(),
            None => return Ok(()),
        }
    };

    // サーバーデータの取得
    let guild_data = GuildData::from(guild_id).await?;

    // 入退出ログと入退出音声通知の並列実行
    {
        let mut log_future = None;
        let mut voice_future = None;

        if guild_data.options.is_entrance_exit_log {
            log_future = Some(async {
                // メッセージを送信
                let msg = match user_action {
                    UserAction::Entrance => format!("> **{}** さんが参加しました。", user_name),
                    UserAction::Exit => format!("> **{}** さんが退席しました。", user_name),
                };

                let mut tasks = FuturesUnordered::new();

                for i in log_channels.iter() {
                    tasks.push(i.say(&ctx.http, &msg));
                }

                while let Some(item) = tasks.next().await {
                    item?;
                }

                Ok::<(), SonorustError>(())
            });
        };

        if guild_data.options.is_entrance_exit_play {
            voice_future = Some(async {
                let msg = match user_action {
                    UserAction::Entrance => format!("{} さんが参加しました。", user_name),
                    UserAction::Exit => format!("{} さんが退席しました。", user_name),
                };

                if let Some(channel) = log_channels.iter().next() {
                    handler
                        .infer_client
                        .play_on_vc(handler, ctx, Some(guild_id), *channel, user_id, &msg)
                        .await?;
                }

                Ok::<(), SonorustError>(())
            })
        }

        let future_1 = async {
            if let Some(future) = log_future {
                future.await?;
            }
            Ok::<(), SonorustError>(())
        };
        let future_2 = async {
            if let Some(future) = voice_future {
                future.await?;
            }
            Ok::<(), SonorustError>(())
        };

        let (r1, r2) = tokio::join!(future_1, future_2);
        r1?;
        r2?;
    }

    Ok(())
}
