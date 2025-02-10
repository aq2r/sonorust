use langrustang::{format_t, lang_t};
use serenity::all::{ComponentInteraction, ComponentInteractionDataKind, Context};
use sonorust_db::UserDataMut;

use crate::{crate_extensions::sonorust_setting::SettingJsonExt, errors::SonorustError, Handler};

pub async fn style(
    handler: &Handler,
    ctx: &Context,
    interaction: &ComponentInteraction,
) -> Result<(), SonorustError> {
    let lang = handler.setting_json.get_bot_lang();

    // 返信する処理
    let send_msg = |content| {
        eq_uilibrium::create_response_msg!(
            interaction,
            &ctx.http,
            content = content,
            ephemeral = true
        )
    };

    // 選択したモデルとスタイルを取得
    let (choice_model, choice_style) = {
        let choice_value = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => &values[0],
            _ => {
                log::error!(lang_t!("log.fail_get_data"));
                send_msg(lang_t!("msg.failed.get", lang)).await?;
                return Ok(());
            }
        };

        let vec: Vec<_> = choice_value.split("||").collect();
        (vec[0], vec[1])
    };

    // ユーザーデータの更新
    let (is_changed_model, before_model) = {
        let mut user_data = UserDataMut::from(interaction.user.id).await?;

        // モデルが変更されたかどうか
        let is_changed_model = &user_data.model_name != choice_model;
        let before_model = user_data.model_name;

        // ユーザーデータを更新
        user_data.model_name = choice_model.to_string();
        user_data.style_name = choice_style.to_string();

        user_data.update().await?;

        (is_changed_model, before_model)
    };

    // 返答するメッセージを作成 モデルが変更されたなら変更したと通知する
    let content = match is_changed_model {
        true => format_t!(
            "style.changed_with_model",
            lang,
            before_model,
            choice_model,
            choice_style
        ),
        false => {
            format_t!("style.changed", lang, choice_style)
        }
    };

    eq_uilibrium::create_response_msg!(interaction, &ctx.http, content = content, ephemeral = true)
        .await?;
    Ok(())
}
