use std::{
    fmt::Display,
    fs::{self, create_dir_all, File},
    io::Write,
    path::PathBuf,
    sync::{LazyLock, RwLock},
};

use dialoguer::{Confirm, Input, Select};
use serde::{Deserialize, Serialize};

const APPDATA_PATH: &str = "./appdata";
const JSON_PATH: &str = "./appdata/settings.json";
pub static SETTINGS_JSON: LazyLock<RwLock<SettingsJson>> = LazyLock::new(|| {
    RwLock::new(SettingsJson {
        sbv2_path: None,
        read_limit: 50,
        default_model: "".into(),
        prefix: "sn!".into(),
        host: "127.0.0.1".into(),
        port: 5000,
        bot_lang: SettingLang::Ja,
        infer_lang: SettingLang::Ja,
        infer_use: InferUse::Python,
    })
});

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum SettingLang {
    Ja,
    En,
}

impl Display for SettingLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SettingLang::Ja => "Ja",
            SettingLang::En => "En",
        };

        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum InferUse {
    Python,
    Rust,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsJson {
    pub sbv2_path: Option<String>,
    pub read_limit: u32,
    pub default_model: String,
    pub prefix: String,
    pub host: String,
    pub port: u32,
    pub bot_lang: SettingLang,
    pub infer_lang: SettingLang,
    pub infer_use: InferUse,
}

impl SettingsJson {
    pub fn new() -> SettingsJson {
        let text = match fs::read_to_string(JSON_PATH) {
            Ok(text) => text,
            Err(_) => Self::init_json(),
        };

        match serde_json::from_str::<SettingsJson>(&text) {
            Ok(json) => json,
            Err(_) => {
                Self::init_json();
                Self::new()
            }
        }
    }

    /// jsonファイルが存在しなければユーザーに聞いて作成する
    fn init_json() -> String {
        log::info!("Perform initial settings.");

        let is_default = Confirm::new()
            .with_prompt("Do you want to use the default settings?")
            .default(true)
            .interact()
            .unwrap();

        let string = match is_default {
            true => default_process(),
            false => not_default_process(),
        };

        create_dir_all(APPDATA_PATH).expect("Can't open file.");
        let mut file = File::create(JSON_PATH).expect("Can't open file.");
        file.write_all(string.as_bytes()).expect("Can't open file.");

        string
    }

    /// jsonファイルへの書き込みと LazyLock の更新を行う
    pub fn dump_json(settings_json: SettingsJson) {
        let json_str = serde_json::to_string_pretty(&settings_json).unwrap();

        create_dir_all(APPDATA_PATH).expect("Can't open file.");
        let mut file = File::create(JSON_PATH).expect("Can't open file.");
        file.write_all(json_str.as_bytes())
            .expect("Can't open file.");

        {
            let mut lock = SETTINGS_JSON.write().unwrap();
            *lock = settings_json;
        }
    }
}

/// 言語選択
fn select_lang() -> (SettingLang, SettingLang) {
    let choices = ["En (Google or DeepL translate)", "Ja"];
    println!("Select Bot language:");
    let selection_bot = Select::new().items(&choices).interact().unwrap();

    let choices = ["En", "Ja", "Zh"];
    println!("Select SBV2 Infer language:");
    let selection_infer = Select::new().items(&choices).interact().unwrap();

    (
        match selection_bot {
            0_usize => SettingLang::En,
            1_usize => SettingLang::Ja,
            _ => SettingLang::Ja,
        },
        match selection_infer {
            0_usize => SettingLang::En,
            1_usize => SettingLang::Ja,
            _ => SettingLang::Ja,
        },
    )
}

/// 推論にどちらを使うか
fn select_infer_use() -> InferUse {
    let choices = ["litagin02/Style-Bert-VITS2 (Default)", "tuna2134/sbv2-api"];
    println!("Select the library to use for inference:");
    let selection = Select::new().items(&choices).interact().unwrap();

    match selection {
        0_usize => InferUse::Python,
        1_usize => InferUse::Rust,
        _ => InferUse::Python,
    }
}

/// sbv2のパスを入力してもらう
fn input_sbv2_path() -> Option<String> {
    let if_input_path = Confirm::new()
        .with_prompt("Do you want to set the path for SBV2 to start automatically?")
        .default(true)
        .interact()
        .unwrap();

    if !if_input_path {
        return None;
    }

    // sbv2 のパスを入力してもらう
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

/// ユーザーがデフォルト設定を使用するを選択したときの処理
fn default_process() -> String {
    let (bot_lang, infer_lang) = select_lang();
    let infer_use = select_infer_use();
    let sbv2_path = match infer_use {
        InferUse::Python => input_sbv2_path(),
        InferUse::Rust => None,
    };

    let settings_json = SettingsJson {
        sbv2_path,
        read_limit: 50,
        default_model: "".to_string(),
        prefix: "sn!".to_string(),
        host: "127.0.0.1".to_string(),
        port: 5000,
        bot_lang,
        infer_lang,
        infer_use,
    };

    serde_json::to_string_pretty(&settings_json).unwrap()
}

/// ユーザーがデフォルト設定を使用しないを選択したときの処理
fn not_default_process() -> String {
    let read_limit: u32 = Input::new()
        .with_prompt("Input `Maximum number of characters to read`")
        .with_initial_text("50")
        .interact_text()
        .unwrap();

    let default_model: String = Input::new()
        .with_prompt("Input `Default model name`")
        .with_initial_text("None")
        .interact_text()
        .unwrap();

    let prefix: String = Input::new()
        .with_prompt("Input `Prefix`")
        .with_initial_text("sn!")
        .interact_text()
        .unwrap();

    let host: String = Input::new()
        .with_prompt("Input `SBV2 API host`")
        .with_initial_text("127.0.0.1")
        .interact_text()
        .unwrap();

    let port: u32 = Input::new()
        .with_prompt("Input `SBV2 API port`")
        .with_initial_text("5000")
        .interact_text()
        .unwrap();

    let (bot_lang, infer_lang) = select_lang();
    let infer_use = select_infer_use();
    let sbv2_path = match infer_use {
        InferUse::Python => input_sbv2_path(),
        InferUse::Rust => None,
    };

    let settings_json = SettingsJson {
        sbv2_path,
        read_limit,
        default_model,
        prefix,
        host,
        port,
        bot_lang,
        infer_lang,
        infer_use,
    };
    serde_json::to_string_pretty(&settings_json).unwrap()
}
