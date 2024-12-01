use std::{path::PathBuf, process, time::Duration};

use anyhow::Context as _;

use crate::model_info::Sbv2ModelInfo;

pub struct Sbv2Client {
    pub host: String,
    pub port: u32,

    pub(crate) client: reqwest::Client,
}

pub struct Sbv2InferParam {
    pub model_id: u32,
    pub speaker_id: u32,
    pub style_name: String,
    pub length: f64,
    pub language: String,
}

impl Sbv2Client {
    pub fn from(host: &str, port: u32) -> Self {
        let client = reqwest::Client::new();

        Self {
            host: host.into(),
            port,
            client,
        }
    }

    pub async fn update_modelinfo(&self) -> anyhow::Result<()> {
        Sbv2ModelInfo::update_modelinfo(self).await?;
        Ok(())
    }

    pub async fn is_api_activation(&self) -> bool {
        let url = {
            let host = self.host.as_str();
            let port = self.port;

            format!("http://{host}:{port}/models/refresh")
        };

        match self.client.get(url).send().await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn infer(&self, text: &str, param: Sbv2InferParam) -> anyhow::Result<Vec<u8>> {
        let url = {
            // パラメーター設定
            let host = self.host.as_str();
            let port = self.port;
            let model_id = param.model_id;
            let speaker_id = param.speaker_id;
            let style_name = param.style_name;
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
                style_weight=5"
            )
        };

        let res = self.client.get(url).send().await?;

        let bytes = res.bytes().await?;

        Ok(bytes.to_vec())
    }

    pub async fn launch_api_win(&self, sbv2_path: &str) -> anyhow::Result<()> {
        let sbv2_path = PathBuf::from(sbv2_path);
        let python_path = sbv2_path.join("venv/Scripts/python.exe");
        let api_py_path = sbv2_path.join("server_fastapi.py");

        let mut child = process::Command::new("cmd")
            .args([
                "/C",
                "start",
                python_path.to_str().context("launch_api Failed")?,
                api_py_path.to_str().context("launch_api Failed")?,
            ])
            .current_dir(sbv2_path)
            .spawn()?;

        child.wait()?;

        loop {
            match self.is_api_activation().await {
                true => break,
                false => tokio::time::sleep(Duration::from_secs(3)).await,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use crate::Sbv2InferParam;

    use super::Sbv2Client;

    static CLIENT: LazyLock<Sbv2Client> = LazyLock::new(|| Sbv2Client::from("127.0.0.1", 5000));

    #[tokio::test]
    async fn test_is_api_activation() {
        dbg!(CLIENT.is_api_activation().await);
    }

    #[ignore]
    #[tokio::test]
    async fn test_infer() {
        let _ = dbg!(
            CLIENT
                .infer(
                    "こんにちは",
                    Sbv2InferParam {
                        model_id: 0,
                        speaker_id: 0,
                        style_name: "Default".into(),
                        length: 1.0,
                        language: "JP".into()
                    }
                )
                .await
        );
    }
}
