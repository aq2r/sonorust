use std::time::Instant;

use either::Either;
use langrustang::lang_t;
use serenity::all::{
    ChannelType, CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, EditMessage,
    ResolvedOption, ResolvedValue,
};

use crate::{
    commands, Handler,
    _langrustang_autogen::Lang,
    crate_extensions::{
        infer_api::InferApiExt, rwlock::RwLockExt, sonorust_setting::SettingJsonExt,
    },
    errors::SonorustError,
};

pub async fn slash_command(
    handler: &Handler,
    ctx: &Context,
    interaction: &CommandInteraction,
) -> Result<(), SonorustError> {
    let lang = handler.setting_json.get_bot_lang();
    let prefix = handler.setting_json.with_read(|lock| lock.prefix.clone());
    let command_name = interaction.data.name.as_str();

    // コマンドが使用されたときのデバッグログ
    let debug_log = || {
        log::debug!(
            "SlashCommand used: /{command_name} (Name: {}, ID: {})",
            interaction.user.name,
            interaction.user.id,
        )
    };

    match command_name {
        "ping" => {
            debug_log();

            // pong を送信
            let embed = commands::ping::measuring_embed(lang);
            let message = CreateInteractionResponseMessage::new().embed(embed);
            let builder = CreateInteractionResponse::Message(message);

            let now = Instant::now();
            interaction.create_response(&ctx.http, builder).await?;
            let elapsed = now.elapsed();

            let mut send_msg = interaction.get_response(&ctx.http).await?;

            // description を計測した時間に書き換え
            let embed = commands::ping::measured_embed(elapsed);
            let edit_msg = EditMessage::new().embed(embed);

            send_msg.edit(&ctx.http, edit_msg).await?;
        }
        "help" => {
            let embed = commands::help(ctx, lang, &prefix).await;
            eq_uilibrium::create_response_msg!(interaction, &ctx.http, embed = embed).await?;
        }
        "join" => {
            // Defer を送信
            let msg = CreateInteractionResponseMessage::new();
            let builder = CreateInteractionResponse::Defer(msg);
            interaction.create_response(&ctx.http, builder).await?;

            let result = commands::join(
                handler,
                ctx,
                lang,
                interaction.guild_id,
                interaction.channel_id,
                interaction.user.id,
            )
            .await;

            match result {
                Ok(s) => {
                    let help_embed = commands::help(ctx, lang, &prefix).await;
                    let builder = CreateInteractionResponseFollowup::new()
                        .content(s)
                        .embed(help_embed);
                    interaction.create_followup(&ctx.http, builder).await?;
                }
                Err(s) => {
                    let builder = CreateInteractionResponseFollowup::new().content(s);
                    interaction.create_followup(&ctx.http, builder).await?;
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
                    interaction.guild_id,
                    interaction.channel_id,
                    interaction.user.id,
                    lang_t!("join.connected", lang),
                )
                .await?;
        }
        "leave" => {
            debug_log();

            let content = commands::leave(handler, ctx, lang, interaction.guild_id).await;
            eq_uilibrium::create_response_msg!(interaction, &ctx.http, content = content).await?;
        }
        "model" => {
            debug_log();

            let (embed, components) = commands::model(handler, lang).await;
            eq_uilibrium::create_response_msg!(
                interaction,
                &ctx.http,
                embed = embed,
                components = components,
            )
            .await?;
        }
        "speaker" => {
            debug_log();

            let (embed, components) = commands::speaker(handler, interaction.user.id, lang).await?;
            eq_uilibrium::create_response_msg!(
                interaction,
                &ctx.http,
                embed = embed,
                components = components,
            )
            .await?;
        }
        "style" => {
            debug_log();

            let (embed, components) = commands::style(handler, interaction.user.id, lang).await?;
            eq_uilibrium::create_response_msg!(
                interaction,
                &ctx.http,
                embed = embed,
                components = components,
            )
            .await?;
        }
        "length" => {
            debug_log();

            // スラッシュコマンドの引数を取得
            let command_args = &interaction.data.options();
            let length: f64 = match command_args.get(0) {
                Some(ResolvedOption {
                    value: ResolvedValue::Number(num),
                    ..
                }) => *num as _,

                _ => 1.0,
            };

            let content = commands::length(interaction.user.id, length, lang).await?;
            eq_uilibrium::create_response_msg!(interaction, &ctx.http, content = content).await?;
        }
        "wav" => {
            // Defer を送信
            let msg = CreateInteractionResponseMessage::new();
            let builder = CreateInteractionResponse::Defer(msg);
            interaction.create_response(&ctx.http, builder).await?;

            // スラッシュコマンドの引数を取得
            let command_args = &interaction.data.options();
            let content = match command_args.get(0) {
                Some(ResolvedOption {
                    value: ResolvedValue::String(content),
                    ..
                }) => content.to_string(),

                _ => String::default(),
            };

            let builder = match commands::wav(handler, interaction.user.id, &content).await? {
                Either::Left(attachment) => {
                    CreateInteractionResponseFollowup::new().add_file(attachment)
                }
                Either::Right(content) => CreateInteractionResponseFollowup::new().content(content),
            };

            interaction.create_followup(&ctx.http, builder).await?;
        }
        "dict" => {
            debug_log();

            let (embed, components) = commands::dict(ctx, interaction.guild_id, lang).await?;
            eq_uilibrium::create_response_msg!(
                interaction,
                &ctx.http,
                embed = embed,
                components = components,
            )
            .await?;
        }
        "now" => {
            debug_log();

            let embed = commands::now(handler, &interaction.user, lang).await?;
            eq_uilibrium::create_response_msg!(interaction, &ctx.http, embed = embed).await?;
        }
        "reload" => {
            debug_log();

            let content = commands::reload(handler, ctx, interaction.user.id, lang).await;
            eq_uilibrium::create_response_msg!(interaction, &ctx.http, content = content).await?;
        }
        "server" => {
            debug_log();

            let (embed, components) =
                commands::server(ctx, interaction.guild_id, interaction.user.id, lang).await?;

            eq_uilibrium::create_response_msg!(
                interaction,
                &ctx.http,
                embed = embed,
                components = components,
            )
            .await?;
        }
        "autojoin" => {
            debug_log();

            let command_args = &interaction.data.options();

            let voice_ch = command_args.get(0).map(|opt| match opt.value {
                ResolvedValue::Channel(partial_channel) => partial_channel,
                _ => unreachable!(),
            });
            let text_ch = command_args.get(1).map(|opt| match opt.value {
                ResolvedValue::Channel(partial_channel) => partial_channel,
                _ => unreachable!(),
            });

            let (embed, text) = match (voice_ch, text_ch) {
                (Some(v), Some(t)) => {
                    if v.kind != ChannelType::Voice {
                        (None, Some(lang_t!("autojoin.not_voicech", lang)))
                    } else if t.kind != ChannelType::Text {
                        (None, Some(lang_t!("autojoin.not_textch", lang)))
                    } else {
                        commands::autojoin(
                            ctx,
                            interaction.guild_id,
                            interaction.user.id,
                            lang,
                            Some(v.id),
                            Some(t.id),
                        )
                        .await?
                    }
                }

                _ => {
                    commands::autojoin(
                        ctx,
                        interaction.guild_id,
                        interaction.user.id,
                        lang,
                        None,
                        None,
                    )
                    .await?
                }
            };

            let mut create_message = CreateInteractionResponseMessage::new();

            if let Some(embed) = embed {
                create_message = create_message.add_embed(embed);
            }

            if let Some(text) = text {
                create_message = create_message.content(text);
            }

            let builder = CreateInteractionResponse::Message(create_message);
            interaction.create_response(&ctx.http, builder).await?;
        }
        "read_add" => {
            debug_log();

            let content = commands::read_add(
                handler,
                ctx,
                interaction.guild_id,
                interaction.channel_id,
                interaction.user.id,
            )?;
            eq_uilibrium::create_response_msg!(interaction, &ctx.http, content = content).await?;
        }
        "read_remove" => {
            debug_log();

            let content = commands::read_remove(
                handler,
                ctx,
                interaction.guild_id,
                interaction.channel_id,
                interaction.user.id,
            )?;
            eq_uilibrium::create_response_msg!(interaction, &ctx.http, content = content).await?;
        }
        "clear" => {
            debug_log();

            let content =
                commands::clear(handler, ctx, interaction.guild_id, interaction.user.id).await?;
            eq_uilibrium::create_response_msg!(interaction, &ctx.http, content = content).await?;
        }

        _ => {
            log::error!(
                "{}: {}",
                lang_t!("log.not_implemented_command"),
                command_name
            );
        }
    }

    Ok(())
}

pub fn registers(lang: Lang) -> Vec<CreateCommand> {
    vec![
        commands::autojoin::create_command(lang),
        commands::clear::create_command(lang),
        commands::dict::create_command(lang),
        commands::help::create_command(lang),
        commands::join::create_command(lang),
        commands::leave::create_command(lang),
        commands::length::create_command(lang),
        commands::model::create_command(lang),
        commands::now::create_command(lang),
        commands::ping::create_command(),
        commands::read_add::create_command(lang),
        commands::read_remove::create_command(lang),
        commands::reload::create_command(lang),
        commands::server::create_command(lang),
        commands::speaker::create_command(lang),
        commands::style::create_command(lang),
        commands::wav::create_command(lang),
    ]
}
