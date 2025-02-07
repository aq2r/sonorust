use std::time::Duration;

use langrustang::{format_t, lang_t};
use serenity::all::{CreateCommand, CreateEmbed};

use crate::_langrustang_autogen::Lang;

pub fn measuring_embed(lang: Lang) -> CreateEmbed {
    CreateEmbed::new()
        .title(lang_t!("ping.embed.title"))
        .description(lang_t!("ping.embed.measuring", lang))
}

pub fn measured_embed(elapsed: Duration) -> CreateEmbed {
    CreateEmbed::new()
        .title(lang_t!("ping.embed.title"))
        .description(format_t!("ping.embed.measured", elapsed))
}

pub fn create_command() -> CreateCommand {
    CreateCommand::new("ping").description(lang_t!("ping.command.description"))
}
