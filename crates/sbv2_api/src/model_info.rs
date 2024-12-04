use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use anyhow::Context as _;

use crate::client::Sbv2Client;

pub static SBV2_MODELINFO: LazyLock<RwLock<Sbv2ModelInfo>> = LazyLock::new(|| {
    RwLock::new(Sbv2ModelInfo {
        name_to_model: HashMap::new(),
        id_to_model: HashMap::new(),
    })
});

/// 特定のモデルの情報を入れておく構造体
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub model_id: u32,
    pub model_name: String,
    pub spk2id: HashMap<String, u32>,
    pub id2spk: HashMap<u32, String>,
    pub style2id: HashMap<String, u32>,
    pub id2style: HashMap<u32, String>,
}

/// APIの全てのモデルの情報を入れておく構造体
#[derive(Debug)]
pub struct Sbv2ModelInfo {
    pub name_to_model: HashMap<String, ModelInfo>,
    pub id_to_model: HashMap<u32, ModelInfo>,
}

/// Sbv2ModelInfoに含まれるモデル
#[derive(Debug)]
pub struct ValidModel {
    pub model_name: String,
    pub speaker_name: String,
    pub style_name: String,
    pub model_id: u32,
    pub speaker_id: u32,
}

impl Sbv2ModelInfo {
    pub(crate) async fn update_modelinfo(client: &Sbv2Client) -> anyhow::Result<()> {
        let url = {
            let host = client.host.as_str();
            let port = client.port;

            format!("http://{host}:{port}/models/refresh")
        };

        let client = &client.client;

        let modelinfo_text = client.post(url).send().await?.text().await?;
        let json_value: serde_json::Value = serde_json::from_str(&modelinfo_text)?;

        let mut name_to_model = HashMap::new();
        let mut id_to_model = HashMap::new();

        for (model_id_obj, model_info_obj) in json_value.as_object().unwrap().iter() {
            let model_id: u32 = model_id_obj.parse()?;

            let config_path = model_info_obj["config_path"].to_string();

            let split_pattern = match config_path.contains("\\\\") {
                true => "\\\\",
                false => "/",
            };

            // モデルのフォルダ名
            let folder_name = config_path
                .split(split_pattern)
                .nth(1)
                .context("None Error")?
                .to_string();

            // HashMap<speaker_name, speaker_id>
            let spk2id: HashMap<_, _> = model_info_obj["spk2id"]
                .as_object()
                .context("None Error")?
                .iter()
                .map(|(speaker_name, speaker_id)| {
                    (
                        speaker_name.to_string(),
                        speaker_id.as_u64().unwrap() as u32,
                    )
                })
                .collect();

            // HashMap<style_name, style_id>
            let style2id: HashMap<_, _> = model_info_obj["style2id"]
                .as_object()
                .context("None Error")?
                .iter()
                .map(|(style_name, style_id)| {
                    (style_name.to_string(), style_id.as_u64().unwrap() as u32)
                })
                .collect();

            // キーと値を反転
            let id2spk: HashMap<_, _> = spk2id.iter().map(|(k, v)| (*v, k.clone())).collect();
            let id2style: HashMap<_, _> = style2id.iter().map(|(k, v)| (*v, k.clone())).collect();

            let model_info = ModelInfo {
                model_id,
                model_name: folder_name.clone(),
                spk2id,
                id2spk,
                style2id,
                id2style,
            };

            name_to_model.insert(folder_name, model_info.clone());
            id_to_model.insert(model_id, model_info);
        }

        let sbv2_modelinfo = Self {
            name_to_model,
            id_to_model,
        };

        {
            let mut lock = SBV2_MODELINFO.write().unwrap();
            *lock = sbv2_modelinfo;
        }

        Ok(())
    }

    /// 引数をもとにSbv2ModelInfoに含まれる有効なモデルを取得する
    pub fn get_valid_model(
        model_name: &str,
        speaker_name: &str,
        style_name: &str,
        default_model: &str,
    ) -> ValidModel {
        let sbv2_modelinfo = SBV2_MODELINFO.read().unwrap();

        // デフォルトモデルが無ければ デフォルトモデル、さらに無ければ id が 0 のものを返す
        let model = match sbv2_modelinfo.name_to_model.get(model_name) {
            Some(model) => model,

            None => match sbv2_modelinfo.name_to_model.get(default_model) {
                Some(model) => model,
                None => sbv2_modelinfo.id_to_model.get(&0).unwrap(),
            },
        };
        let valid_model_name = model.model_name.clone();
        let valid_model_id = model.model_id;

        // 指定した話者が存在しなければ id が 0 の話者を選択する
        let valid_speaker_id = match model.spk2id.get(speaker_name) {
            Some(id) => *id,
            None => 0,
        };
        let valid_speaker_name = model.id2spk.get(&valid_speaker_id).unwrap().clone();

        // 指定したスタイルが存在しなければ id が 0 のスタイルを選択する
        let valid_style_id = match model.style2id.get(style_name) {
            Some(id) => *id,
            None => 0,
        };
        let valid_style_name = model.id2style.get(&valid_style_id).unwrap().clone();

        ValidModel {
            model_name: valid_model_name,
            speaker_name: valid_speaker_name,
            style_name: valid_style_name,
            model_id: valid_model_id,
            speaker_id: valid_speaker_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn update_modelinfo() {
        let client = Sbv2Client::from("127.0.0.1", 5000);

        Sbv2ModelInfo::update_modelinfo(&client).await.unwrap();
        println!("{:#?}", *(SBV2_MODELINFO.read().unwrap()))
    }
}
