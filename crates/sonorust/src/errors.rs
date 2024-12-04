use std::fmt::Display;

use langrustang::lang_t;
use serenity::all::{ChannelId, Colour, Context, CreateEmbed, GuildId, Interaction};
use setting_inputter::SettingsJson;
use sonorust_db::SonorustDBError;

use crate::crate_extensions::SettingsJsonExtension;

#[derive(Debug)]
pub enum SonorustError {
    SonorustDBError(SonorustDBError),
    SerenityError(serenity::Error),
    UseOnNotGuild,
}

impl SonorustError {
    /// エラーメッセージを discord に送信する
    pub async fn send_err_msg(
        &self,
        ctx: &Context,
        channel_id: ChannelId,
    ) -> Result<(), serenity::Error> {
        let embed = self.create_embed();

        eq_uilibrium::send_msg!(&ctx.http, channel_id, embed = embed).await?;
        Ok(())
    }

    pub async fn send_err_responce(
        &self,
        ctx: &Context,
        interaction: &Interaction,
    ) -> Result<(), serenity::Error> {
        let embed = self.create_embed();

        match interaction {
            Interaction::Command(command_interaction) => {
                eq_uilibrium::create_response_msg!(
                    &ctx.http,
                    command_interaction,
                    embed = embed,
                    ephemeral = true
                )
                .await?;
            }
            Interaction::Component(component_interaction) => {
                eq_uilibrium::create_response_msg!(
                    &ctx.http,
                    component_interaction,
                    embed = embed,
                    ephemeral = true
                )
                .await?;
            }
            Interaction::Modal(modal_interaction) => {
                eq_uilibrium::create_response_msg!(
                    &ctx.http,
                    modal_interaction,
                    embed = embed,
                    ephemeral = true
                )
                .await?;
            }
            Interaction::Ping(_) => {
                log::error!("Can't respond ping interaction: {}", self);
            }
            Interaction::Autocomplete(command_interaction) => {
                eq_uilibrium::create_response_msg!(
                    &ctx.http,
                    command_interaction,
                    embed = embed,
                    ephemeral = true
                )
                .await?;
            }

            _ => log::error!("Can't respond Error message: {}", self),
        }

        Ok(())
    }

    fn create_embed(&self) -> CreateEmbed {
        let lang = SettingsJson::get_bot_lang();

        let (title, description) = match self {
            SonorustError::SerenityError(error) => (lang_t!("msg.error", lang), error.to_string()),
            SonorustError::UseOnNotGuild => (
                lang_t!("msg.only_use_guild_1", lang),
                lang_t!("msg.only_use_guild_2", lang).to_string(),
            ),

            SonorustError::SonorustDBError(error) => (
                lang_t!("msg.error", lang),
                match error {
                    SonorustDBError::InitDatabase(_) => lang_t!("msg.failed.update", lang),
                    SonorustDBError::GetUserData(_) => lang_t!("msg.failed.get", lang),
                    SonorustDBError::UpdateUserData(_) => lang_t!("msg.failed.update", lang),
                    SonorustDBError::GetGuildData(_) => lang_t!("msg.failed.get", lang),
                    SonorustDBError::UpdateGuildData(_) => lang_t!("msg.failed.update", lang),
                    SonorustDBError::Unknown(_) => lang_t!("msg.error", lang),
                }
                .to_string(),
            ),
        };

        CreateEmbed::new()
            .title(title)
            .description(description)
            .colour(Colour::from_rgb(255, 0, 0))
    }
}

impl Display for SonorustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SonorustError::*;

        match self {
            SonorustDBError(sonorust_dberror) => write!(f, "{}", sonorust_dberror),
            SerenityError(error) => write!(f, "{}", error),
            UseOnNotGuild => write!(f, "Guild Id is None"),
        }
    }
}

impl std::error::Error for SonorustError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use SonorustError::*;

        match self {
            SonorustDBError(sonorust_dberror) => Some(sonorust_dberror),
            SerenityError(error) => Some(error),
            UseOnNotGuild => None,
        }
    }
}

// 各エラー型からの変換
impl From<SonorustDBError> for SonorustError {
    fn from(value: SonorustDBError) -> Self {
        SonorustError::SonorustDBError(value)
    }
}

impl From<serenity::Error> for SonorustError {
    fn from(value: serenity::Error) -> Self {
        SonorustError::SerenityError(value)
    }
}

// GuildId が None だった時に?でリターンする用
pub trait NoneToSonorustError<T> {
    fn ok_or_sonorust_err(self) -> Result<T, SonorustError>;
}

impl NoneToSonorustError<GuildId> for Option<GuildId> {
    fn ok_or_sonorust_err(self) -> Result<GuildId, SonorustError> {
        self.ok_or(SonorustError::UseOnNotGuild)
    }
}
