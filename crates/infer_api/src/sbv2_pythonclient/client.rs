use std::{collections::HashMap, path::Path, time::Duration};

use tokio::process;

use super::errors::Sbv2PythonError;

#[derive(Debug, Clone)]
pub struct Sbv2PythonModel {
    pub model_id: u64,
    pub model_name: String,
    pub spk2id: HashMap<String, u64>,
    pub id2spk: HashMap<u64, String>,
    pub style2id: HashMap<String, u64>,
    pub id2style: HashMap<u64, String>,
}

#[derive(Debug, Clone)]
pub struct Sbv2PythonModelMap {
    pub name_to_model: HashMap<String, Sbv2PythonModel>,
    pub id_to_model: HashMap<u64, Sbv2PythonModel>,
}

/// Sbv2ModelInfoに含まれるモデル
#[derive(Debug)]
pub struct Sbv2PythonValidModel {
    pub model_name: String,
    pub speaker_name: String,
    pub style_name: String,
    pub model_id: u64,
    pub speaker_id: u64,
}

/// 推論時に指定するパラメーター
#[derive(Debug, Clone)]
pub struct Sbv2PythonInferParam {
    pub model_name: String,
    pub speaker_name: String,
    pub style_name: String,
    pub length: f64,
    pub language: String,
}

#[derive(Debug)]
pub struct Sbv2PythonClient {
    host: String,
    port: u32,

    client: reqwest::Client,
    model_info: Sbv2PythonModelMap,
}

impl Sbv2PythonClient {
    pub async fn connect(host: &str, port: u32) -> Result<Self, Sbv2PythonError> {
        let host: String = host.into();

        let client = reqwest::Client::new();
        let model_info = Self::get_modelinfo(&client, &host, port).await?;

        Ok(Self {
            host,
            port,
            client,
            model_info,
        })
    }

    /// PythonのAPIに推論を送る
    ///
    /// 存在しないモデル名を指定しても存在するモデルに変換してから推論を行う
    pub async fn infer(
        &self,
        text: &str,
        param: Sbv2PythonInferParam,
        default_model: &str,
    ) -> Result<Vec<u8>, Sbv2PythonError> {
        let valid_model = self
            .get_valid_model(
                &param.model_name,
                &param.speaker_name,
                &param.style_name,
                default_model,
            )
            .await;

        let url = {
            // パラメーター設定
            let host = self.host.as_str();
            let port = self.port;
            let model_id = valid_model.model_id;
            let speaker_id = valid_model.speaker_id;
            let style_name = valid_model.style_name;
            let length = param.length;

            let sdp_ratio = 0.2;
            let noise = 0.6;
            let noisew = 0.8;

            let language = match param.language.as_str() {
                "Jp" => "JP",
                "Ja" => "JP",
                "En" => "EN",
                "Zh" => "ZH",

                "JP" => "JP",
                "JA" => "JP",
                "EN" => "EN",
                "ZH" => "ZH",

                "jp" => "JP",
                "ja" => "JP",
                "en" => "EN",
                "zh" => "ZH",

                _ => "JP",
            };

            format!(
                "\
                    http://{host}:{port}/voice?\
                    text={text}&\
                    encoding=utf-8&\
                    model_id={model_id}&\
                    speaker_id={speaker_id}&\
                    sdp_ratio={sdp_ratio}&\
                    noise={noise}&\
                    noisew={noisew}&\
                    length={length}&\
                    language={language}&\
                    auto_split=true&\
                    split_interval=0.5&\
                    assist_text_weight=1&\
                    style={style_name}&\
                    style_weight=5
                "
            )
        };

        let mut res = self.client.get(url).send().await?;

        let mut result = Vec::with_capacity(res.content_length().unwrap_or(0) as usize);
        while let Some(chunk) = res.chunk().await? {
            result.extend_from_slice(&chunk);
        }

        Ok(result)
    }

    pub async fn get_valid_model(
        &self,
        model_name: &str,
        speaker_name: &str,
        style_name: &str,
        default_model: &str,
    ) -> Sbv2PythonValidModel {
        let model_info = &self.model_info;

        // デフォルトモデルが無ければ デフォルトモデル、さらに無ければ id が 0 のものを返す
        let model = match model_info.name_to_model.get(model_name) {
            Some(m) => m,
            None => match model_info.name_to_model.get(default_model) {
                Some(m) => m,
                None => model_info
                    .id_to_model
                    .get(&0)
                    .expect("id:0 Model not found"),
            },
        };

        let valid_model_name = model.model_name.clone();
        let valid_model_id = model.model_id;

        // 指定した話者が存在しなければ id が 0 の話者を選択する
        let valid_speaker_id = match model.spk2id.get(speaker_name) {
            Some(id) => *id,
            None => 0,
        };
        let valid_speaker_name = model
            .id2spk
            .get(&valid_speaker_id)
            .expect(&format!("Speaker: {valid_speaker_id} not found"))
            .clone();

        // 指定したスタイルが存在しなければ id が 0 のスタイルを選択する
        let valid_style_id = match model.style2id.get(style_name) {
            Some(id) => *id,
            None => 0,
        };
        let valid_style_name = model
            .id2style
            .get(&valid_style_id)
            .expect(&format!("Style: {valid_style_id} not found"))
            .clone();

        Sbv2PythonValidModel {
            model_name: valid_model_name,
            speaker_name: valid_speaker_name,
            style_name: valid_style_name,
            model_id: valid_model_id,
            speaker_id: valid_speaker_id,
        }
    }

    pub async fn launch_api_windows<P>(
        sbv2_path: P,
        host: &str,
        port: u32,
    ) -> Result<(), std::io::Error>
    where
        P: AsRef<Path>,
    {
        let sbv2_path = sbv2_path.as_ref();

        // すでに起動しているか確かめる
        let client = reqwest::Client::new();
        let url = format!("http://{host}:{port}/");

        if let Ok(_) = client.get(&url).send().await {
            return Ok(());
        }

        let python_path = sbv2_path.join("venv/Scripts/python.exe");
        let api_py_path = sbv2_path.join("server_fastapi.py");

        log::info!("Starting SBV2 API...");
        let mut child = process::Command::new("cmd")
            .args([
                "/C",
                "start",
                &python_path.to_string_lossy(),
                &api_py_path.to_string_lossy(),
            ])
            .current_dir(sbv2_path)
            .spawn()?;

        child.wait().await?;

        // 接続を試す

        const MAX_RETRIES: u32 = 10;
        for _ in 0..=MAX_RETRIES {
            match client.get(&url).send().await {
                Ok(_) => {
                    log::info!("API is started.");
                    break;
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        Ok(())
    }

    async fn get_modelinfo(
        client: &reqwest::Client,
        host: &str,
        port: u32,
    ) -> Result<Sbv2PythonModelMap, Sbv2PythonError> {
        let url = format!("http://{host}:{port}/models/refresh");

        let modelinfo_text = client.post(url).send().await?.text().await?;

        let json_value: serde_json::Value = serde_json::from_str(&modelinfo_text)?;

        let mut name_to_model = HashMap::new();
        let mut id_to_model = HashMap::new();

        for (model_id_obj, model_info_obj) in json_value.as_object().unwrap().iter() {
            let model_id: u64 = model_id_obj.parse().map_err(|err| {
                Sbv2PythonError::ModelInfoParseError(format!("model_id cannot parsed: {err}"))
            })?;

            let config_path = model_info_obj["config_path"].to_string();

            let split_pattern = match config_path.contains("\\\\") {
                true => "\\\\",
                false => "/",
            };

            // モデルのフォルダ名
            let folder_name = config_path
                .split(split_pattern)
                .nth(1)
                .ok_or_else(|| {
                    Sbv2PythonError::ModelInfoParseError("folder_name is None".to_string())
                })?
                .to_string();

            // HashMap<speaker_name, speaker_id>
            let spk2id = {
                let object = model_info_obj["spk2id"].as_object().ok_or_else(|| {
                    Sbv2PythonError::ModelInfoParseError("spk2id is None".to_string())
                })?;

                let mut map = HashMap::new();
                for (speaker_name, speaker_id) in object {
                    let speaker_name = speaker_name.clone();
                    let speaker_id = speaker_id.as_u64().ok_or_else(|| {
                        Sbv2PythonError::ModelInfoParseError(format!(
                            "Invalid speaker_id: {speaker_name}"
                        ))
                    })?;

                    map.insert(speaker_name, speaker_id);
                }
                map
            };

            // HashMap<style_name, style_id>
            let style2id = {
                let object = model_info_obj["style2id"].as_object().ok_or_else(|| {
                    Sbv2PythonError::ModelInfoParseError("style2id is None".to_string())
                })?;

                let mut map = HashMap::new();
                for (style_name, style_id) in object {
                    let style_name = style_name.clone();
                    let style_id = style_id.as_u64().ok_or_else(|| {
                        Sbv2PythonError::ModelInfoParseError(format!(
                            "Invalid style_id: {style_name}"
                        ))
                    })?;

                    map.insert(style_name, style_id);
                }
                map
            };

            // キーと値を反転
            let id2spk: HashMap<_, _> = spk2id.iter().map(|(k, v)| (*v, k.clone())).collect();
            let id2style: HashMap<_, _> = style2id.iter().map(|(k, v)| (*v, k.clone())).collect();

            let model_info = Sbv2PythonModel {
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

        Ok(Sbv2PythonModelMap {
            name_to_model,
            id_to_model,
        })
    }

    pub async fn update_modelinfo(&mut self) -> Result<(), Sbv2PythonError> {
        let model_info = Self::get_modelinfo(&self.client, &self.host, self.port).await?;
        self.model_info = model_info;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_connect() {
        let _ = dbg!(Sbv2PythonClient::connect("127.0.0.1", 5000).await);
    }

    #[ignore]
    #[tokio::test]
    async fn test_launch_api() {
        let _ = dbg!(Sbv2PythonClient::launch_api_windows("url", "127.0.0.1", 5000).await);
    }

    #[ignore]
    #[tokio::test]
    async fn test_get_valid_model() -> anyhow::Result<()> {
        let client = Sbv2PythonClient::connect("127.0.0.1", 5000).await?;
        dbg!(
            client
                .get_valid_model("model_name", "speaker_name", "style_name", "none")
                .await
        );

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_infer() -> anyhow::Result<()> {
        let client = Sbv2PythonClient::connect("127.0.0.1", 5000).await?;
        client
            .infer(
                "text",
                Sbv2PythonInferParam {
                    model_name: "None".into(),
                    speaker_name: "None".into(),
                    style_name: "None".into(),
                    length: 1.0,
                    language: "Jp".into(),
                },
                "None",
            )
            .await?;

        Ok(())
    }
}
