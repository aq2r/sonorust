use std::{
    collections::{HashMap, VecDeque},
    sync::{LazyLock, RwLock},
    time::Duration,
};

use langrustang::lang_t;
use sbv2_api::{Sbv2Client, Sbv2InferParam, Sbv2ModelInfo, ValidModel};
use serenity::all::{ChannelId, Context, GuildId, UserId};
use setting_inputter::{settings_json::SETTINGS_JSON, SettingsJson};
use songbird::input::Input;
use sonorust_db::{GuildData, UserData};

use crate::errors::SonorustError;

use super::SettingsJsonExtension;

/// ギルドの読み上げるチャンネルを登録しておく
pub static READ_CHANNELS: LazyLock<RwLock<HashMap<GuildId, ChannelId>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 読み上げるチャンネルの queue
pub static CHANNEL_QUEUES: LazyLock<RwLock<HashMap<ChannelId, VecDeque<(String, UserId)>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub trait Sbv2ClientExtension {
    fn get_valid_model_from_userdata(userdata: &UserData) -> ValidModel;

    async fn infer_from_user(text: &str, userdata: &UserData) -> anyhow::Result<Vec<u8>>;

    async fn play_on_voice_channel(
        ctx: &Context,
        guild_id: Option<GuildId>,
        channel_id: ChannelId,
        user_id: UserId,
        play_content: &str,
    ) -> Result<(), SonorustError>;
}

impl Sbv2ClientExtension for Sbv2Client {
    fn get_valid_model_from_userdata(userdata: &UserData) -> ValidModel {
        let default_model = {
            let lock = SETTINGS_JSON.read().unwrap();
            lock.default_model.clone()
        };

        Sbv2ModelInfo::get_valid_model(
            &userdata.model_name,
            &userdata.speaker_name,
            &userdata.style_name,
            &default_model,
        )
    }

    async fn infer_from_user(text: &str, userdata: &UserData) -> anyhow::Result<Vec<u8>> {
        let client = SettingsJson::get_sbv2_client();
        let infer_lang = SettingsJson::get_infer_lang();

        let valid_model = Sbv2Client::get_valid_model_from_userdata(userdata);

        let param = Sbv2InferParam {
            model_id: valid_model.model_id,
            speaker_id: valid_model.speaker_id,
            style_name: valid_model.style_name,
            length: userdata.length,
            language: infer_lang.to_string(),
        };

        client.infer(text, param).await
    }

    async fn play_on_voice_channel(
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

        let lang = SettingsJson::get_bot_lang();

        // ボイスチャンネルに参加していないサーバーの場合無視する
        let Some(handler_lock) = manager.get(guild_id) else {
            return Ok(());
        };

        // join時に登録した読み上げる対象のチャンネルを取得する
        let read_ch = {
            let read_channels = READ_CHANNELS.read().unwrap();

            match read_channels.get(&guild_id) {
                Some(ch) => *ch,
                None => return Ok(()),
            }
        };

        // 読み上げる対象のチャンネルでない場合無視する
        if channel_id != read_ch {
            return Ok(());
        }

        // そのチャンネルのqueueにメッセージを追加する
        {
            let mut channel_queues = CHANNEL_QUEUES.write().unwrap();
            let Some(read_ch_queue) = channel_queues.get_mut(&channel_id) else {
                log::error!(lang_t!("log.fail_ch_queue"));
                return Ok(());
            };

            read_ch_queue.push_front((play_content.to_string(), user_id));

            // もし再生待ちが1つだけなら再生に移る
            // (下の方ではqueueがなくなるまで繰り返すため)
            if read_ch_queue.len() != 1 {
                return Ok(());
            }
        }

        let infer_lang = SettingsJson::get_infer_lang();

        // そのチャンネルのqueueがなくなるまで繰り返す
        loop {
            // 次に再生する文章とユーザーを取り出す
            let (play_content, user_id) = {
                // そのチャンネルのqueueを取得
                let mut channel_queues = CHANNEL_QUEUES.write().unwrap();
                let Some(read_ch_queue) = channel_queues.get_mut(&channel_id) else {
                    log::error!(lang_t!("log.fail_ch_queue"));
                    return Ok(());
                };

                // すべてを再生し終えたらreturnして終了する
                match read_ch_queue.back() {
                    Some(s) => s.clone(),
                    None => return Ok(()),
                }
            };

            let Ok(mut userdata) = UserData::from(user_id).await else {
                log::error!(lang_t!("log.fail_update_guilddata"));
                return Ok(());
            };

            let Ok(guilddata) = GuildData::from(guild_id).await else {
                log::error!(lang_t!("log.fail_get_userdata"));
                return Ok(());
            };

            // オプションがオンになっていて一定の文字数より多い場合、素早く読む
            let fastread_border = {
                use setting_inputter::settings_json::SettingLang::*;

                match infer_lang {
                    Ja => 30,
                    En => 80,
                }
            };

            if guilddata.options.is_if_long_fastread
                && play_content.chars().count() >= fastread_border
            {
                userdata.length = 0.5;
            }

            // 音声を生成
            // API が起動していなく、音声を生成できなかった場合 VC から退出する
            let Ok(voice_data) = Sbv2Client::infer_from_user(&play_content, &userdata).await else {
                if let Err(err) = manager.remove(guild_id).await {
                    log::error!("{}: {err}", lang_t!("log.fail_leave_vc"))
                }
                channel_id
                    .say(&ctx.http, lang_t!("msg.failed.infer", lang))
                    .await?;
                return Ok(());
            };

            // 再生時間を求める
            // ビット数 = Vec<u8>の数 * 8
            // 1秒あたりの情報量 = 44.1 kHz * 16 bit
            let voice_playtime = (voice_data.len() * 8) as f64 / (44100.0 * 16.0);

            // 音声を VC で作成
            let input = Input::from(voice_data);
            {
                let mut handler = handler_lock.lock().await;

                let track_handle = handler.play_input(input);
                if let Err(err) = track_handle.set_volume(0.1) {
                    log::error!("{}: {err}", lang_t!("log.fail_adj_vol"))
                }
            }

            // その音声の再生時間だけスリープする
            let duration = Duration::from_secs_f64(voice_playtime);
            tokio::time::sleep(duration).await;

            // 再生した音声を queue から削除する
            {
                let mut channel_queues = CHANNEL_QUEUES.write().unwrap();
                let Some(read_ch_queue) = channel_queues.get_mut(&channel_id) else {
                    log::error!(lang_t!("log.fail_get_userdata"));
                    return Ok(());
                };

                read_ch_queue.pop_back();
            }
        }
    }
}
