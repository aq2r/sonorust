use langrustang::lang_t;
use serenity::all::{Context, CreateCommand, GuildId, UserId};

use crate::{
    _langrustang_autogen::Lang,
    crate_extensions::{rwlock::RwLockExt, sonorust_setting::SettingJsonExt},
    errors::SonorustError,
    Handler,
};

pub async fn clear(
    handler: &Handler,
    ctx: &Context,
    guild_id: Option<GuildId>,
    user_id: UserId,
) -> Result<&'static str, SonorustError> {
    let lang = handler.setting_json.get_bot_lang();
    let guild_id = guild_id.ok_or_else(|| SonorustError::GuildIdIsNone)?;

    let is_user_in_vc = {
        let Some(guild) = guild_id.to_guild_cached(&ctx.cache) else {
            return Ok(lang_t!("read_add.failed", lang));
        };

        guild.voice_states.contains_key(&user_id)
    };

    // ユーザーが接続していない場合
    if !is_user_in_vc {
        return Ok(lang_t!("clear.user_notconnect", lang));
    }

    // キューをクリア
    handler.channel_queues.with_write(|lock| {
        let queue = lock.get_mut(&guild_id);

        if let Some(queue) = queue {
            queue.clear();
        }
    });

    let manager = songbird::get(ctx).await.unwrap();
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;

        let queue = handler.queue();
        let _ = queue.skip();
    } else {
        return Ok(lang_t!("clear.bot_notconnect", lang));
    }

    Ok(lang_t!("clear.cleard", lang))
}

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("clear").description(lang_t!("clear.command.description", lang))
}
