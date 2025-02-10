use langrustang::{format_t, lang_t};
use serenity::all::{ComponentInteraction, ComponentInteractionDataKind, Context};
use sonorust_db::UserDataMut;

use crate::{crate_extensions::sonorust_setting::SettingJsonExt, errors::SonorustError, Handler};

pub async fn model(
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

    // 選択したモデル名を取得
    let choice_model = match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => &values[0],
        _ => {
            log::error!(lang_t!("log.fail_get_data"));
            send_msg(lang_t!("msg.failed.get", lang)).await?;
            return Ok(());
        }
    };

    // ユーザーデータを更新
    {
        let mut userdata_mut = UserDataMut::from(interaction.user.id).await?;

        userdata_mut.model_name = choice_model.to_string();
        userdata_mut.speaker_name = String::default();
        userdata_mut.style_name = String::default();

        userdata_mut.update().await?;
    }

    // 返答
    let content = format_t!("model.changed", lang, choice_model);
    send_msg(&content).await?;
    Ok(())
}
