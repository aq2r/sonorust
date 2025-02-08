use langrustang::{format_t, lang_t};
use serenity::all::{
    ButtonStyle, Context, CreateActionRow, CreateButton, CreateCommand, CreateEmbed, GuildId,
};
use sonorust_db::GuildData;

use crate::{_langrustang_autogen::Lang, errors::SonorustError};

pub async fn dict(
    ctx: &Context,
    guild_id: Option<GuildId>,
    lang: Lang,
) -> Result<(CreateEmbed, Vec<CreateActionRow>), SonorustError> {
    let guild_id = guild_id.ok_or_else(|| SonorustError::GuildIdIsNone)?;

    let guilddata = GuildData::from(guild_id).await?;
    let guild_name = guild_id
        .name(&ctx.cache)
        .unwrap_or_else(|| "Unknown".to_string());

    let embed = {
        let mut description = guilddata
            .dict
            .iter()
            .map(|(k, v)| format!("{k} -> {v}"))
            .collect::<Vec<_>>()
            .join("\n");

        if guilddata.dict.len() == 0 {
            description = lang_t!("dict.unregistered", lang).to_string();
        }

        if description.chars().count() >= 4000 {
            description = lang_t!("dict.too_many", lang).to_string()
        }

        let title = format_t!("dict.embed.title", lang, guild_name);
        CreateEmbed::new().title(title).description(description)
    };

    let component = {
        let button_add = CreateButton::new(lang_t!("customid.dict.add"))
            .label(lang_t!("dict.label.add"))
            .style(ButtonStyle::Primary);
        let button_remove = CreateButton::new(lang_t!("customid.dict.remove"))
            .label(lang_t!("dict.label.remove"))
            .style(ButtonStyle::Secondary);

        let row = CreateActionRow::Buttons(vec![button_add, button_remove]);
        vec![row]
    };

    Ok((embed, component))
}

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("dict").description(lang_t!("dict.command.description", lang))
}
