use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use sbv2_core::{SynthesizeOptions, TtsModelHolder, TtsModelHolderFromPath};

use super::errors::Sbv2RustError;

#[derive(Debug, Clone, PartialEq)]
pub struct Sbv2RustModel {
    pub name: String,
    path: PathBuf,
}

pub struct Sbv2RustClient {
    model_holder: Arc<Mutex<TtsModelHolder>>,
    models: Vec<Sbv2RustModel>,
    loaded_models: Vec<Sbv2RustModel>,
    max_model_load_count: u64,
}

impl Sbv2RustClient {
    pub async fn new_from_model_folder<P>(
        bert_model_path: P,
        tokenizer_path: P,
        modelfolder_path: P,
        max_model_load_count: Option<u64>,
    ) -> Result<Sbv2RustClient, Sbv2RustError>
    where
        P: AsRef<Path>,
    {
        let modelfolder_path = modelfolder_path.as_ref();
        let max_model_load_count = max_model_load_count.unwrap_or(u64::MAX);

        let model_paths = Self::get_model_paths_from_folder(modelfolder_path).await?;

        if model_paths.len() == 0 {
            log::debug!("Model Not Found");
            return Err(Sbv2RustError::ModelNotFound);
        }

        let bert_model_path = bert_model_path.as_ref().to_owned();
        let tokenizer_path = tokenizer_path.as_ref().to_owned();

        log::debug!("Loading bert_model, tokenizer");
        let model_holder = tokio::task::spawn_blocking(move || {
            TtsModelHolder::new_from_filepath(bert_model_path, tokenizer_path, None)
                .map_err(|err| Sbv2RustError::Sbv2CoreError(err.to_string()))
        })
        .await??;

        Ok(Self {
            model_holder: Arc::new(Mutex::new(model_holder)),
            models: model_paths,
            loaded_models: vec![],
            max_model_load_count,
        })
    }

    async fn get_model_paths_from_folder<P>(
        modelfolder_path: P,
    ) -> Result<Vec<Sbv2RustModel>, Sbv2RustError>
    where
        P: AsRef<Path>,
    {
        log::debug!("Model find from: {:?}", modelfolder_path.as_ref());
        let mut model_paths = vec![];

        let mut entries = tokio::fs::read_dir(modelfolder_path).await?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();

            let Some(extention) = path.extension() else {
                continue;
            };

            if extention != "sbv2" {
                continue;
            }

            if let Some(file_name) = path.file_stem() {
                log::debug!("Find model: {file_name:?}");
                model_paths.push(Sbv2RustModel {
                    name: file_name.to_string_lossy().to_string(),
                    path,
                });
            }
        }

        Ok(model_paths)
    }

    pub fn get_modelinfo(&self) -> &Vec<Sbv2RustModel> {
        &self.models
    }

    pub fn get_valid_model(&self, model_name: &str, default_model: &str) -> &Sbv2RustModel {
        match self.models.iter().find(|model| model.name == model_name) {
            Some(model) => model,
            None => match self.models.iter().find(|model| model.name == default_model) {
                Some(model) => model,
                None => self.models.get(0).expect("Model Not Found"),
            },
        }
    }

    pub async fn update_model<P>(&mut self, modelfolder_path: P) -> Result<(), Sbv2RustError>
    where
        P: AsRef<Path>,
    {
        let models = Self::get_model_paths_from_folder(modelfolder_path).await?;

        if models.len() == 0 {
            return Err(Sbv2RustError::ModelNotFound);
        }

        // すでに読み込まれているモデルをアンロード
        let arc = self.model_holder.clone();
        let send_loaded_models = std::mem::replace(&mut self.loaded_models, vec![]);

        tokio::task::spawn_blocking(move || {
            let mut model_holder = arc.lock().unwrap();

            for i in send_loaded_models {
                log::debug!("Model unload: {}", i.name);
                model_holder.unload(&i.name);
            }
        });

        self.models = models;
        Ok(())
    }

    /// 存在しないモデル名を指定しても存在するモデルに変換してから推論を行う
    pub async fn infer(
        &mut self,
        text: &str,
        model_name: &str,
        length: f32,
        default_model: &str,
    ) -> Result<Vec<u8>, Sbv2RustError> {
        let model = self.get_valid_model(model_name, default_model).clone();

        if let None = self.loaded_models.iter().find(|m| **m == model) {
            if self.loaded_models.len() >= self.max_model_load_count as usize {
                let unload_model = self.loaded_models.remove(0);

                log::debug!("Model unload: {}", unload_model.name);

                let arc = self.model_holder.clone();
                tokio::task::spawn_blocking(move || {
                    let mut lock = arc.lock().unwrap();
                    debug_assert!(lock.unload(&unload_model.name));
                })
                .await?;
            }

            log::debug!("Model load: {}", model.name);
            let arc = self.model_holder.clone();
            let model_to_thread = model.clone();

            tokio::task::spawn_blocking(move || {
                let mut lock = arc.lock().unwrap();
                lock.load_from_sbv2file_path(&model_to_thread.name, &model_to_thread.path)
                    .map_err(|err| Sbv2RustError::Sbv2CoreError(err.to_string()))
            })
            .await??;

            self.loaded_models.push(model.clone());
        }

        // 推論は他スレッドに逃がして行う
        let arc = self.model_holder.clone();
        let text = text.to_owned();

        let result = tokio::task::spawn_blocking(move || {
            let mut lock = arc.lock().unwrap();

            let mut option = SynthesizeOptions::default();
            option.length_scale = length;

            log::debug!("synthesize - Model: {} - Content: {text}", model.name);
            lock.synthesize(&model.name, &text, 0, 0, option)
                .map_err(|err| Sbv2RustError::Sbv2CoreError(err.to_string()))
        })
        .await??;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{
        fs::{create_dir_all, File},
        io::AsyncWriteExt,
    };

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_new_from_model_folder() -> anyhow::Result<()> {
        env_logger::init();

        let client = Sbv2RustClient::new_from_model_folder(
            "appdata/downloads/deberta.onnx",
            "appdata/downloads/tokenizer.json",
            "sbv2api_models",
            Some(5),
        )
        .await?;

        dbg!(&client.models);

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_infer() -> anyhow::Result<()> {
        env_logger::init();

        let mut client = Sbv2RustClient::new_from_model_folder(
            "appdata/downloads/deberta.onnx",
            "appdata/downloads/tokenizer.json",
            "sbv2api_models",
            Some(1),
        )
        .await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        let _ = client.infer("text", "model1", 1.0, "model1").await?;
        create_dir_all("appdata").await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        let _ = client.infer("text", "model2", 1.0, "model1").await?;
        create_dir_all("appdata").await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        let result = client.infer("text", "Unknown", 1.0, "model1").await?;
        create_dir_all("appdata").await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        let mut file = File::create("appdata/result.wav").await?;
        file.write_all(&result).await?;

        // モデル読み込みチェック
        println!("model reload");
        client.update_model("sbv2api_models").await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        let _ = client.infer("text", "model1", 1.0, "model1").await?;
        create_dir_all("appdata").await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        let _ = client.infer("text", "model2", 1.0, "model1").await?;
        create_dir_all("appdata").await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        let result = client.infer("text", "Unknown", 1.0, "model1").await?;
        create_dir_all("appdata").await?;

        tokio::time::sleep(Duration::from_secs(5)).await;

        let mut file = File::create("appdata/result.wav").await?;
        file.write_all(&result).await?;

        Ok(())
    }
}
