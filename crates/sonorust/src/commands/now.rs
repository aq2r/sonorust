use either::Either;
use langrustang::{format_t, lang_t};
use serenity::all::{CreateCommand, CreateEmbed, User};
use sonorust_db::UserData;

use crate::{
    _langrustang_autogen::Lang, crate_extensions::rwlock::RwLockExt, errors::SonorustError, Handler,
};

struct ModelInfo {
    model_name: String,
    speaker_name: String,
    style_name: String,
    length: f64,
}

pub async fn now(handler: &Handler, user: &User, lang: Lang) -> Result<CreateEmbed, SonorustError> {
    let userdata = UserData::from(user.id).await?;
    let default_model = handler
        .setting_json
        .with_read(|lock| lock.default_model.clone());

    let model_info = {
        let client = handler.infer_client.read().await;

        match client.as_ref() {
            Either::Left(python_client) => {
                let valid_model = python_client
                    .get_valid_model(
                        &userdata.model_name,
                        &userdata.speaker_name,
                        &userdata.style_name,
                        &default_model,
                    )
                    .await;

                ModelInfo {
                    model_name: valid_model.model_name,
                    speaker_name: valid_model.speaker_name,
                    style_name: valid_model.style_name,
                    length: userdata.length,
                }
            }

            Either::Right(rust_client) => {
                let valid_model = rust_client.get_valid_model(&userdata.model_name, &default_model);

                ModelInfo {
                    model_name: valid_model.name.clone(),
                    speaker_name: "default".to_string(),
                    style_name: "default".to_string(),
                    length: userdata.length,
                }
            }
        }
    };

    let fields = [
        (
            lang_t!("now.embed.model", lang),
            model_info.model_name,
            false,
        ),
        (
            lang_t!("now.embed.speaker", lang),
            model_info.speaker_name,
            false,
        ),
        (
            lang_t!("now.embed.style", lang),
            model_info.style_name,
            false,
        ),
        (
            lang_t!("now.embed.speech_rate", lang),
            model_info.length.to_string(),
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

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("now").description(lang_t!("now.command.description", lang))
}
