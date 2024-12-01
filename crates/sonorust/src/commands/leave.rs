use langrustang::lang_t;
use serenity::all::{Context, CreateCommand, GuildId};
use setting_inputter::SettingsJson;

use crate::crate_extensions::{sbv2_api::READ_CHANNELS, SettingsJsonExtension};

pub async fn leave(ctx: &Context, guild_id: Option<GuildId>) -> &'static str {
    let lang = SettingsJson::get_bot_lang();

    let Some(guild_id) = guild_id else {
        return lang_t!("msg.only_use_guild_2", lang);
    };

    let manager = songbird::get(ctx).await.unwrap();

    // ボットがvcにいるなら切断、いないなら接続していませんと返す
    match manager.get(guild_id) {
        Some(_) => {
            if let Err(err) = manager.remove(guild_id).await {
                log::error!("{}: {err}", lang_t!("log.fail_leave_vc"));
                return lang_t!("leave.cannot_disconnect", lang);
            }
        }
        None => return lang_t!("leave.already", lang),
    }

    // 読み上げる対象から外す
    {
        let mut read_channels = READ_CHANNELS.write().unwrap();
        read_channels.remove(&guild_id);
    }

    lang_t!("leave.disconnected", lang)
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("leave").description(lang_t!("leave.command.description", lang))
}
