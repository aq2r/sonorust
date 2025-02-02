use std::path::PathBuf;

feature_repetition!(
    feature = "infer-python",
    use crate::Sbv2InferParam;,
    use crate::modelinfo::{ModelInfo, Sbv2ModelInfo, SBV2_MODELINFO};,
    use anyhow::Context as _;,
    use std::collections::HashMap;,
    use std::{process::Command, time::Duration};,
);

#[cfg(feature = "infer-python")]
pub struct Sbv2PythonClient {
    pub host: String,
    pub port: u32,

    pub(crate) client: reqwest::Client,
}

#[cfg(feature = "infer-python")]
impl Sbv2PythonClient {
    pub fn new(host: &str, port: u32) -> Self {
        let client = reqwest::Client::new();

        Self {
            host: host.to_string(),
            port,
            client,
        }
    }

    pub async fn update_modelinfo(&self) -> anyhow::Result<()> {
        let url = {
            let host = self.host.as_str();
            let port = self.port;

            format!("http://{host}:{port}/models/refresh")
        };

        let client = &self.client;

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

        let sbv2_modelinfo = Sbv2ModelInfo {
            name_to_model,
            id_to_model,
        };

        {
            let mut lock = SBV2_MODELINFO.write().unwrap();
            *lock = sbv2_modelinfo;
        }

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

    // windowsのみ対応
    pub async fn launch_api_win(&self, sbv2_path: &str) -> anyhow::Result<()> {
        let sbv2_path = PathBuf::from(sbv2_path);
        let python_path = sbv2_path.join("venv/Scripts/python.exe");
        let api_py_path = sbv2_path.join("server_fastapi.py");

        let mut child = Command::new("cmd")
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

feature_repetition!(
    feature = "infer-rust",
    use std::{collections::HashMap, io::Read as _};,
    use tokio::fs::File;,
    use anyhow::{bail};,
    use sbv2_core::TtsModelHolder;,
    use sbv2_core::TtsModelHolderFromPath as _;,
    use crate::modelinfo::{ModelInfo, Sbv2ModelInfo, SBV2_MODELINFO};,
    use tokio::{
        fs::create_dir_all,
        io::{AsyncReadExt as _, AsyncWriteExt as _},
    };,
    use zip::ZipArchive;,
    use flate2::read::MultiGzDecoder;,
    use tar::Archive;,
    use crate::{ValidModel};,
);

#[cfg(feature = "infer-rust")]
pub struct Sbv2RustClient {
    pub(crate) model_holder: TtsModelHolder,
}

#[cfg(feature = "infer-rust")]
impl Sbv2RustClient {
    pub fn new(model_holder: TtsModelHolder) -> Self {
        Self { model_holder }
    }

    pub async fn update_modelinfo<P>(&mut self, model_path: P) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        let model_path: PathBuf = model_path.into();

        // 各モデルのパスを取得
        if !model_path.exists() {
            bail!("model_path is not exists")
        }

        let mut entries = tokio::fs::read_dir(&model_path).await?;

        let mut load_model_paths = vec![];
        while let Ok(Some(e)) = entries.next_entry().await {
            let file_path = e.path();
            let Some(extension) = file_path.extension() else {
                continue;
            };

            if extension == "sbv2" {
                load_model_paths.push(file_path);
            }
        }

        if load_model_paths.len() == 0 {
            bail!("model not found")
        }

        // モデルのアンロード
        let model_holder = &mut self.model_holder;

        let model_idents = model_holder.model_idents();
        for i in model_idents {
            model_holder.unload(&i);
        }

        // モデルの読み込みと static の更新
        let mut name_to_model = HashMap::new();
        let mut id_to_model = HashMap::new();

        for (idx, path) in load_model_paths.iter().enumerate() {
            let model_name = match path.file_stem() {
                Some(stem) => stem.to_string_lossy().to_string(),
                None => "Unknown-Model".to_string(),
            };

            log::info!("Loading Model: {}", model_name);
            model_holder
                .load_from_sbv2file_path(&model_name, path)
                .expect("Failed load Sbv2 File");

            let modelinfo = ModelInfo {
                model_id: idx as u32,
                model_name: model_name.clone(),
                spk2id: HashMap::from([("default".into(), 0)]),
                id2spk: HashMap::from([(0, "default".into())]),
                style2id: HashMap::from([("default".into(), 0)]),
                id2style: HashMap::from([(0, "default".into())]),
            };

            name_to_model.insert(model_name, modelinfo.clone());
            id_to_model.insert(idx as u32, modelinfo);
        }

        {
            let mut lock = SBV2_MODELINFO.write().unwrap();
            *lock = Sbv2ModelInfo {
                name_to_model,
                id_to_model,
            };
        }

        Ok(())
    }

    pub async fn infer(
        &mut self,
        text: &str,
        length: f64,
        validmodel: ValidModel,
    ) -> anyhow::Result<Vec<u8>> {
        let result = self.model_holder.synthesize(
            &validmodel.model_name,
            text,
            0,
            0,
            sbv2_core::SynthesizeOptions {
                sdp_ratio: 0.0,
                length_scale: length as f32,
                style_weight: 1.0,
                split_sentences: true,
            },
        )?;

        Ok(result)
    }

    pub async fn download_tokenizer_and_debert<P>(download_to_path: P) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        let download_to_path: PathBuf = download_to_path.into();

        let devert_path = download_to_path.join("deberta.onnx");
        let tokenizer_path = download_to_path.join("tokenizer.json");

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

    // onnxruntime のダウンロード関数まわり

    async fn download_from_url<P>(download_to_path: P, url: &str) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        let download_to_path: PathBuf = download_to_path.into();

        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        let content = response.bytes().await?;

        let mut file = File::create(download_to_path).await?;
        file.write_all(&content).await?;
        file.flush().await?;

        Ok(())
    }

    async fn unzip<P>(zip_file_path: P) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        let zip_file_path: PathBuf = zip_file_path.into();

        let output_dir = match zip_file_path.parent() {
            Some(path) => path.to_path_buf(),
            None => bail!("zip file path parent is none"),
        };

        let mut file = File::open(&zip_file_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let mut archive = ZipArchive::new(std::io::Cursor::new(buffer))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = output_dir.join(file.mangled_name());

            if file.name().ends_with('/') {
                create_dir_all(&outpath).await?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        create_dir_all(p).await?;
                    }
                }

                let mut outfile = File::create(&outpath).await?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                outfile.write_all(&buffer).await?;
            }
        }

        tokio::fs::remove_file(&zip_file_path).await?;

        Ok(())
    }

    async fn extract_tgz<P>(tgz_file_path: P) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        let tgz_file_path: PathBuf = tgz_file_path.into();

        let output_dir = match tgz_file_path.parent() {
            Some(path) => path.to_path_buf(),
            None => bail!("zip file path parent is none"),
        };

        let mut file = File::open(&tgz_file_path).await?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await?;

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let gz = MultiGzDecoder::new(&contents[..]);
            let mut archive = Archive::new(gz);
            archive.unpack(output_dir)?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    // #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    pub async fn download_and_set_onnxruntime_x86_64_windows<P>(
        download_to_folder: P,
        is_gpu_version: bool,
    ) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        log::info!("Downloading onnxruntime...");

        let download_to_folder: PathBuf = download_to_folder.into();

        let download_to_path = download_to_folder.join("onnxruntime.zip");
        let onnruntime_dll_path = match is_gpu_version {
            true => {
                download_to_folder.join("onnxruntime-win-x64-gpu-1.20.1.zip/lib/onnxruntime.dll")
            }
            false => download_to_folder.join("onnxruntime-win-x64-1.20.1/lib/onnxruntime.dll"),
        };

        let url = match is_gpu_version {
            true => "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-win-x64-gpu-1.20.1.zip",
            false => "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-win-x64-1.20.1.zip",
        };

        if !onnruntime_dll_path.exists() {
            Self::download_from_url(&download_to_path, url).await?;
            Self::unzip(download_to_path).await?;
        }

        let dll_absolute_path = std::env::current_dir()?.join(onnruntime_dll_path);
        std::env::set_var(
            "ORT_DYLIB_PATH",
            dll_absolute_path.to_string_lossy().replace("\\", "/"),
        );

        Ok(())
    }

    pub async fn download_and_set_onnxruntime_x86_64_linux<P>(
        download_to_folder: P,
        is_gpu_version: bool,
    ) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        log::info!("Downloading onnxruntime...");

        let download_to_folder: PathBuf = download_to_folder.into();

        let download_to_path = download_to_folder.join("onnxruntime.zip");
        let onnruntime_dll_path = match is_gpu_version {
            true => download_to_folder
                .join("onnxruntime-linux-x64-gpu-1.20.1/lib/libonnxruntime_providers_shared.so"),
            false => download_to_folder
                .join("onnxruntime-linux-x64-1.20.1/lib/libonnxruntime_providers_shared.so"),
        };

        let url = match is_gpu_version {
            true => "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-linux-x64-gpu-1.20.1.tgz",
            false => "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-linux-x64-1.20.1.tgz",
        };

        if !onnruntime_dll_path.exists() {
            Self::download_from_url(&download_to_path, url).await?;
            Self::extract_tgz(download_to_path).await?;
        }

        let dll_absolute_path = std::env::current_dir()?.join(onnruntime_dll_path);
        std::env::set_var(
            "ORT_DYLIB_PATH",
            dll_absolute_path.to_string_lossy().replace("\\", "/"),
        );

        Ok(())
    }
}

#[cfg(all(test, feature = "infer-rust"))]
mod tests {
    use tokio::fs::create_dir_all;

    use super::Sbv2RustClient;

    #[ignore]
    #[tokio::test]
    async fn test_download_onnxruntime_x86_64_windows() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;

        Sbv2RustClient::download_and_set_onnxruntime_x86_64_windows("appdata", false).await?;
        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_download_and_set_onnxruntime_x86_64_linux() -> anyhow::Result<()> {
        create_dir_all("appdata").await?;

        Sbv2RustClient::download_and_set_onnxruntime_x86_64_linux("appdata", false).await?;
        Ok(())
    }
}
