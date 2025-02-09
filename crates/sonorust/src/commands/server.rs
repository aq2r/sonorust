use langrustang::{format_t, lang_t};
use serenity::all::{
    Context, CreateActionRow, CreateCommand, CreateEmbed, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption, GuildId, UserId,
};
use sonorust_db::GuildData;

use crate::{
    _langrustang_autogen::Lang, crate_extensions::serenity::SerenityHttpExt as _,
    errors::SonorustError,
};

pub async fn server(
    ctx: &Context,
    guild_id: Option<GuildId>,
    user_id: UserId,
    lang: Lang,
) -> Result<(CreateEmbed, Vec<CreateActionRow>), SonorustError> {
    let guild_id = guild_id.ok_or_else(|| SonorustError::GuildIdIsNone)?;
    let member = guild_id.member(&ctx.http, user_id).await?;
    let guilddata = GuildData::from(guild_id).await?;

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

    let guild_name = guild_id
        .name(&ctx.cache)
        .unwrap_or_else(|| "Unknown".to_string());

    let embed = {
        let bool_to_onoff = |bool_: bool| match bool_ {
            true => "ON",
            false => "OFF",
        };

        let title = format_t!("server.embed.title", lang, guild_name);

        let fields = [
            (
                lang_t!("guild.desc.is_dic_onlyadmin", lang),
                format!("{}", bool_to_onoff(guilddata.options.is_dic_onlyadmin)),
                false,
            ),
            (
                lang_t!("guild.desc.is_entrance_exit_log", lang),
                format!("{}", bool_to_onoff(guilddata.options.is_entrance_exit_log)),
                false,
            ),
            (
                lang_t!("guild.desc.is_entrance_exit_play", lang),
                format!("{}", bool_to_onoff(guilddata.options.is_entrance_exit_play)),
                false,
            ),
            (
                lang_t!("guild.desc.is_notice_attachment", lang),
                format!("{}", bool_to_onoff(guilddata.options.is_notice_attachment)),
                false,
            ),
            (
                lang_t!("guild.desc.is_if_long_fastread", lang),
                format!("{}", bool_to_onoff(guilddata.options.is_if_long_fastread)),
                false,
            ),
        ];

        CreateEmbed::new().fields(fields).title(title)
    };

    match is_admin || is_bot_owner {
        true => Ok((embed, vec![create_select_menu(lang)])),
        false => Ok((embed, vec![])),
    }
}

fn create_select_menu(lang: Lang) -> CreateActionRow {
    let dic_only_admin = CreateSelectMenuOption::new(
        lang_t!("guild.desc.is_dic_onlyadmin", lang),
        lang_t!("guild.is_dic_onlyadmin"),
    );
    let is_entrance_exit_log = CreateSelectMenuOption::new(
        lang_t!("guild.desc.is_entrance_exit_log", lang),
        lang_t!("guild.is_entrance_exit_log"),
    );
    let is_entrance_exit_play = CreateSelectMenuOption::new(
        lang_t!("guild.desc.is_entrance_exit_play", lang),
        lang_t!("guild.is_entrance_exit_play"),
    );
    let is_notice_attachment = CreateSelectMenuOption::new(
        lang_t!("guild.desc.is_notice_attachment", lang),
        lang_t!("guild.is_notice_attachment"),
    );
    let is_if_long_fastread = CreateSelectMenuOption::new(
        lang_t!("guild.desc.is_if_long_fastread", lang),
        lang_t!("guild.is_if_long_fastread"),
    );

    let select_menu = CreateSelectMenu::new(
        lang_t!("customid.change_server_settings"),
        CreateSelectMenuKind::String {
            options: vec![
                dic_only_admin,
                is_entrance_exit_log,
                is_entrance_exit_play,
                is_notice_attachment,
                is_if_long_fastread,
            ],
        },
    )
    .placeholder(lang_t!("server.components.placeholder", lang));

    CreateActionRow::SelectMenu(select_menu)
}

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("server").description(lang_t!("server.command.description", lang))
}
