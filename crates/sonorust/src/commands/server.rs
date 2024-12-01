use langrustang::{format_t, lang_t};
use serenity::all::{
    Context, CreateActionRow, CreateCommand, CreateEmbed, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption, GuildId, UserId,
};
use setting_inputter::SettingsJson;
use sonorust_db::GuildData;

use crate::{
    crate_extensions::SettingsJsonExtension,
    errors::{NoneToSonorustError, SonorustError},
    registers::APP_OWNER_ID,
};

pub async fn server(
    ctx: &Context,
    guild_id: Option<GuildId>,
    user_id: UserId,
) -> Result<(CreateEmbed, Vec<CreateActionRow>), SonorustError> {
    // 必要な情報の取得
    let guild_id = guild_id.ok_or_sonorust_err()?;
    let user = guild_id.member(&ctx.http, user_id).await?;
    let guilddata = GuildData::from(guild_id).await?;

    let bot_owner_id = {
        let app_owner_id = APP_OWNER_ID.read().unwrap();
        *app_owner_id
    };

    let is_bot_owner = { bot_owner_id == Some(user_id) };

    let guild_name = guild_id
        .name(&ctx.cache)
        .unwrap_or_else(|| "Unknown".to_string());

    let embed = create_embed(&guild_name, guilddata);

    // 管理者でない、またはbotの所有者でないならセレクトメニューを追加せずに返す
    match user.permissions(&ctx.cache) {
        Ok(permissons) => {
            if !permissons.administrator() && !is_bot_owner {
                return Ok((embed, vec![]));
            }
        }

        Err(_) => return Ok((embed, vec![])),
    };

    return Ok((embed, vec![create_select_menu()]));
}

fn create_embed(guild_name: &str, guild_data: GuildData) -> CreateEmbed {
    let lang = SettingsJson::get_bot_lang();

    let bool_to_onoff = |bool_: bool| match bool_ {
        true => "ON",
        false => "OFF",
    };

    let title = format_t!("server.embed.title", lang, guild_name);

    let fields = [
        (
            lang_t!("guild.desc.is_auto_join", lang),
            format!("{}", bool_to_onoff(guild_data.options.is_auto_join)),
            false,
        ),
        (
            lang_t!("guild.desc.is_dic_onlyadmin", lang),
            format!("{}", bool_to_onoff(guild_data.options.is_dic_onlyadmin)),
            false,
        ),
        (
            lang_t!("guild.desc.is_entrance_exit_log", lang),
            format!("{}", bool_to_onoff(guild_data.options.is_entrance_exit_log)),
            false,
        ),
        (
            lang_t!("guild.desc.is_entrance_exit_play", lang),
            format!(
                "{}",
                bool_to_onoff(guild_data.options.is_entrance_exit_play)
            ),
            false,
        ),
        (
            lang_t!("guild.desc.is_notice_attachment", lang),
            format!("{}", bool_to_onoff(guild_data.options.is_notice_attachment)),
            false,
        ),
        (
            lang_t!("guild.desc.is_if_long_fastread", lang),
            format!("{}", bool_to_onoff(guild_data.options.is_if_long_fastread)),
            false,
        ),
    ];

    CreateEmbed::new().fields(fields).title(title)
}

fn create_select_menu() -> CreateActionRow {
    let lang = SettingsJson::get_bot_lang();

    let is_auto_join = CreateSelectMenuOption::new(
        lang_t!("guild.desc.is_auto_join", lang),
        lang_t!("guild.is_auto_join"),
    );
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
                is_auto_join,
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

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("server").description(lang_t!("server.command.description", lang))
}
