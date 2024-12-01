use langrustang::{format_t, lang_t};
use serenity::all::{ComponentInteraction, Context, CreateQuickModal, ModalInteraction};
use setting_inputter::SettingsJson;
use sonorust_db::{GuildData, GuildDataMut};

use crate::{
    crate_extensions::SettingsJsonExtension,
    errors::{NoneToSonorustError, SonorustError},
    registers::APP_OWNER_ID,
};

pub async fn dict_add(
    ctx: &Context,
    interaction: &ComponentInteraction,
) -> Result<(), SonorustError> {
    let lang = SettingsJson::get_bot_lang();

    let guild_id = interaction.guild_id.ok_or_sonorust_err()?;
    let guild_data = GuildData::from(guild_id).await?;

    let bot_owner_id = {
        let lock = APP_OWNER_ID.read().unwrap();
        *lock
    };

    let inter_member = guild_id.member(&ctx.http, interaction.user.id).await?;

    let is_admin = inter_member.permissions(&ctx.cache)?.administrator();
    let is_bot_owner = { bot_owner_id == Some(interaction.user.id) };

    // サーバー辞書の編集が管理者に限定されていた場合
    // もしサーバーの管理者でないなら返す (BOT の所有者の場合許可)
    let is_dic_adminonly = guild_data.options.is_dic_onlyadmin;

    if (is_dic_adminonly && !is_admin) && !is_bot_owner {
        eq_uilibrium::create_response_msg!(
            &ctx.http,
            interaction,
            content = lang_t!("msg.only_admin", lang),
            ephemeral = true
        )
        .await?;

        return Ok(());
    }

    // modal の送信と処理
    let modal = create_quickmodal();
    let Ok(Some(response)) = interaction.quick_modal(ctx, modal).await else {
        return Ok(());
    };

    let inputs = response.inputs;
    on_submit(ctx, &response.interaction, inputs).await?;

    Ok(())
}

pub fn create_quickmodal() -> CreateQuickModal {
    let lang = SettingsJson::get_bot_lang();

    CreateQuickModal::new(lang_t!("dict.modal.add.title", lang))
        .timeout(std::time::Duration::from_secs(600))
        .short_field(lang_t!("dict.modal.add.word", lang))
        .short_field(lang_t!("dict.modal.add.readings", lang))
}

async fn on_submit(
    ctx: &Context,
    interaction: &ModalInteraction,
    inputs: Vec<String>,
) -> Result<(), SonorustError> {
    // 入力内容の取得
    let (key, value) = (&inputs[0], &inputs[1]);

    let guild_id = interaction.guild_id.ok_or_sonorust_err()?;

    {
        let mut guild_data_mut = GuildDataMut::from(guild_id).await?;
        guild_data_mut.dict.insert(key.clone(), value.clone());

        guild_data_mut.update().await?;
    }

    let lang = SettingsJson::get_bot_lang();

    // 返答するメッセージを作成
    eq_uilibrium::create_response_msg!(
        &ctx.http,
        interaction,
        content = format_t!("dict.modal.add.set", lang, key, value),
        ephemeral = true
    )
    .await?;

    Ok(())
}
