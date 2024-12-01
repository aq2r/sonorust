langrustang::i18n!("./lang/bot_lang.yaml");

mod commands;
mod components;
mod crate_extensions;
mod errors;
mod registers;

use std::time::Duration;

use engtokana::EngToKana;
use sbv2_api::Sbv2Client;
use serenity::all::GatewayError::DisallowedGatewayIntents;
use serenity::{
    all::{Context, EventHandler, GatewayIntents, Interaction, Message, Ready, VoiceState},
    async_trait, Client,
};
use setting_inputter::settings_json::{SettingsJson, SETTINGS_JSON};
use songbird::SerenityInit as _;
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

    // sbv2の起動
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
