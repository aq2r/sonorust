use std::collections::HashSet;

use langrustang::lang_t;
use serenity::all::{
    ChannelId, CommandOptionType, Context, CreateCommand, CreateCommandOption, CreateEmbed,
    CreateEmbedFooter, GuildId, UserId,
};
use sonorust_db::{GuildData, GuildDataMut};

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
) -> Result<(Option<CreateEmbed>, Option<&str>), SonorustError> {
    match (voice_ch_id, text_ch_id) {
        (Some(v), Some(t)) => autojoin_setting(ctx, guild_id, user_id, lang, v, t).await,
        _ => Ok((autojoin_embed(ctx, guild_id, lang).await?, None)),
    }
}

// voice_ch_id と text_ch_id が Noneの場合
async fn autojoin_embed(
    ctx: &Context,
    guild_id: Option<GuildId>,
    lang: Lang,
) -> Result<Option<CreateEmbed>, SonorustError> {
    let guild_id = guild_id.ok_or_else(|| SonorustError::GuildIdIsNone)?;

    // 存在しないチャンネルがないかチェックして、ないなら消す処理
    let mut guild_channels = None;

    if let Some(guild) = ctx.cache.guild(guild_id) {
        let channel_set = guild
            .channels
            .iter()
            .map(|(ch, _)| *ch)
            .collect::<HashSet<_>>();

        guild_channels = Some(channel_set);
    }

    if let Some(guild_channels) = guild_channels {
        let guilddata = GuildData::from(guild_id).await?;

        for (voice_ch, text_chs) in &guilddata.autojoin_channels {
            for i in text_chs {
                if guild_channels.get(voice_ch).is_none() || guild_channels.get(i).is_none() {
                    let mut guilddata_mut = GuildDataMut::from(guild_id).await?;

                    let Some(channel_pair) = guilddata_mut.autojoin_channels.get_mut(voice_ch)
                    else {
                        continue;
                    };

                    channel_pair.remove(i);
                    guilddata_mut.update().await?;
                }
            }
        }
    }

    // embed 作成
    let guilddata = GuildData::from(guild_id).await?;
    let embed = {
        let mut description = String::new();

        for (voice_ch, text_chs) in &guilddata.autojoin_channels {
            for i in text_chs {
                description = format!("{description}\n<#{voice_ch}> <- <#{i}>");
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

    Ok(Some(embed))
}

async fn autojoin_setting(
    ctx: &Context,
    option_guild_id: Option<GuildId>,
    user_id: UserId,
    lang: Lang,
    voice_ch_id: ChannelId,
    text_ch_id: ChannelId,
) -> Result<(Option<CreateEmbed>, Option<&str>), SonorustError> {
    let guild_id = option_guild_id.ok_or_else(|| SonorustError::GuildIdIsNone)?;
    let member = guild_id.member(&ctx.http, user_id).await?;

    let is_bot_owner = {
        let app_owner_id = ctx.http.get_bot_owner_id().await;
        app_owner_id == user_id
    };

    let is_admin = {
        #[allow(deprecated)]
        match member.permissions(&ctx.cache) {
            Ok(permissons) => permissons.administrator(),
            Err(_) => false,
        }
    };

    // 管理者でもbotの所有者でもなければ
    if !is_admin && !is_bot_owner {
        return Ok((None, Some(lang_t!("msg.only_admin", lang))));
    }

    let text = {
        let mut guilddata_mut = GuildDataMut::from(guild_id).await?;
        let hashset = guilddata_mut
            .autojoin_channels
            .entry(voice_ch_id)
            .or_insert_with(|| HashSet::new());

        let text = match hashset.contains(&text_ch_id) {
            true => {
                hashset.remove(&text_ch_id);
                lang_t!("autojoin.removed", lang)
            }
            false => {
                hashset.insert(text_ch_id);
                lang_t!("autojoin.inserted", lang)
            }
        };

        guilddata_mut.update().await?;
        text
    };

    let embed = autojoin_embed(ctx, option_guild_id, lang).await?;

    Ok((embed, Some(text)))
}

pub fn create_command(lang: Lang) -> CreateCommand {
    // TODO: 後でチャンネル選択を追加
    CreateCommand::new("autojoin")
        .description(lang_t!("autojoin.command.description", lang))
        .add_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            lang_t!("autojoin.option.voice_ch"),
            lang_t!("autojoin.option.voice_ch.description", lang),
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            lang_t!("autojoin.option.text_ch"),
            lang_t!("autojoin.option.text_ch.description", lang),
        ))
}
