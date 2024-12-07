use std::sync::LazyLock;

use crate::_langrustang_autogen::Lang;
use sbv2_api::Sbv2Client;
use setting_inputter::settings_json::SettingLang;
use setting_inputter::{settings_json::SETTINGS_JSON, SettingsJson};

// キャッシュしておく (再起動しないと更新されない)
static LANG_CACHE: LazyLock<Lang> = LazyLock::new(|| {
    let lang = {
        let lock = SETTINGS_JSON.read().unwrap();
        lock.bot_lang
    };

    match lang {
        SettingLang::Ja => Lang::Ja,
        SettingLang::En => Lang::En,
    }
});

pub trait SettingsJsonExtension {
    fn get_bot_lang() -> Lang;
    fn get_prefix() -> String;
    fn get_sbv2_client() -> Sbv2Client;
}

impl SettingsJsonExtension for SettingsJson {
    fn get_bot_lang() -> Lang {
        *LANG_CACHE
    }

    fn get_prefix() -> String {
        let lock = SETTINGS_JSON.read().unwrap();
        lock.prefix.clone()
    }

    fn get_sbv2_client() -> Sbv2Client {
        let lock = SETTINGS_JSON.read().unwrap();
        Sbv2Client::from(&lock.host, lock.port)
    }
}
