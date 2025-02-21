use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{errors::SonorustError, Handler};
use either::Either;
use infer_api::{Sbv2PythonClient, Sbv2PythonInferParam, Sbv2RustClient};
use langrustang::lang_t;
use serenity::all::{ChannelId, Context, GuildId, UserId};
use songbird::input::Input;
use sonorust_db::{GuildData, UserData};
use sonorust_setting::{InferLang, InferUse, SettingJson};
use tokio::sync::RwLock as TokioRwLock;

use super::rwlock::RwLockExt;

type ArcRwLock<T> = Arc<RwLock<T>>;

pub trait InferApiExt {
    async fn infer_from_user(
        &self,
        text: &str,
        userdata: UserData,
        setting_json: &ArcRwLock<SettingJson>,
    ) -> Result<Vec<u8>, SonorustError>;

    async fn play_on_vc(
        &self,
        handler: &Handler,
        ctx: &Context,
        guild_id: Option<GuildId>,
        channel_id: ChannelId,
        user_id: UserId,
        play_content: &str,
    ) -> Result<(), SonorustError>;
}

impl InferApiExt for TokioRwLock<Either<Sbv2PythonClient, Sbv2RustClient>> {
    async fn infer_from_user(
        &self,
        text: &str,
        userdata: UserData,
        setting_json: &ArcRwLock<SettingJson>,
    ) -> Result<Vec<u8>, SonorustError> {
        let (language, default_model) =
            setting_json.with_read(|lock| (lock.infer_lang.clone(), lock.default_model.clone()));

        let mut lock = self.write().await;
        match lock.as_mut() {
            Either::Left(python_client) => {
                let param = Sbv2PythonInferParam {
                    model_name: userdata.model_name,
                    speaker_name: userdata.speaker_name,
                    style_name: userdata.style_name,
                    length: userdata.length,
                    language: language.to_string(),
                };

                let data = python_client.infer(text, param, &default_model).await?;
                Ok(data)
            }

            Either::Right(rust_client) => {
                let data = rust_client
                    .infer(
                        text,
                        &userdata.model_name,
                        userdata.length as f32,
                        &default_model,
                    )
                    .await?;
                Ok(data)
            }
        }
    }

    async fn play_on_vc(
        &self,
        handler: &Handler,
        ctx: &Context,
        guild_id: Option<GuildId>,
        channel_id: ChannelId,
        user_id: UserId,
        play_content: &str,
    ) -> Result<(), SonorustError> {
        // サーバー上でない場合何もしない
        let Some(guild_id) = guild_id else {
            return Ok(());
        };

        // 自分自身のメッセージの場合無視する
        if user_id == ctx.cache.current_user().id {
            return Ok(());
        }

        // 何も再生するメッセージがない場合何もしない
        if play_content.is_empty() {
            return Ok(());
        }

        let manager = songbird::get(ctx).await.unwrap();
        // ボイスチャンネルに参加していないサーバーの場合無視する
        let Some(handler_lock) = manager.get(guild_id) else {
            return Ok(());
        };

        // 読み上げるチャンネルかどうか確認
        let is_read_ch = {
            let read_channels = handler.read_channels.read().unwrap();

            match read_channels.get(&guild_id) {
                Some(set) => set.contains(&channel_id),
                None => false,
            }
        };

        // 読み上げる対象のチャンネルでない場合無視する
        if !is_read_ch {
            return Ok(());
        }

        // -- 推論
        let mut userdata = UserData::from(user_id).await?;
        let guilddata = GuildData::from(guild_id).await?;

        // オプションがオンになっていて一定の文字数より多い場合、素早く読む
        let fastread_border = handler.setting_json.with_read(|lock| lock.fastread_limit);

        if guilddata.options.is_if_long_fastread
            && play_content.chars().count() >= fastread_border as usize
        {
            userdata.length = 0.5;
        }

        let audio_data = handler
            .infer_client
            .infer_from_user(play_content, userdata, &handler.setting_json)
            .await?;
        // ------

        // そのチャンネルのqueueに音声データを追加する
        {
            let mut channel_queues = handler.channel_queues.write().unwrap();
            let Some(read_ch_queue) = channel_queues.get_mut(&guild_id) else {
                log::error!(lang_t!("log.fail_ch_queue"));
                return Ok(());
            };

            read_ch_queue.push_front(audio_data);

            // もし再生待ちが1つだけなら再生に移る
            // (下の方ではqueueがなくなるまで繰り返すため)
            if read_ch_queue.len() != 1 {
                return Ok(());
            }
        }

        let infer_use = handler.setting_json.with_read(|lock| lock.infer_use);
        loop {
            let voice_data = {
                let mut channel_queues = handler.channel_queues.write().unwrap();
                let Some(read_ch_queue) = channel_queues.get_mut(&guild_id) else {
                    log::error!(lang_t!("log.fail_ch_queue"));
                    return Ok(());
                };

                let mut voice_data = vec![];

                match read_ch_queue.back_mut() {
                    Some(data) => std::mem::swap(&mut voice_data, data),
                    None => return Ok(()),
                }

                voice_data
            };

            // 再生時間を求める
            let voice_playtime = match infer_use {
                InferUse::Python => (voice_data.len() * 8) as f64 / (44100.0 * 16.0),
                InferUse::Rust => (voice_data.len() * 8) as f64 / (44100.0 * 32.0),
            };

            // 音声を VC で作成
            let input = Input::from(voice_data);
            {
                let mut handler = handler_lock.lock().await;

                let track_handle = handler.play_input(input);

                let set_volume = match infer_use {
                    InferUse::Python => track_handle.set_volume(0.1),
                    InferUse::Rust => track_handle.set_volume(0.3),
                };

                if let Err(err) = set_volume {
                    log::error!("{}: {err}", lang_t!("log.fail_adj_vol"))
                }
            }

            // その音声の再生時間だけスリープする
            let duration = Duration::from_secs_f64(voice_playtime);
            tokio::time::sleep(duration).await;

            {
                let mut channel_queues = handler.channel_queues.write().unwrap();
                let Some(read_ch_queue) = channel_queues.get_mut(&guild_id) else {
                    log::error!(lang_t!("log.fail_ch_queue"));
                    return Ok(());
                };

                // すべてを再生し終えたらreturnして終了する
                read_ch_queue.pop_back()
            };
        }
    }
}
