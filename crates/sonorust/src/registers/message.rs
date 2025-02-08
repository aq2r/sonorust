use std::{sync::OnceLock, time::Instant};

use either::Either;
use langrustang::{format_t, lang_t};
use serenity::all::{Context, CreateMessage, EditMessage, Message};

use crate::{
    commands,
    crate_extensions::{
        infer_api::InferApiExt, rwlock::RwLockExt, sonorust_setting::SettingJsonExt,
    },
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

            let result = commands::join(
                handler,
                ctx,
                lang,
                msg.guild_id,
                msg.channel_id,
                msg.author.id,
            )
            .await;

            match result {
                Ok(s) => {
                    let help_embed = commands::help(ctx, lang, prefix).await;
                    eq_uilibrium::send_msg!(
                        msg.channel_id,
                        &ctx.http,
                        content = s,
                        embed = help_embed
                    )
                    .await?;
                }
                Err(s) => {
                    eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, content = s).await?;
                }
            };

            // すでにボイスチャンネルに参加していた場合などは返す
            if let Err(_) = result {
                return Ok(());
            };

            // 音声再生
            handler
                .infer_client
                .play_on_vc(
                    handler,
                    ctx,
                    msg.guild_id,
                    msg.channel_id,
                    msg.author.id,
                    lang_t!("join.connected", lang),
                )
                .await?;
        }
        "leave" => {
            debug_log();

            let text = commands::leave(handler, ctx, lang, msg.guild_id).await;
            msg.channel_id.say(&ctx.http, text).await?;
        }
        "model" => {
            debug_log();

            let (embed, components) = commands::model(handler, lang).await;
            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "speaker" => {
            debug_log();

            let (embed, components) = commands::speaker(handler, msg.author.id, lang).await?;
            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "style" => {
            debug_log();

            let (embed, components) = commands::style(handler, msg.author.id, lang).await?;
            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "length" => {
            debug_log();

            // 数字部分を取得
            let Some(length) = command_rest.get(0).map(|i| *i) else {
                msg.channel_id
                    .say(&ctx.http, format_t!("length.usage", lang, prefix))
                    .await?;
                return Ok(());
            };

            // 数字に変換できなければ返す
            let Ok(length) = length.parse::<f64>() else {
                msg.channel_id
                    .say(&ctx.http, lang_t!("length.not_num", lang))
                    .await?;
                return Ok(());
            };

            // ユーザーデータを変更してメッセージを送信
            let content = commands::length(msg.author.id, length, lang).await?;
            msg.channel_id.say(&ctx.http, content).await?;
        }
        "wav" => {
            debug_log();

            let mut splitn = msg.content.splitn(2, " ");

            // 生成部分を取得
            let Some(content) = splitn.nth(1) else {
                msg.channel_id
                    .say(&ctx.http, format_t!("wav.usage", lang, prefix))
                    .await?;
                return Ok(());
            };

            match commands::wav(handler, msg.author.id, content).await? {
                Either::Left(attachment) => {
                    eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, add_file = attachment)
                        .await?
                }
                Either::Right(content) => {
                    eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, content = content).await?
                }
            };
        }
        "dict" => {
            debug_log();

            let (embed, components) = commands::dict(ctx, msg.guild_id, lang).await?;
            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "now" => {
            debug_log();

            let embed = commands::now(handler, &msg.author, lang).await?;
            eq_uilibrium::send_msg!(msg.channel_id, &ctx.http, embed = embed).await?;
        }
        "reload" => {
            debug_log();

            let content = commands::reload(handler, ctx, msg.author.id, lang).await;
            msg.channel_id.say(&ctx.http, content).await?;
        }
        "server" => {
            debug_log();

            let (embed, components) =
                commands::server(ctx, msg.guild_id, msg.author.id, lang).await?;

            eq_uilibrium::send_msg!(
                msg.channel_id,
                &ctx.http,
                embed = embed,
                components = components
            )
            .await?;
        }
        "autojoin" => {
            debug_log();
        }
        "read_add" => {
            debug_log();
        }
        "read_remove" => {
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
