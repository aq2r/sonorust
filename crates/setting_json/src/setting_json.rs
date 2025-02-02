use std::{
    io::{stdin, stdout, Write as _},
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    execute,
    terminal::{Clear, ClearType},
};
use dialoguer::{Input, Select};
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt as _};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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

#[cfg(feature = "infer-python")]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum InferLang {
    Ja,
    En,
    Zh,
}

#[cfg(feature = "infer-python")]
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

static SETTING_JSON: OnceLock<RwLock<SettingJson>> = OnceLock::new();

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingJson {
    bot_token: String,
    read_limit: u32,
    default_model: String,
    prefix: String,
    bot_lang: BotLang,

    #[cfg(feature = "infer-python")]
    sbv2_path: Option<String>,

    #[cfg(feature = "infer-python")]
    host: String,

    #[cfg(feature = "infer-python")]
    port: u32,

    #[cfg(feature = "infer-python")]
    infer_lang: InferLang,

    #[cfg(feature = "infer-rust")]
    model_path: String,
}

impl SettingJson {
    pub fn get_lock<'a>() -> RwLockReadGuard<'a, SettingJson> {
        SETTING_JSON
            .get()
            .expect("Not initialized SettingJson")
            .read()
            .unwrap()
    }

    pub fn get_write_lock<'a>() -> RwLockWriteGuard<'a, SettingJson> {
        SETTING_JSON
            .get()
            .expect("Not initialized SettingJson")
            .write()
            .unwrap()
    }

    pub async fn init<P>(json_path: P) -> anyhow::Result<()>
    where
        P: Into<PathBuf>,
    {
        let json_path: PathBuf = json_path.into();

        let text = match tokio::fs::read_to_string(&json_path).await {
            Ok(text) => text,
            Err(_) => {
                Self::create_json(&json_path)
                    .await
                    .expect("Failed create Json");
                tokio::fs::read_to_string(&json_path)
                    .await
                    .expect("Failed load Json")
            }
        };

        let setting_json = match serde_json::from_str::<SettingJson>(&text) {
            Ok(json) => json,
            Err(_) => {
                Self::create_json(&json_path)
                    .await
                    .expect("Failed create Json");
                let new_text = tokio::fs::read_to_string(&json_path)
                    .await
                    .expect("Failed load Json");

                serde_json::from_str::<SettingJson>(&new_text).expect("Failed load Json")
            }
        };

        SETTING_JSON
            .set(RwLock::new(setting_json))
            .expect("Failed set SETTINGS_JSON");

        Ok(())
    }

    pub async fn dump_json<P>(json_path: P, setting_json: SettingJson)
    where
        P: Into<PathBuf>,
    {
        let json_path: PathBuf = json_path.into();

        let json_str = serde_json::to_string_pretty(&setting_json).unwrap();

        let mut file = File::create(&json_path).await.expect("Can't open file.");
        file.write_all(json_str.as_bytes())
            .await
            .expect("Can't open file.");
        {
            let mut lock = Self::get_write_lock();
            *lock = setting_json;
        }
    }

    async fn create_json(json_path: &Path) -> anyhow::Result<()> {
        // bot_token: String
        log::info!("Input Bot Token");
        print!("Your Bot Token: ");
        stdout().flush()?;

        let mut buffer = String::new();
        stdin().read_line(&mut buffer)?;

        let bot_token = buffer.trim().to_string();

        // 入力内容を書き換え
        execute!(
            std::io::stdout(),
            MoveUp(1),
            Clear(ClearType::FromCursorDown),
            MoveToColumn(0)
        )?;
        println!("Your Bot Token: {}", "*".repeat(bot_token.len()));
        stdout().flush()?;

        // read_limit: u32,
        let read_limit: u32 = Input::new()
            .with_prompt("Input `Maximum number of characters to read`")
            .with_initial_text("50")
            .interact_text()
            .unwrap();

        // default_model: String,
        let default_model: String = Input::new()
            .with_prompt("Input `Default model name`")
            .with_initial_text("None")
            .interact_text()
            .unwrap();

        // prefix: String,
        let prefix: String = Input::new()
            .with_prompt("Input `Prefix`")
            .with_initial_text("sn!")
            .interact_text()
            .unwrap();

        // bot_lang: BotLang,
        let choices = ["En (Google or DeepL translate)", "Ja"];
        println!("Select Bot language:");
        let bot_lang = match Select::new().items(&choices).interact().unwrap() {
            0 => BotLang::En,
            1 => BotLang::Ja,
            _ => BotLang::Ja,
        };

        #[cfg(feature = "infer-python")]
        let (sbv2_path, host, port, infer_lang) = {
            use dialoguer::Confirm;

            let choices = ["En", "Ja", "Zh"];
            println!("Select SBV2 Infer language:");
            let infer_lang = match Select::new().items(&choices).interact().unwrap() {
                0 => InferLang::En,
                1 => InferLang::Ja,
                2 => InferLang::Zh,
                _ => InferLang::Ja,
            };

            // 自動起動はwindowsのみ対応
            // sbv2_path: Option<String>,
            #[cfg(target_os = "windows")]
            let sbv2_path = {
                let if_input_path = Confirm::new()
                    .with_prompt("Do you want to set the path for SBV2 to start automatically?")
                    .default(true)
                    .interact()
                    .unwrap();

                match if_input_path {
                    true => {
                        let input: String = dialoguer::Input::new()
                            .with_prompt("Please enter the SBV2 path")
                            .validate_with(|input: &String| -> Result<(), &str> {
                                let input_path = PathBuf::from(input);
                                let server_fastapi_py = input_path.join("server_fastapi.py");
                                let python_exe = input_path.join("venv/Scripts/python.exe");

                                match (server_fastapi_py.exists(), python_exe.exists()) {
                                    (true, true) => Ok(()),
                                    _ => Err("The path you entered is not an SBV2 path."),
                                }
                            })
                            .interact()
                            .unwrap();

                        Some(input)
                    }
                    false => None,
                }
            };

            #[cfg(not(target_os = "windows"))]
            let sbv2_path: Option<String> = None;

            // host: String,
            let host: String = Input::new()
                .with_prompt("Input `SBV2 API host`")
                .with_initial_text("127.0.0.1")
                .interact_text()
                .unwrap();

            // port: String,
            let port: u32 = Input::new()
                .with_prompt("Input `SBV2 API port`")
                .with_initial_text("5000")
                .interact_text()
                .unwrap();

            (sbv2_path, host, port, infer_lang)
        };

        #[cfg(feature = "infer-rust")]
        let model_path: String = Input::new()
            .with_prompt("Input `sbv2_api model path`")
            .interact_text()
            .unwrap();

        #[cfg(feature = "infer-python")]
        let setting_json = SettingJson {
            bot_token,
            read_limit,
            default_model,
            prefix,
            bot_lang,
            sbv2_path,
            host,
            port,
            infer_lang,
        };

        #[cfg(feature = "infer-rust")]
        let setting_json = SettingJson {
            bot_token,
            read_limit,
            default_model,
            prefix,
            bot_lang,
            model_path,
        };

        let json_string = serde_json::to_string_pretty(&setting_json)?;
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
        sonorust_logger::setup_logger();
        create_dir_all("appdata").await?;
        SettingJson::init("appdata/setting.json").await?;

        let _lock = dbg!(SettingJson::get_lock());

        Ok(())
    }

    #[ignore]
    #[tokio::test]
    async fn test_dump() -> anyhow::Result<()> {
        sonorust_logger::setup_logger();
        create_dir_all("appdata").await?;
        SettingJson::init("appdata/setting.json").await?;

        let mut settings_json = { dbg!(SettingJson::get_lock()).clone() };
        settings_json.bot_token = "test_token_test".to_string();
        SettingJson::dump_json("appdata/setting.json", settings_json).await;

        let _settings_json = dbg!(SettingJson::get_lock());

        Ok(())
    }
}
