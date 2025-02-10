use either::Either;
use infer_api::{Sbv2PythonModelMap, Sbv2RustModel};
use langrustang::lang_t;
use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateCommand, CreateEmbed, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};

use crate::{Handler, _langrustang_autogen::Lang};
pub async fn model(handler: &Handler, lang: Lang) -> (CreateEmbed, Vec<CreateActionRow>) {
    let (model_names, is_model_26_more) = {
        let client = handler.infer_client.read().await;
        match client.as_ref() {
            Either::Left(python_client) => get_python_modelnames(python_client.model_info()),
            Either::Right(rust_client) => get_rust_model_names(rust_client.get_modelinfo()),
        }
    };

    let embed = {
        let content = model_names
            .iter()
            .enumerate()
            .map(|(idx, name)| format!("{}: {name}", idx + 1,))
            .collect::<Vec<_>>()
            .join("\n");

        CreateEmbed::new()
            .title(lang_t!("model.embed.title", lang))
            .description(content)
    };

    let select_menu = {
        let mut selectoption_vec = vec![];

        for i in model_names.iter() {
            selectoption_vec.push(CreateSelectMenuOption::new(i, i));
        }

        CreateSelectMenu::new(
            lang_t!("customid.select.model"),
            CreateSelectMenuKind::String {
                options: selectoption_vec,
            },
        )
    };

    // コンポーネントの行を作成
    let row0 = CreateActionRow::SelectMenu(select_menu);
    let mut components_vec = vec![row0];

    // モデルの数が 26 以上ならページ移動ボタンを追加
    if is_model_26_more {
        components_vec.push(create_button_row());
    }

    (embed, components_vec)
}

fn get_python_modelnames(model_info: &Sbv2PythonModelMap) -> (Vec<String>, bool) {
    let mut result = vec![];
    let mut is_model_26_more = true;

    for i in 0..=24 {
        match model_info.id_to_model.get(&i) {
            Some(model) => result.push(model.model_name.clone()),
            None => {
                is_model_26_more = false;
                break;
            }
        }
    }

    (result, is_model_26_more)
}

fn get_rust_model_names(model_info: &Vec<Sbv2RustModel>) -> (Vec<String>, bool) {
    let mut result = vec![];
    let mut is_model_26_more = true;

    for i in 0..=24 {
        match model_info.get(i) {
            Some(model) => result.push(model.name.clone()),
            None => {
                is_model_26_more = false;
                break;
            }
        }
    }

    (result, is_model_26_more)
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

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("model").description(lang_t!("model.command.description", lang))
}
