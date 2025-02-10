use langrustang::{format_t, lang_t};
use serenity::all::{Context, CreateCommand, CreateEmbed};

use crate::_langrustang_autogen::Lang;

pub async fn help(ctx: &Context, lang: Lang, prefix: &str) -> CreateEmbed {
    const IS_INLINE: bool = false;

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
            lang_t!("autojoin.command.name"),
            &format_t!("autojoin.command.description", lang, prefix),
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
        (
            lang_t!("read_add.command.name"),
            lang_t!("read_add.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("read_remove.command.name"),
            lang_t!("read_remove.command.description", lang),
            IS_INLINE,
        ),
        (
            lang_t!("clear.command.name"),
            lang_t!("clear.command.description", lang),
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

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("help").description(lang_t!("help.command.description", lang))
}
