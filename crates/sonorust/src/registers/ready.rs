use langrustang::format_t;
use serenity::all::{ActivityData, Context, Ready};

use crate::{
    crate_extensions::{rwlock::RwLockExt, sonorust_setting::SettingJsonExt},
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
}
