use std::{
    io::{stdin, stdout, Write as _},
    path::PathBuf,
};

use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    execute,
    terminal::{Clear, ClearType},
};
use dialoguer::{Confirm, Input, Select};

use crate::setting_json::{BotLang, InferLang, InferUse, SettingJson};

pub fn ask_to_create_setting_json() -> anyhow::Result<SettingJson> {
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

    let read_limit: u32 = Input::new()
        .with_prompt("Enter the maximum number of characters the bot will read")
        .with_initial_text("50")
        .interact_text()
        .unwrap();

    let default_model: String = Input::new()
        .with_prompt("Enter the model name to be used as the default")
        .with_initial_text("None")
        .interact_text()
        .unwrap();

    let prefix: String = Input::new()
        .with_prompt("Enter the bot command prefix")
        .with_initial_text("sn!")
        .interact_text()
        .unwrap();

    let bot_lang = {
        let options = ["Japanese", "English (Used Google or DeepL translate)"];
        let index = Select::new()
            .with_prompt("Select bot language")
            .items(&options)
            .interact()
            .unwrap();

        match index {
            0 => BotLang::Ja,
            1 => BotLang::En,
            _ => unreachable!(),
        }
    };

    let infer_use = {
        let options = ["litagin02/Style-Bert-VITS2", "tuna2134/sbv2-api"];
        let index = Select::new()
            .with_prompt("Select the library to use for inference:")
            .items(&options)
            .interact()
            .unwrap();

        match index {
            0 => InferUse::Python,
            1 => InferUse::Rust,
            _ => unreachable!(),
        }
    };

    let setting_json = match infer_use {
        InferUse::Python => {
            let sbv2_path = input_sbv2_path();

            let host: String = Input::new()
                .with_prompt("Enter SBV2 API host")
                .with_initial_text("127.0.0.1")
                .interact_text()
                .unwrap();

            let port: u32 = Input::new()
                .with_prompt("Enter SBV2 API port")
                .with_initial_text("5000")
                .interact_text()
                .unwrap();

            let infer_lang = {
                let options = ["En", "Ja", "Zh"];
                let index = Select::new()
                    .with_prompt("Select SBV2 Infer language")
                    .items(&options)
                    .interact()
                    .unwrap();

                match index {
                    0 => InferLang::En,
                    1 => InferLang::Ja,
                    2 => InferLang::Zh,
                    _ => unreachable!(),
                }
            };

            SettingJson {
                bot_token,
                read_limit,
                default_model,
                prefix,
                bot_lang,
                infer_use,
                sbv2_path,
                host,
                port,
                infer_lang,
                onnx_model_path: PathBuf::new(),
                max_load_model_count: None,
                is_gpu_version_runtime: false,
            }
        }

        InferUse::Rust => {
            let onnx_model_path = {
                let inputed: String = Input::new()
                    .with_prompt("Enter the folder path where the ***.sbv2 file")
                    .interact_text()
                    .unwrap();

                PathBuf::from(inputed)
            };

            let max_load_model_count: u32 = Input::new()
                .with_prompt("Enter the maximum number of models to load")
                .with_initial_text("5")
                .interact_text()
                .unwrap();

            // onnxruntimeの自動ダウンロードはwindowsのみ対応
            let is_gpu_version_runtime = {
                let is_x86_win = cfg!(target_os = "windows") && cfg!(target_arch = "x86_64");

                match is_x86_win {
                    true => {
                        let options = ["Cpu", "Cuda"];
                        let index = Select::new()
                            .with_prompt("Select Infer Device")
                            .items(&options)
                            .interact()
                            .unwrap();

                        match index {
                            0 => false,
                            1 => true,
                            _ => unreachable!(),
                        }
                    }
                    false => false,
                }
            };

            SettingJson {
                bot_token,
                read_limit,
                default_model,
                prefix,
                bot_lang,
                infer_use,
                sbv2_path: None,
                host: "127.0.0.1".to_string(),
                port: 5000,
                infer_lang: InferLang::Ja,
                onnx_model_path,
                max_load_model_count: Some(max_load_model_count),
                is_gpu_version_runtime,
            }
        }
    };

    Ok(setting_json)
}

fn input_sbv2_path() -> Option<PathBuf> {
    if !cfg!(target_os = "windows") {
        return None;
    }

    let if_input_path = Confirm::new()
        .with_prompt("Do you want to set the path for SBV2 to start automatically?")
        .default(true)
        .interact()
        .unwrap();

    let sbv2_path: Option<String> = match if_input_path {
        true => {
            let inputed = Input::new()
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
                .interact_text()
                .unwrap();

            Some(inputed)
        }

        false => None,
    };

    sbv2_path.map(|i| PathBuf::from(i))
}
