use langrustang::{format_t, lang_t};
use serenity::all::{
    ButtonStyle, Context, CreateActionRow, CreateButton, CreateCommand, CreateEmbed, GuildId,
};
use setting_inputter::SettingsJson;
use sonorust_db::GuildData;

use crate::{
    crate_extensions::SettingsJsonExtension,
    errors::{NoneToSonorustError, SonorustError},
};

pub async fn dict(
    ctx: &Context,
    guild_id: Option<GuildId>,
) -> Result<(CreateEmbed, Vec<CreateActionRow>), SonorustError> {
    let guild_id = guild_id.ok_or_sonorust_err()?;
    let guilddata = GuildData::from(guild_id).await?;

    let guild_name = guild_id
        .name(&ctx.cache)
        .unwrap_or_else(|| "Unknown".to_string());

    // コンポーネントと embed の作成
    let component_row0 = create_button_row();
    let embed = create_embed(&guild_name, guilddata);

    Ok((embed, vec![component_row0]))
}

fn create_embed(guild_name: &str, guilddata: GuildData) -> CreateEmbed {
    let lang = SettingsJson::get_bot_lang();

    let server_dict = guilddata.dict;
    let mut description = String::default();

    // embed の内容作成
    for (i, (k, v)) in server_dict.iter().enumerate() {
        match i {
            0 => description += &format!("{k} -> {v}"),
            _ => description += &format!("\n{k} -> {v}"),
        }
    }

    if server_dict.len() == 0 {
        description = lang_t!("dict.unregistered", lang).to_string();
    }

    if description.len() >= 4000 {
        description = lang_t!("dict.too_many", lang).to_string()
    }

    let title = format_t!("dict.embed.title", lang, guild_name);
    CreateEmbed::new().title(title).description(description)
}

fn create_button_row() -> CreateActionRow {
    let button_add = CreateButton::new(lang_t!("customid.dict.add"))
        .label(lang_t!("dict.label.add"))
        .style(ButtonStyle::Primary);
    let button_remove = CreateButton::new(lang_t!("customid.dict.remove"))
        .label(lang_t!("dict.label.remove"))
        .style(ButtonStyle::Secondary);

    CreateActionRow::Buttons(vec![button_add, button_remove])
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("dict").description(lang_t!("dict.command.description", lang))
}
