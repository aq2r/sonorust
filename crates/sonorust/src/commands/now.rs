use langrustang::{format_t, lang_t};
use sbv2_api::Sbv2Client;
use serenity::all::{CreateCommand, CreateEmbed, User};
use setting_inputter::SettingsJson;
use sonorust_db::UserData;

use crate::{
    crate_extensions::{sbv2_api::Sbv2ClientExtension, SettingsJsonExtension},
    errors::SonorustError,
};

pub async fn now(user: &User) -> Result<CreateEmbed, SonorustError> {
    let user_data = UserData::from(user.id).await?;
    let lang = SettingsJson::get_bot_lang();

    let valid_model = Sbv2Client::get_valid_model_from_userdata(&user_data);

    // embed の内容設定
    let model_name = &valid_model.model_name;
    let speaker_name = &valid_model.speaker_name;
    let style_name = &valid_model.style_name;
    let length = user_data.length;

    let fields = [
        (lang_t!("now.embed.model", lang), model_name, false),
        (lang_t!("now.embed.speaker", lang), speaker_name, false),
        (lang_t!("now.embed.style", lang), style_name, false),
        (
            lang_t!("now.embed.speech_rate", lang),
            &length.to_string(),
            false,
        ),
    ];

    // ユーザーの名前を取得
    let username = match &user.global_name {
        Some(name) => name,
        None => &user.name,
    };

    Ok(CreateEmbed::new()
        .title(format_t!("now.embed.title", lang, username))
        .fields(fields))
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("now").description(lang_t!("now.command.description", lang))
}
