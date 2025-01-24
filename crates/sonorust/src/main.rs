langrustang::i18n!("./lang/bot_lang.yaml");

mod commands;
mod components;
mod crate_extensions;
mod errors;
mod registers;

use std::path::PathBuf;
use std::time::Duration;

use crate_extensions::sbv2_api_rust::{TtsModelHolderExtension, TTS_MODEL_HOLDER};
use crate_extensions::SettingsJsonExtension;
use engtokana::EngToKana;
use sbv2_api::Sbv2Client;
use sbv2_core::{TtsModelHolder, TtsModelHolderFromPath};
use serenity::all::GatewayError::DisallowedGatewayIntents;
use serenity::{
    all::{Context, EventHandler, GatewayIntents, Interaction, Message, Ready, VoiceState},
    async_trait, Client,
};
use setting_inputter::settings_json::{InferUse, SettingsJson, SETTINGS_JSON};
use songbird::SerenityInit as _;
use tokio::fs::create_dir_all;
use tokio::runtime::Runtime;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        registers::ready(&ctx, &ready).await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if let Err(err) = registers::message(&ctx, &msg).await {
            if let Err(err) = err.send_err_msg(&ctx, msg.channel_id).await {
                log::error!("Can't respond message: {}", err);
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(inter) => {
                if let Err(err) = registers::slash_commands(&ctx, inter).await {
                    if let Err(err) = err.send_err_responce(&ctx, &interaction).await {
                        log::error!("Can't respond interaction: {}", err);
                    }
                }
            }

            Interaction::Component(inter) => {
                if let Err(err) = registers::components(&ctx, inter).await {
                    if let Err(err) = err.send_err_responce(&ctx, &interaction).await {
                        log::error!("Can't respond interaction: {}", err);
                    }
                }
            }

            _ => (),
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        if let Err(sonorust_err) = registers::voice_state_update(ctx, old, new).await {
            log::error!("Can't respond voice_state_update: {}", sonorust_err);
        }
    }
}

pub async fn bot_start() {
    // json ファイルの初期設定
    {
        let settings_json = SettingsJson::new();
        let mut lock = SETTINGS_JSON.write().unwrap();
        *lock = settings_json;
    }

    // sbv2 の準備
    let (sbv2_client, sbv2_path) = {
        let lock = SETTINGS_JSON.read().unwrap();
        (
            Sbv2Client::from(&lock.host, lock.port),
            lock.sbv2_path.clone(),
        )
    };

    /// 起動できなかった場合に待機してから終了する処理
    async fn exit_program(text: &str) {
        log::error!("{}", text);
        tokio::time::sleep(Duration::from_secs(30)).await;

        std::process::exit(1);
    }

    let infer_use = SettingsJson::get_sbv2_inferuse();

    // sbv2の起動 (python版の場合)
    if let InferUse::Python = infer_use {
        if !sbv2_client.is_api_activation().await {
            if let Some(path) = sbv2_path {
                if cfg!(target_os = "windows") {
                    log::info!("Waiting for SBV2 API to start...");

                    if let Err(_) = sbv2_client.launch_api_win(&path).await {
                        exit_program("The API could not be started. Exit the program.").await;
                    }

                    log::info!("The API has been started.");
                }
            }
        }

        if let Err(_) = sbv2_client.update_modelinfo().await {
            exit_program("Failed to Connect SBV2 API.").await;
        }
    }

    // 必要なモデルのダウンロードと準備 (Rust版の場合)
    if let InferUse::Rust = infer_use {
        if let Err(_) = TtsModelHolder::download_tokenizer_and_debert().await {
            exit_program("Failed to download Model. Exit the program.").await;
        };

        log::info!("Loading Sbv2 Model...");
        TtsModelHolder::load_to_static(
            "./appdata/downloads/deberta.onnx",
            "./appdata/downloads/tokenizer.json",
            None,
        )
        .await
        .expect("Failed to Init Sbv2");

        // モデルの読み込み
        let models_path = PathBuf::from("./sbv2api_models");
        if !models_path.exists() {
            create_dir_all(&models_path)
                .await
                .expect("Failed create Folder");
        }

        let mut entries = tokio::fs::read_dir(&models_path)
            .await
            .expect("Falied read Folder");

        let mut model_paths = vec![];
        while let Ok(Some(e)) = entries.next_entry().await {
            let file_path = e.path();
            let Some(extension) = file_path.extension() else {
                continue;
            };

            if extension == "sbv2" {
                model_paths.push(file_path);
            }
        }

        if model_paths.len() == 0 {
            exit_program("No model, please put sbv2 file in `sbv2api_models`.").await;
        }

        let mut lock = TTS_MODEL_HOLDER.lock().await;
        let model_holder = lock.as_mut().unwrap();
        for i in &model_paths {
            let model_name = match i.file_stem() {
                Some(stem) => stem.to_string_lossy().to_string(),
                None => "".to_string(),
            };

            log::info!("Loaded Model: {}", model_name);
            model_holder
                .load_from_sbv2file_path(&model_name, i)
                .expect("Failed load Sbv2 File");
        }
    }

    // カタカナ読み辞書の初期化
    if let Err(_) = EngToKana::download_init_dic().await {
        exit_program("Failed to download the kana reading dictionary. Exit the program.").await;
    };

    // BOTのclient作成
    let intents = GatewayIntents::all();

    // ログインできなかった場合トークンをもう一度入力してもらいログインを試す
    loop {
        let token = setting_inputter::get_or_set_token()
            .await
            .expect("Can't open file");

        let mut client = Client::builder(token, intents)
            .event_handler(Handler)
            .register_songbird()
            .await
            .expect("Can't create client");

        // Bot にログイン
        let result = client.start().await;

        match result {
            // Intents が足りてなかった場合
            Err(serenity::Error::Gateway(DisallowedGatewayIntents)) => {
                exit_program(
                "Missing intent, please change the settings in the Discord Developer Portal. (https://discord.com/developers/applications)"
            ).await;
            }

            // ログインできなかった場合
            Err(_) => {
                log::info!("Login failed. Input Discord Bot Token.");
                setting_inputter::input_token()
                    .await
                    .expect("Can't open file");

                continue;
            }

            Ok(_) => break,
        }
    }
}

fn main() {
    sonorust_logger::setup_logger();

    let runtime = Runtime::new().unwrap();
    runtime.block_on(async { bot_start().await });
}
