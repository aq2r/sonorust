use std::{path::PathBuf, sync::LazyLock};

use sbv2_api::ValidModel;
use sbv2_core::{SynthesizeOptions, TtsModelHolder};
use setting_inputter::settings_json::SETTINGS_JSON;
use sonorust_db::UserData;
use tokio::{
    fs::{create_dir_all, File},
    sync::Mutex,
};

pub static TTS_MODEL_HOLDER: LazyLock<Mutex<Option<TtsModelHolder>>> =
    LazyLock::new(|| Mutex::new(None));

pub trait TtsModelHolderExtension {
    async fn download_tokenizer_and_debert() -> anyhow::Result<()>;

    async fn load_to_static<P>(
        bert_model: P,
        tokenizer: P,
        max_loaded_models: Option<usize>,
    ) -> anyhow::Result<()>
    where
        P: Into<PathBuf>;

    fn get_valid_model_from_userdata(&self, userdata: &UserData) -> ValidModel;
    async fn infer_from_user(&mut self, text: &str, userdata: &UserData)
        -> anyhow::Result<Vec<u8>>;
}

impl TtsModelHolderExtension for TtsModelHolder {
    async fn download_tokenizer_and_debert() -> anyhow::Result<()> {
        let download_path = "./appdata/downloads";
        create_dir_all(download_path).await?;

        let devert_path = "./appdata/downloads/deberta.onnx";
        let tokenizer_path = "./appdata/downloads/tokenizer.json";

        let devert_path = PathBuf::from(devert_path);
        let tokenizer_path = PathBuf::from(tokenizer_path);

        if !devert_path.exists() {
            log::info!("Downloading 'deverta.onnx'...");

            // download deverta.onnx
            let response = reqwest::get("https://huggingface.co/googlefan/sbv2_onnx_models/resolve/main/deberta.onnx?download=true").await?;
            let bytes = response.bytes().await?;

            let mut file = File::create(devert_path).await?;
            tokio::io::copy(&mut bytes.as_ref(), &mut file).await?;
        }

        if !tokenizer_path.exists() {
            log::info!("Downloading 'tokenizer.json'...");

            let response = reqwest::get("https://huggingface.co/googlefan/sbv2_onnx_models/resolve/main/tokenizer.json?download=true").await?;
            let bytes = response.bytes().await?;

            let mut file = File::create(tokenizer_path).await?;
            tokio::io::copy(&mut bytes.as_ref(), &mut file).await?;
        }

        Ok(())
    }

    async fn load_to_static<P>(
        bert_model: P,
        tokenizer: P,
        max_loaded_models: Option<usize>,
    ) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        let bert_model: PathBuf = bert_model.into();
        let tokenizer: PathBuf = tokenizer.into();

        let result = tokio::task::spawn_blocking(move || -> anyhow::Result<TtsModelHolder> {
            let tts_model_holder =
                TtsModelHolder::new_from_filepath(bert_model, tokenizer, max_loaded_models)?;
            Ok(tts_model_holder)
        })
        .await??;

        let mut lock = TTS_MODEL_HOLDER.lock().await;
        *lock = Some(result);

        Ok(())
    }

    fn get_valid_model_from_userdata(&self, userdata: &UserData) -> ValidModel {
        let default_model = {
            let lock = SETTINGS_JSON.read().unwrap();
            lock.default_model.clone()
        };

        let model_idents = self.model_idents();

        let valid_model_name = match model_idents.iter().find(|i| **i == userdata.model_name) {
            Some(model_name) => model_name.clone(),

            None => match model_idents.iter().find(|i| **i == default_model) {
                Some(model_name) => model_name.clone(),
                None => model_idents.get(0).unwrap().clone(),
            },
        };

        ValidModel {
            model_name: valid_model_name,
            speaker_name: "default".to_string(),
            style_name: "default".to_string(),
            model_id: 0,
            speaker_id: 0,
        }
    }

    async fn infer_from_user(
        &mut self,
        text: &str,
        userdata: &UserData,
    ) -> anyhow::Result<Vec<u8>> {
        let valid_model = self.get_valid_model_from_userdata(userdata);

        let mut param = SynthesizeOptions::default();
        param.length_scale = userdata.length as f32;

        let data = self.synthesize(&valid_model.model_name, text, 0, 0, param)?;

        Ok(data)
    }
}
