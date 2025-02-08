use anyhow::anyhow;
use either::Either;
use langrustang::lang_t;
use serenity::all::{Context, CreateCommand, UserId};

use crate::{Handler, _langrustang_autogen::Lang, crate_extensions::rwlock::RwLockExt};

pub async fn reload(handler: &Handler, ctx: &Context, user_id: UserId, lang: Lang) -> &'static str {
    let app_owner_id = {
        match ctx.http.get_current_application_info().await {
            Ok(info) => match info.owner {
                Some(owner) => owner.id,
                None => UserId::new(1),
            },
            Err(_) => UserId::new(1),
        }
    };
    let onnx_model_folder = handler
        .setting_json
        .with_read(|lock| lock.onnx_model_path.clone());

    if user_id != app_owner_id {
        return lang_t!("msg.only_owner", lang);
    }

    let result = {
        let mut client = handler.infer_client.write().await;

        match client.as_mut() {
            Either::Left(python_client) => python_client
                .update_modelinfo()
                .await
                .map_err(|err| anyhow!(err)),
            Either::Right(rust_client) => rust_client
                .update_model(onnx_model_folder)
                .await
                .map_err(|err| anyhow!(err)),
        }
    };

    match result {
        Ok(_) => lang_t!("reload.executed", lang),
        Err(_) => lang_t!("msg.failed.update", lang),
    }
}

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("reload").description(lang_t!("reload.command.description", lang))
}
