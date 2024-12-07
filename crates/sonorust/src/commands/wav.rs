use langrustang::lang_t;
use sbv2_api::Sbv2Client;
use serenity::all::{
    CommandOptionType, CreateAttachment, CreateCommand, CreateCommandOption, UserId,
};
use setting_inputter::SettingsJson;
use sonorust_db::UserData;

use crate::{
    crate_extensions::{sbv2_api::Sbv2ClientExtension as _, SettingsJsonExtension},
    errors::SonorustError,
};

pub enum AttachmentOrStr {
    Attachment(CreateAttachment),
    Str(&'static str),
}

pub async fn wav(user_id: UserId, text: &str) -> Result<AttachmentOrStr, SonorustError> {
    let lang = SettingsJson::get_bot_lang();
    let userdata = UserData::from(user_id).await?;

    let Ok(voice_data) = Sbv2Client::infer_from_user(text, &userdata).await else {
        log::error!(lang_t!("log.fail_inter_not_launch"));
        return Ok(AttachmentOrStr::Str(lang_t!("wav.fail_infer", lang)));
    };

    Ok(AttachmentOrStr::Attachment(CreateAttachment::bytes(
        voice_data,
        "audio.mp3",
    )))
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("wav")
        .description(lang_t!("wav.command.description", lang))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                lang_t!("wav.option.content"),
                lang_t!("wav.option.content.description", lang),
            )
            .required(true),
        )
}
