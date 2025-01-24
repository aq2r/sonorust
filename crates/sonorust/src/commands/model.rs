use std::ops::Deref;

use langrustang::lang_t;
use sbv2_api::{Sbv2ModelInfo, SBV2_MODELINFO};
use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateCommand, CreateEmbed, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use setting_inputter::{settings_json::InferUse, SettingsJson};

use crate::crate_extensions::{sbv2_api_rust::TTS_MODEL_HOLDER, SettingsJsonExtension};

pub async fn model() -> (CreateEmbed, Vec<CreateActionRow>) {
    let infer_use = SettingsJson::get_sbv2_inferuse();

    match infer_use {
        InferUse::Python => sbv2_model().await,
        InferUse::Rust => rust_model().await,
    }
}

async fn sbv2_model() -> (CreateEmbed, Vec<CreateActionRow>) {
    // API のモデルデータを取得
    let lock = SBV2_MODELINFO.read().unwrap();
    let sbv2_modelinfo = lock.deref();

    // embed とプルダウンリスト作成
    let embed = sbv2_create_embed(sbv2_modelinfo);
    let select_menu = sbv2_create_select_menu(sbv2_modelinfo);

    // コンポーネントの行を作成
    let row0 = CreateActionRow::SelectMenu(select_menu);
    let mut components_vec = vec![row0];

    // モデルの数が 26 以上ならページ移動ボタンを追加
    if sbv2_modelinfo.id_to_model.len() >= 26 {
        components_vec.push(create_button_row());
    }

    (embed, components_vec)
}

fn sbv2_create_embed(apimodelinfo: &Sbv2ModelInfo) -> CreateEmbed {
    // model 25個分の表示を作成 25個以下だったらそこで終了
    let mut content = String::new();
    for i in 0..=24 {
        match apimodelinfo.id_to_model.get(&i) {
            Some(model) => {
                let text = format!("{}: {}\n", i + 1, model.model_name);
                content += &text
            }
            None => break,
        }
    }

    CreateEmbed::new()
        .title("使用できるモデル一覧")
        .description(content)
}

fn sbv2_create_select_menu(apimodelinfo: &Sbv2ModelInfo) -> CreateSelectMenu {
    // model 25個までのプルダウンリストを作成
    let mut selectoption_vec = vec![];
    for i in 0..=24 {
        match apimodelinfo.id_to_model.get(&i) {
            Some(model) => selectoption_vec.push(CreateSelectMenuOption::new(
                model.model_name.as_str(),
                model.model_name.as_str(),
            )),
            None => break,
        }
    }

    CreateSelectMenu::new(
        lang_t!("customid.select.model"),
        CreateSelectMenuKind::String {
            options: selectoption_vec,
        },
    )
}

async fn rust_model() -> (CreateEmbed, Vec<CreateActionRow>) {
    let lock = TTS_MODEL_HOLDER.lock().await;
    let model_holder = lock.as_ref().unwrap();

    let model_idents = model_holder.model_idents();

    // embed 作成
    let mut content = String::new();
    for i in 0..24 {
        match model_idents.get(i) {
            Some(model_name) => {
                let text = format!("{}: {}\n", i + 1, model_name);
                content += &text
            }
            None => break,
        }
    }

    let embed = CreateEmbed::new()
        .title("使用できるモデル一覧")
        .description(content);

    // component 作成
    let mut selectoption_vec = vec![];
    for i in 0..24 {
        match model_idents.get(i) {
            Some(model_name) => {
                selectoption_vec.push(CreateSelectMenuOption::new(
                    model_name.as_str(),
                    model_name.as_str(),
                ));
            }

            None => break,
        }
    }

    let select_menu = CreateSelectMenu::new(
        lang_t!("customid.select.model"),
        CreateSelectMenuKind::String {
            options: selectoption_vec,
        },
    );

    let row0 = CreateActionRow::SelectMenu(select_menu);
    let components_vec = vec![row0];

    // Rust版 sbv2 では現在26個以上のモデルは非対応、対応するかは未定

    // モデルの数が 26 以上ならページ移動ボタンを追加
    // if model_idents.len() >= 26 {
    //     components_vec.push(create_button_row());
    // }

    (embed, components_vec)
}

fn create_button_row() -> CreateActionRow {
    let page_back = CreateButton::new(lang_t!("customid.page.model.back"))
        .label("<-")
        .style(ButtonStyle::Primary)
        .disabled(true);

    let page_number = CreateButton::new(lang_t!("customid.page.model.number"))
        .label("1")
        .style(ButtonStyle::Secondary)
        .disabled(true);

    let page_forward = CreateButton::new(lang_t!("customid.page.model.forward"))
        .label("->")
        .style(ButtonStyle::Primary)
        .disabled(false);

    CreateActionRow::Buttons(vec![page_back, page_number, page_forward])
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("model").description(lang_t!("model.command.description", lang))
}
