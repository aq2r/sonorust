langrustang::i18n!("./lang/bot_lang.yaml");

mod commands;
mod components;
mod crate_extensions;
mod errors;
mod registers;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::RwLock;
use std::{path::PathBuf, sync::Arc, time::Duration};

use crate_extensions::rwlock::RwLockExt;
use crate_extensions::sonorust_setting::SettingJsonExt;
use either::Either;
use engtokana::EngToKana;
use errors::SonorustError;
use infer_api::{Sbv2PythonClient, Sbv2RustClient, Sbv2RustDownloads, Sbv2RustError};
use langrustang::lang_t;
use serenity::all::GatewayError::DisallowedGatewayIntents;
use serenity::all::{
    ChannelId, Colour, Context, CreateEmbed, GuildId, Interaction, Message, Ready, VoiceState,
};
use serenity::{
    all::{EventHandler, GatewayIntents},
    async_trait, Client,
};
use songbird::SerenityInit;
use sonorust_setting::{InferUse, SettingJson};
use tokio::sync::RwLock as TokioRwLock;

type ArcRwLock<T> = Arc<RwLock<T>>;

struct Handler {
    pub setting_json: ArcRwLock<SettingJson>,
    pub infer_client: Arc<TokioRwLock<Either<Sbv2PythonClient, Sbv2RustClient>>>,
    pub read_channels: ArcRwLock<HashMap<GuildId, HashSet<ChannelId>>>,
    pub channel_queues: ArcRwLock<HashMap<GuildId, VecDeque<Vec<u8>>>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        registers::ready(self, &ctx, &ready).await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if let Err(err) = registers::message(self, &ctx, &msg).await {
            match err {
                SonorustError::GuildIdIsNone => {
                    let lang = self.setting_json.get_bot_lang();

                    let embed = CreateEmbed::new()
                        .title(lang_t!("msg.only_use_guild_1", lang))
                        .description(lang_t!("msg.only_use_guild_2", lang))
                        .colour(Colour::from_rgb(255, 0, 0));

                    if let Err(err) =
                        eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, embed = embed).await
                    {
                        log::error!("Cannot send message: {}", err);
                    }
                }
                _ => log::error!("Error on message: {err}"),
            }
        };
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(inter) => {
                if let Err(err) = registers::slash_command(self, &ctx, &inter).await {
                    match err {
                        SonorustError::GuildIdIsNone => {
                            let lang = self.setting_json.get_bot_lang();

                            let embed = CreateEmbed::new()
                                .title(lang_t!("msg.only_use_guild_1", lang))
                                .description(lang_t!("msg.only_use_guild_2", lang))
                                .colour(Colour::from_rgb(255, 0, 0));

                            if let Err(err) =
                                eq_uilibrium::create_response_msg!(inter, &ctx.http, embed = embed)
                                    .await
                            {
                                log::error!("Cannot send message: {}", err);
                            }
                        }
                        _ => log::error!("Error on slash_command: {err}"),
                    }
                };
            }

            Interaction::Component(inter) => {
                if let Err(err) = registers::component(self, &ctx, &inter).await {
                    match err {
                        SonorustError::GuildIdIsNone => {
                            let lang = self.setting_json.get_bot_lang();

                            let embed = CreateEmbed::new()
                                .title(lang_t!("msg.only_use_guild_1", lang))
                                .description(lang_t!("msg.only_use_guild_2", lang))
                                .colour(Colour::from_rgb(255, 0, 0));

                            if let Err(err) =
                                eq_uilibrium::create_response_msg!(inter, &ctx.http, embed = embed)
                                    .await
                            {
                                log::error!("Cannot send message: {}", err);
                            }
                        }
                        _ => log::error!("Error on component: {err}"),
                    }
                }
            }

            _ => (),
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        registers::voice_state_update(self, &ctx, &old, &new).await;
    }
}

#[tokio::main]
async fn main() {
    sonorust_logger::setup_logger();

    // Decoration
    let title = "Sonorust App";
    let top_border = "╭".to_string() + &"─".repeat(38) + "╮";
    let bottom_border = "╰".to_string() + &"─".repeat(38) + "╯";
    let left_gradient = "░▒▓█";
    let right_gradient = "█▓▒░";

    println!("\n\x1b[1;32m{}", top_border);
    println!("│{:^38}│", "");
    println!(
        "│    {:^38}   │",
        format!("\x1b[1;32m{} {} {}", left_gradient, title, right_gradient)
    );
    println!("│{:^38}│", "");
    println!("{}\x1b[0m\n", bottom_border);

    // main
    let downloads_folder = PathBuf::from("appdata/downloads");

    tokio::fs::create_dir_all(&downloads_folder)
        .await
        .expect("Failed create folder");

    let setting_json = SettingJson::init("appdata/setting.json")
        .await
        .expect("Failed init json");

    // カタカナ読み辞書の初期化
    EngToKana::download_and_init_dic("appdata/downloads")
        .await
        .expect("Failed init engtokana dict");

    // データベースの初期化
    sonorust_db::init_database("appdata/database.db")
        .await
        .expect("Failed init database");

    // 推論部分の初期化
    let infer_client: Either<Sbv2PythonClient, Sbv2RustClient> = match setting_json.infer_use {
        InferUse::Python => {
            // windowsの場合のみsbv2の自動起動に対応
            if let Some(path) = &setting_json.sbv2_path {
                if cfg!(target_os = "windows") {
                    Sbv2PythonClient::launch_api_windows(
                        path,
                        &setting_json.host,
                        setting_json.port,
                    )
                    .await
                    .expect("Failed launch sbv2api");
                }
            }

            let python_client =
                match Sbv2PythonClient::connect(&setting_json.host, setting_json.port).await {
                    Ok(client) => client,
                    Err(_) => {
                        log::error!("SBV2 API is not running");
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        panic!("SBV2 API is not running")
                    }
                };

            Either::Left(python_client)
        }

        InferUse::Rust => {
            // 必要なもののダウンロードなど
            let download_client = Sbv2RustDownloads::new();

            log::info!("Preparing for sbv2...");
            let (r1, r2) = tokio::join!(
                download_client.download_debertaonnx(&downloads_folder),
                download_client.download_tokenizer(&downloads_folder),
            );

            match (r1, r2) {
                (Ok(_), Ok(_)) => (),
                _ => {
                    log::warn!("Failed Download sbv2 Model.")
                }
            }

            let result = download_client
                .download_and_set_onnxruntime(
                    &downloads_folder,
                    setting_json.is_gpu_version_runtime,
                )
                .await;

            if let Err(_) = result {
                log::warn!("Automatic download of ONNXRuntime is only available for Windows.");
            }

            // rust client の作成
            let count = setting_json.max_load_model_count.map(|i| i as u64);
            let deberta_path = downloads_folder.join("deberta.onnx");
            let tokenizer_path = downloads_folder.join("tokenizer.json");

            let result = Sbv2RustClient::new_from_model_folder(
                deberta_path.as_path(),
                tokenizer_path.as_path(),
                setting_json.onnx_model_path.as_path(),
                count,
            )
            .await;

            let rust_client = match result {
                Ok(c) => c,
                Err(Sbv2RustError::ModelNotFound) => {
                    log::error!("Model Not Found.");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    panic!("Model Not Found");
                }
                _ => panic!("Failed create Sbv2RustClient"),
            };

            log::info!("Preparing complete.");
            Either::Right(rust_client)
        }
    };

    let setting_json = Arc::new(RwLock::new(setting_json));
    let infer_client = Arc::new(TokioRwLock::new(infer_client));
    let read_channels = Arc::new(RwLock::new(HashMap::new()));
    let channel_queues = Arc::new(RwLock::new(HashMap::new()));

    loop {
        let bot_token = setting_json.with_read(|lock| lock.bot_token.clone());

        let mut client = Client::builder(&bot_token, GatewayIntents::all())
            .event_handler(Handler {
                setting_json: setting_json.clone(),
                infer_client: infer_client.clone(),
                read_channels: read_channels.clone(),
                channel_queues: channel_queues.clone(),
            })
            .register_songbird()
            .await
            .expect("Can't create client");

        let result = client.start().await;

        match result {
            // Intents が足りてなかった場合
            Err(serenity::Error::Gateway(DisallowedGatewayIntents)) => {
                log::error!(
                    "Missing intent, please change the settings in the Discord Developer Portal. (https://discord.com/developers/applications)"
                );

                tokio::time::sleep(Duration::from_secs(10)).await;
                break;
            }

            // ログインできなかった場合
            Err(_) => {
                log::error!("Login failed. Input Discord Bot Token.");
                tokio::time::sleep(Duration::from_secs(1)).await;

                let token = SettingJson::token_reinput()
                    .await
                    .expect("Failed token input");

                {
                    let cloned = {
                        let mut lock = setting_json.write().unwrap();
                        lock.bot_token = token;

                        lock.clone()
                    };

                    SettingJson::write_json("appdata/setting.json", &cloned)
                        .await
                        .expect("Failed write json");
                }

                continue;
            }

            Ok(_) => break,
        }
    }
}
