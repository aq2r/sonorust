use langrustang::format_t;
use serenity::all::{ActivityData, Command, Context, Ready};

use crate::{
    crate_extensions::{rwlock::RwLockExt, sonorust_setting::SettingJsonExt},
    registers::slash_command,
    Handler,
};

pub async fn ready(handler: &Handler, ctx: &Context, ready: &Ready) {
    log::info!("Logged in: {}", ready.user.name);

    let lang = handler.setting_json.get_bot_lang();
    let bot_prefix = handler.setting_json.with_read(|lock| lock.prefix.clone());

    ctx.set_activity(Some(ActivityData::custom(format_t!(
        "msg.bot_activity",
        lang,
        bot_prefix
    ))));

    // スラッシュコマンドの登録
    log::info!("Registering SlashCommands...");

    let commands = slash_command::registers(lang);
    match Command::set_global_commands(&ctx.http, commands).await {
        Ok(_) => log::info!("Slash command has been registered."),
        Err(_) => log::error!("Failed to register slash command."),
    }
}
