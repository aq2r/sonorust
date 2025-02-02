use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

pub static SBV2_MODELINFO: LazyLock<RwLock<Sbv2ModelInfo>> = LazyLock::new(|| {
    RwLock::new(Sbv2ModelInfo {
        name_to_model: HashMap::new(),
        id_to_model: HashMap::new(),
    })
});

#[derive(Debug)]
pub struct Sbv2ModelInfo {
    pub name_to_model: HashMap<String, ModelInfo>,
    pub id_to_model: HashMap<u32, ModelInfo>,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub model_id: u32,
    pub model_name: String,
    pub spk2id: HashMap<String, u32>,
    pub id2spk: HashMap<u32, String>,
    pub style2id: HashMap<String, u32>,
    pub id2style: HashMap<u32, String>,
}

// Sbv2ModelInfoに含まれるモデル
#[derive(Debug)]
pub struct ValidModel {
    pub model_name: String,
    pub speaker_name: String,
    pub style_name: String,
    pub model_id: u32,
    pub speaker_id: u32,
}

impl ValidModel {
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

pub struct Sbv2InferParam {
    pub model_id: u32,
    pub speaker_id: u32,
    pub style_name: String,
    pub length: f64,
    pub language: String,
}
