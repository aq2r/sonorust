use std::{
    collections::{HashMap, VecDeque},
    sync::{LazyLock, RwLock},
};

use sbv2_api::{Sbv2Client, Sbv2InferParam, Sbv2ModelInfo, ValidModel};
use serenity::all::{ChannelId, GuildId, UserId};
use setting_inputter::{settings_json::SETTINGS_JSON, SettingsJson};

use sonorust_db::UserData;

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
}
