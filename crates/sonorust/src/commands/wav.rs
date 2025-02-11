use either::Either;
use infer_api::Sbv2PythonInferParam;
use langrustang::lang_t;
use serenity::all::{
    CommandOptionType, CreateAttachment, CreateCommand, CreateCommandOption, UserId,
};
use sonorust_db::UserData;

use crate::{
    crate_extensions::{rwlock::RwLockExt, sonorust_setting::SettingJsonExt},
    errors::SonorustError,
    Handler,
    _langrustang_autogen::Lang,
};

pub async fn wav(
    handler: &Handler,
    user_id: UserId,
    text: &str,
) -> Result<Either<CreateAttachment, &'static str>, SonorustError> {
    let lang = handler.setting_json.get_bot_lang();
    let wav_read_limit = handler.setting_json.with_read(|lock| lock.wav_read_limit);

    let userdata = UserData::from(user_id).await?;
    let (default_model, language) = handler
        .setting_json
        .with_read(|lock| (lock.default_model.clone(), lock.infer_lang.to_string()));

    let limited_text: String = text.chars().take(wav_read_limit as usize).collect();

    let audio_data: Result<Vec<u8>, SonorustError> = {
        let mut lock = handler.infer_client.write().await;

        match lock.as_mut() {
            Either::Left(python_client) => {
                let param = Sbv2PythonInferParam {
                    model_name: userdata.model_name,
                    speaker_name: userdata.speaker_name,
                    style_name: userdata.style_name,
                    length: userdata.length,
                    language: language,
                };
                python_client
                    .infer(&limited_text, param, &default_model)
                    .await
                    .map_err(|err| err.into())
            }

            Either::Right(rust_client) => rust_client
                .infer(
                    &limited_text,
                    &userdata.model_name,
                    userdata.length as f32,
                    &default_model,
                )
                .await
                .map_err(|err| err.into()),
        }
    };

    match audio_data {
        Ok(data) => Ok(Either::Left(CreateAttachment::bytes(data, "audio.mp3"))),
        Err(_) => Ok(Either::Right(lang_t!("wav.fail_infer", lang))),
    }
}

pub fn create_command(lang: Lang) -> CreateCommand {
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
