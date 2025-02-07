use std::sync::{LazyLock, OnceLock, RwLock};

use sonorust_setting::{BotLang, SettingJson};

use crate::{_langrustang_autogen::Lang, crate_extensions::rwlock::RwLockExt};

pub trait SettingJsonExt {
    fn get_bot_lang(&self) -> Lang;
}

impl SettingJsonExt for RwLock<SettingJson> {
    fn get_bot_lang(&self) -> Lang {
        // 再起動しないと更新されない
        static LANG_CACHE: OnceLock<Lang> = OnceLock::new();

        let lang = LANG_CACHE.get_or_init(|| {
            let bot_lang = self.with_read(|lock| lock.bot_lang);

            match bot_lang {
                BotLang::Ja => Lang::Ja,
                BotLang::En => Lang::En,
            }
        });

        *lang
    }
}
