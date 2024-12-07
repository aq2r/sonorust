use std::time::Instant;

use langrustang::lang_t;
use sbv2_api::Sbv2Client;
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, EditMessage,
    ResolvedOption, ResolvedValue,
};
use setting_inputter::SettingsJson;

use crate::{
    commands::{self, Either},
    crate_extensions::{sbv2_api::Sbv2ClientExtension as _, SettingsJsonExtension as _},
    errors::SonorustError,
};

pub async fn slash_commands(
    ctx: &Context,
    interaction: &CommandInteraction,
) -> Result<(), SonorustError> {
    let lang = SettingsJson::get_bot_lang();
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
            let embed = commands::ping::measuring_embed();
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
            debug_log();

            let embed = commands::help(ctx).await;
            eq_uilibrium::create_response_msg!(&ctx.http, interaction, embed = embed).await?;
        }

        "now" => {
            debug_log();

            let embed = commands::now(&interaction.user).await?;
            eq_uilibrium::create_response_msg!(&ctx.http, interaction, embed = embed).await?;
        }

        "join" => {
            debug_log();

            // Defer を送信
            let msg = CreateInteractionResponseMessage::new();
            let builder = CreateInteractionResponse::Defer(msg);
            interaction.create_response(&ctx.http, builder).await?;

            let result = commands::join(
                ctx,
                interaction.guild_id,
                interaction.channel_id,
                interaction.user.id,
            )
            .await;

            let text = match result {
                Ok(s) => s,
                Err(s) => s,
            };

            let help_embed = commands::help(ctx).await;
            let builder = CreateInteractionResponseFollowup::new()
                .content(text)
                .embed(help_embed);

            interaction.create_followup(&ctx.http, builder).await?;

            // すでにボイスチャンネルに参加していた場合などは返す
            let Ok(_) = result else {
                return Ok(());
            };

            // 接続音声を再生
            Sbv2Client::play_on_voice_channel(
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

            let content = commands::leave(ctx, interaction.guild_id).await;
            eq_uilibrium::create_response_msg!(&ctx.http, interaction, content = content).await?;
        }

        "model" => {
            debug_log();

            let (embed, components) = commands::model().await;
            eq_uilibrium::create_response_msg!(
                &ctx.http,
                interaction,
                embed = embed,
                components = components,
            )
            .await?;
        }

        "speaker" => {
            debug_log();

            let (embed, components) = commands::speaker(interaction.user.id).await?;
            eq_uilibrium::create_response_msg!(
                &ctx.http,
                interaction,
                embed = embed,
                components = components,
            )
            .await?;
        }

        "style" => {
            debug_log();

            let (embed, components) = commands::style(interaction.user.id).await?;
            eq_uilibrium::create_response_msg!(
                &ctx.http,
                interaction,
                embed = embed,
                components = components,
            )
            .await?;
        }

        "server" => {
            debug_log();

            let (embed, components) =
                commands::server(ctx, interaction.guild_id, interaction.user.id).await?;

            eq_uilibrium::create_response_msg!(
                &ctx.http,
                interaction,
                embed = embed,
                components = components,
            )
            .await?;
        }

        "dict" => {
            debug_log();

            let (embed, components) = commands::dict(ctx, interaction.guild_id).await?;
            eq_uilibrium::create_response_msg!(
                &ctx.http,
                interaction,
                embed = embed,
                components = components,
            )
            .await?;
        }

        "reload" => {
            debug_log();

            let content = match commands::reload(interaction.user.id).await {
                Ok(text) => text,
                Err(_) => {
                    log::error!(lang_t!("log.fail_conn_api"));
                    lang_t!("msg.failed.update", lang)
                }
            };

            eq_uilibrium::create_response_msg!(&ctx.http, interaction, content = content).await?;
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

            let content = commands::length(interaction.user.id, length).await?;
            eq_uilibrium::create_response_msg!(&ctx.http, interaction, content = content).await?;
        }

        "wav" => {
            debug_log();

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

            let builder = match commands::wav(interaction.user.id, &content).await? {
                Either::Left(attachment) => {
                    CreateInteractionResponseFollowup::new().add_file(attachment)
                }
                Either::Right(content) => CreateInteractionResponseFollowup::new().content(content),
            };

            interaction.create_followup(&ctx.http, builder).await?;
        }

        _ => {
            log::error!(
                "{}: {}",
                lang_t!("log.not_implemented_command"),
                command_name
            );
        }
    };

    return Ok(());
}

pub fn registers() -> Vec<CreateCommand> {
    vec![
        commands::dict::create_command(),
        commands::help::create_command(),
        commands::join::create_command(),
        commands::leave::create_command(),
        commands::length::create_command(),
        commands::model::create_command(),
        commands::now::create_command(),
        commands::ping::create_command(),
        commands::reload::create_command(),
        commands::server::create_command(),
        commands::speaker::create_command(),
        commands::style::create_command(),
        commands::wav::create_command(),
    ]
}
