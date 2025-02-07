use std::{sync::OnceLock, time::Instant};

use serenity::all::{Context, CreateMessage, EditMessage, Message};

use crate::{
    commands,
    crate_extensions::{rwlock::RwLockExt, sonorust_setting::SettingJsonExt},
    errors::SonorustError,
    Handler,
};

pub async fn message(handler: &Handler, ctx: &Context, msg: &Message) -> Result<(), SonorustError> {
    if msg.author.bot {
        return Ok(());
    }

    // prefix をキャッシュしておく
    static PREFIX: OnceLock<String> = OnceLock::new();

    let prefix = PREFIX
        .get_or_init(|| handler.setting_json.with_read(|lock| lock.prefix.clone()))
        .as_str();

    // コマンドかそうじゃないかで分岐
    match msg.content.starts_with(prefix) {
        true => command_processing(handler, ctx, msg, prefix).await?,
        false => other_processing(handler, ctx, msg).await?,
    }

    Ok(())
}

/// メッセージの内容がコマンドだった場合の処理
async fn command_processing(
    handler: &Handler,
    ctx: &Context,
    msg: &Message,
    prefix: &str,
) -> Result<(), SonorustError> {
    let lang = handler.setting_json.get_bot_lang();

    // メッセージからプレフィックスを除いたものを取得
    let command_content = &msg.content[prefix.len()..];

    let command_args: Vec<&str> = command_content.split_whitespace().collect();

    let Some(command_name) = command_args.get(0) else {
        return Ok(());
    };
    let command_rest = command_args.get(1..).unwrap_or_else(|| &[]);

    let debug_log = || {
        log::debug!(
            "MessageCommand used: /{command_content} {{ Name: {}, ID: {} }}",
            msg.author.name,
            msg.author.id,
        )
    };

    match *command_name {
        "ping" => {
            debug_log();

            let embed = commands::ping::measuring_embed(lang);
            let message = CreateMessage::new().embed(embed);

            let now = Instant::now();
            let mut send_msg = msg.channel_id.send_message(&ctx.http, message).await?;
            let elapsed = now.elapsed();

            // description を計測した時間に書き換え
            let embed = commands::ping::measured_embed(elapsed);
            let edit_msg = EditMessage::new().embed(embed);
            send_msg.edit(&ctx.http, edit_msg).await?;
        }

        "help" => {
            debug_log();

            let embed = commands::help(ctx, lang, prefix).await;
            eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, embed = embed).await?;
        }
        "join" => {
            debug_log();
        }
        "leave" => {
            debug_log();
        }
        "model" => {
            debug_log();
        }
        "speaker" => {
            debug_log();
        }
        "style" => {
            debug_log();
        }
        "length" => {
            debug_log();
        }
        "wav" => {
            debug_log();
        }
        "dict" => {
            debug_log();
        }
        "now" => {
            debug_log();
        }
        "reload" => {
            debug_log();
        }
        "server" => {
            debug_log();
        }
        "autojoin" => {
            debug_log();
        }

        _ => (),
    }

    Ok(())
}

// コマンド以外だった時の処理 (主にvcで音声再生)
async fn other_processing(
    handler: &Handler,
    ctx: &Context,
    msg: &Message,
) -> Result<(), SonorustError> {
    todo!()
}
