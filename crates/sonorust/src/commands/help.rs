use langrustang::lang_t;
use serenity::all::{Context, CreateCommand, CreateEmbed};
use setting_inputter::SettingsJson;

use crate::crate_extensions::SettingsJsonExtension;

pub async fn help(ctx: &Context) -> CreateEmbed {
    let lang = SettingsJson::get_bot_lang();
    let prefix = SettingsJson::get_prefix();

    const IS_INLINE: bool = false;

    // TODO: コマンドヘルプの追加
    let fields = [
        (
            lang_t!("ping.command.name"),
            lang_t!("ping.command.description"),
            IS_INLINE,
        ),
        (
            lang_t!("help.command.name"),
            lang_t!("help.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("now.command.name"),
            lang_t!("now.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("join.command.name"),
            lang_t!("join.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("leave.command.name"),
            lang_t!("leave.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("model.command.name"),
            lang_t!("model.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("speaker.command.name"),
            lang_t!("speaker.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("style.command.name"),
            lang_t!("style.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("server.command.name"),
            lang_t!("server.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("dict.command.name"),
            lang_t!("dict.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("reload.command.name"),
            lang_t!("reload.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("length.command.name"),
            lang_t!("length.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("wav.command.name"),
            lang_t!("wav.command.description", lang),
            IS_INLINE,
        ),
    ];

    let bot_user = ctx.cache.current_user();
    let avatar_url = bot_user
        .avatar_url()
        .unwrap_or_else(|| bot_user.default_avatar_url());

    let fields = fields
        .map(|(name, description, is_inline)| (format!("{prefix}{name}"), description, is_inline));

    CreateEmbed::new()
        .title(lang_t!("help.embed.title", lang))
        .fields(fields)
        .thumbnail(avatar_url)
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("help").description(lang_t!("help.command.description", lang))
}
