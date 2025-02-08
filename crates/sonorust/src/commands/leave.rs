use langrustang::lang_t;
use serenity::all::{Context, GuildId};

use crate::{Handler, _langrustang_autogen::Lang, crate_extensions::rwlock::RwLockExt};

pub async fn leave(
    handler: &Handler,
    ctx: &Context,
    lang: Lang,
    guild_id: Option<GuildId>,
) -> &'static str {
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
            } else {
                log::debug!(
                    "Leaved voice channel (name: {} id: {guild_id})",
                    guild_id
                        .name(&ctx.cache)
                        .unwrap_or_else(|| "Unknown".to_string()),
                )
            }
        }
        None => return lang_t!("leave.already", lang),
    }

    // 読み上げる対象から外す
    handler
        .read_channels
        .with_write(|lock| lock.remove(&guild_id));

    lang_t!("leave.disconnected", lang)
}
