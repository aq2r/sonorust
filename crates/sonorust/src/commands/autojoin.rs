use langrustang::lang_t;
use serenity::all::{
    ButtonStyle, ChannelId, Context, CreateActionRow, CreateButton, CreateCommand, CreateEmbed,
    CreateEmbedFooter, GuildId, UserId,
};
use sonorust_db::GuildData;

use crate::{
    _langrustang_autogen::Lang, crate_extensions::serenity::SerenityHttpExt as _,
    errors::SonorustError,
};

pub async fn autojoin(
    ctx: &Context,
    guild_id: Option<GuildId>,
    user_id: UserId,
    lang: Lang,
    voice_ch_id: Option<ChannelId>,
    text_ch_id: Option<ChannelId>,
) -> Result<CreateEmbed, SonorustError> {
    let guild_id = guild_id.ok_or_else(|| SonorustError::GuildIdIsNone)?;
    let member = guild_id.member(&ctx.http, user_id).await?;
    let guilddata = GuildData::from(guild_id).await?;

    let is_bot_owner = {
        let app_owner_id = ctx.http.get_bot_owner_id().await;
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
            .footer(CreateEmbedFooter::new(lang_t!(
                "autojoin.embed.footer",
                lang
            )))
    };

    Ok(embed)
}

pub fn create_command(lang: Lang) -> CreateCommand {
    // TODO: 後でチャンネル選択を追加
    CreateCommand::new("autojoin").description(lang_t!("autojoin.command.description", lang))
}
