use langrustang::lang_t;
use serenity::all::{
    ButtonStyle, Context, CreateActionRow, CreateButton, CreateCommand, CreateEmbed, GuildId,
    UserId,
};
use sonorust_db::GuildData;

use crate::{_langrustang_autogen::Lang, errors::SonorustError};

pub async fn autojoin(
    ctx: &Context,
    guild_id: Option<GuildId>,
    user_id: UserId,
    lang: Lang,
) -> Result<(CreateEmbed, Vec<CreateActionRow>), SonorustError> {
    let guild_id = guild_id.ok_or_else(|| SonorustError::GuildIdIsNone)?;
    let member = guild_id.member(&ctx.http, user_id).await?;
    let guilddata = GuildData::from(guild_id).await?;

    let is_bot_owner = {
        let app_owner_id = {
            match ctx.http.get_current_application_info().await {
                Ok(info) => match info.owner {
                    Some(owner) => owner.id,
                    None => UserId::new(1),
                },
                Err(_) => UserId::new(1),
            }
        };
        app_owner_id == user_id
    };

    let is_admin = {
        match member.permissions(&ctx.cache) {
            Ok(permissons) => permissons.administrator(),
            Err(_) => false,
        }
    };

    let embed = {
        let mut description = String::new();

        for (voice_ch, text_chs) in &guilddata.autojoin_channels {
            for i in text_chs {
                description = format!("{description}\n#<{voice_ch}> <- #<{i}>");
            }
        }

        if description.len() == 0 {
            description = lang_t!("autojoin.unregistered", lang).to_string();
        }

        if description.chars().count() >= 4000 {
            description = lang_t!("autojoin.too_many", lang).to_string()
        }

        CreateEmbed::new()
            .title(lang_t!("autojoin.embed.title", lang))
            .description(description)
    };

    let create_component = || {
        let button_register = CreateButton::new(lang_t!("customid.autojoin.register"))
            .label(lang_t!("autojoin.label.register"))
            .style(ButtonStyle::Primary);

        let row = CreateActionRow::Buttons(vec![button_register]);
        vec![row]
    };

    let component = match is_bot_owner || is_admin {
        true => create_component(),
        false => vec![],
    };

    Ok((embed, component))
}

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("autojoin").description(lang_t!("autojoin.command.description", lang))
}
