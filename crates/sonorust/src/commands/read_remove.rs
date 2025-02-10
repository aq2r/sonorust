use langrustang::lang_t;
use serenity::all::{ChannelId, Context, CreateCommand, GuildId, UserId};

use crate::{
    crate_extensions::sonorust_setting::SettingJsonExt, errors::SonorustError, Handler,
    _langrustang_autogen::Lang,
};

pub fn read_remove(
    handler: &Handler,
    ctx: &Context,
    guild_id: Option<GuildId>,
    channel_id: ChannelId,
    user_id: UserId,
) -> Result<&'static str, SonorustError> {
    let guild_id = guild_id.ok_or_else(|| SonorustError::GuildIdIsNone)?;
    let lang = handler.setting_json.get_bot_lang();

    // ユーザーがボイスチャンネルに接続しているか確認
    let is_user_in_vc = {
        let Some(guild) = guild_id.to_guild_cached(&ctx.cache) else {
            return Ok(lang_t!("read_remove.failed", lang));
        };

        guild.voice_states.contains_key(&user_id)
    };

    // ユーザーが接続していない場合
    if !is_user_in_vc {
        return Ok(lang_t!("read_add_remove.user_notconnect", lang));
    }

    // サーバーIDと読み上げるチャンネルIDのペアを登録
    {
        let mut read_channels = handler.read_channels.write().unwrap();
        match read_channels.get_mut(&guild_id) {
            Some(guild_read_ch) => {
                let result = guild_read_ch.remove(&channel_id);
                match result {
                    true => Ok(lang_t!("read_remove.removed", lang)),
                    false => Ok(lang_t!("read_remove.already", lang)),
                }
            }

            None => return Ok(lang_t!("read_add_remove.bot_notconnect", lang)),
        }
    }
}

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("read_remove").description(lang_t!("read_remove.command.description", lang))
}
