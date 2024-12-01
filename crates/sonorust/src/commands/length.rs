use langrustang::{format_t, lang_t};
use serenity::all::{CommandOptionType, CreateCommand, CreateCommandOption, UserId};
use setting_inputter::SettingsJson;
use sonorust_db::UserDataMut;

use crate::{crate_extensions::SettingsJsonExtension, errors::SonorustError};

pub async fn length(user_id: UserId, length: f64) -> Result<String, SonorustError> {
    // 0.1 から 5.0 の範囲外ならその範囲に収める
    let length = match length {
        ..=0.1 => 0.1,
        5.0.. => 5.0,
        _ => length,
    };

    // 小数点以下 1 桁までに制限
    let length_rounded = (length * 10.0).round() / 10.0;

    // ユーザーデータを取得して更新
    {
        let mut userdata_mut = UserDataMut::from(user_id).await?;
        userdata_mut.length = length_rounded;

        userdata_mut.update().await?;
    }

    let lang = SettingsJson::get_bot_lang();
    Ok(format_t!("length.changed", lang, length_rounded))
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("length")
        .description(lang_t!("length.command.description", lang))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Number,
                lang_t!("length.option.length"),
                lang_t!("length.option.length.description", lang),
            )
            .min_number_value(0.1)
            .max_number_value(5.0)
            .required(true),
        )
}
