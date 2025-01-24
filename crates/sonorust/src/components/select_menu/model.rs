use langrustang::{format_t, lang_t};
use serenity::all::{ComponentInteraction, ComponentInteractionDataKind, Context};
use setting_inputter::SettingsJson;
use sonorust_db::UserDataMut;

use crate::{crate_extensions::SettingsJsonExtension, errors::SonorustError};

pub async fn model(ctx: &Context, interaction: &ComponentInteraction) -> Result<(), SonorustError> {
    let lang = SettingsJson::get_bot_lang();

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
        let mut user_data_mut = UserDataMut::from(interaction.user.id).await?;

        user_data_mut.model_name = choice_model.to_string();
        user_data_mut.speaker_name = String::default();
        user_data_mut.style_name = String::default();

        user_data_mut.update().await?;
    }

    // 返答するメッセージを作成
    let content = format_t!("model.changed", lang, choice_model);

    // メッセージを送信
    send_msg(&content).await?;
    Ok(())
}
