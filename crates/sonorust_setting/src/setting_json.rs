use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BotLang {
    Ja,
    En,
}

impl std::fmt::Display for BotLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BotLang::Ja => "Ja",
            BotLang::En => "En",
        };

        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InferLang {
    Ja,
    En,
    Zh,
}

impl std::fmt::Display for InferLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            InferLang::Ja => "Ja",
            InferLang::En => "En",
            InferLang::Zh => "Zh",
        };

        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InferUse {
    Python,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingJson {
    // all
    pub bot_token: String,
    pub read_limit: u32,
    pub default_model: String,
    pub prefix: String,
    pub bot_lang: BotLang,
    pub infer_use: InferUse,

    // python
    pub sbv2_path: Option<PathBuf>,
    pub host: String,
    pub port: u32,
    pub infer_lang: InferLang,

    // rust
    pub onnx_model_path: PathBuf,
    pub max_load_model_count: Option<u32>,
    pub is_gpu_version_runtime: bool,
}

impl SettingJson {
    pub async fn init<P>(json_path: P) -> anyhow::Result<SettingJson>
    where
        P: AsRef<Path>,
    {
        let json_path: PathBuf = json_path.as_ref().to_owned();

        let json_string = tokio::fs::read_to_string(&json_path)
            .await
            .unwrap_or_else(|_| String::new());

        let setting_json = {
            let json_string = json_string.clone();

            let result = tokio::task::spawn_blocking(move || {
                serde_json::from_str::<SettingJson>(&json_string)
            })
            .await?;

            match result {
                Ok(json) => json,

                // 指定されたパスにjsonがなかった場合作成
                Err(_) => {
                    let setting_json = crate::ask::ask_to_create_setting_json()?;

                    let json_string = serde_json::to_string_pretty(&setting_json)?;
                    let mut file = File::create(json_path).await?;
                    file.write_all(json_string.as_bytes()).await?;

                    setting_json
                }
            }
        };

        Ok(setting_json)
    }

    pub async fn write_json<P>(json_path: P, setting_json: SettingJson) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let json_path: &Path = json_path.as_ref();

        let json_string = {
            let setting_json = setting_json.clone();
            tokio::task::spawn_blocking(move || serde_json::to_string_pretty(&setting_json))
                .await??
        };

        let mut file = File::create(json_path).await?;
        file.write_all(json_string.as_bytes()).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::fs::create_dir_all;

    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_init() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;
        dbg!(SettingJson::init("appdata/setting.json").await?);

        Ok(())
    }
}
