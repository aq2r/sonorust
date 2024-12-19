use langrustang::{format_t, lang_t};
use serenity::all::{ComponentInteraction, ComponentInteractionDataKind, Context};
use setting_inputter::SettingsJson;
use sonorust_db::UserDataMut;

use crate::{crate_extensions::SettingsJsonExtension, errors::SonorustError};

pub async fn speaker(
    ctx: &Context,
    interaction: &ComponentInteraction,
) -> Result<(), SonorustError> {
    let lang = SettingsJson::get_bot_lang();

    // 選択した値の取得
    let choice_value = match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => &values[0],
        _ => {
            log::error!(lang_t!("log.fail_get_data"));
            eq_uilibrium::create_response_msg!(
                interaction,
                &ctx.http,
                content = lang_t!("msg.failed.get", lang),
                ephemeral = true,
            )
            .await?;

            return Ok(());
        }
    };

    // 選択したモデルと話者を取得
    let (choice_model, choice_speaker) = {
        let vec: Vec<_> = choice_value.split("||").collect();
        (vec[0], vec[1])
    };

    let (is_changed_model, before_model) = {
        let mut user_data = UserDataMut::from(interaction.user.id).await?;

        // モデルが変更されたかどうか
        let is_changed_model = &user_data.model_name != choice_model;
        let before_model = user_data.model_name;

        // ユーザーデータを更新
        user_data.model_name = choice_model.to_string();
        user_data.speaker_name = choice_speaker.to_string();
        user_data.style_name = String::default();

        user_data.update().await?;

        (is_changed_model, before_model)
    };

    // 返答するメッセージを作成 モデルが変更されたなら変更したと通知する
    let content = match is_changed_model {
        true => format_t!(
            "speaker.changed_with_model",
            lang,
            before_model,
            choice_model,
            choice_speaker
        ),
        false => {
            format_t!("speaker.changed", lang, choice_speaker)
        }
    };

    eq_uilibrium::create_response_msg!(interaction, &ctx.http, content = content, ephemeral = true)
        .await?;
    Ok(())
}
