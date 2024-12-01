use langrustang::lang_t;
use sbv2_api::Sbv2Client;
use serenity::all::{CreateCommand, UserId};
use setting_inputter::{settings_json::SETTINGS_JSON, SettingsJson};

use crate::{crate_extensions::SettingsJsonExtension, registers::APP_OWNER_ID};

pub async fn reload(user_id: UserId) -> anyhow::Result<&'static str> {
    let lang = SettingsJson::get_bot_lang();

    let app_owner_id = {
        let lock = APP_OWNER_ID.read().unwrap();
        *lock
    };

    if Some(user_id) != app_owner_id {
        return Ok(lang_t!("msg.only_owner", lang));
    }

    let client = {
        let lock = SETTINGS_JSON.read().unwrap();
        Sbv2Client::from(&lock.host, lock.port)
    };

    client.update_modelinfo().await?;

    Ok(lang_t!("reload.executed", lang))
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("reload").description(lang_t!("reload.command.description", lang))
}
