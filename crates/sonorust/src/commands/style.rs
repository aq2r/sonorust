use either::Either;
use infer_api::{Sbv2PythonModelMap, Sbv2RustClient};
use langrustang::{format_t, lang_t};
use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateCommand, CreateEmbed, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption, UserId,
};
use sonorust_db::UserData;

use crate::{
    Handler, _langrustang_autogen::Lang, crate_extensions::rwlock::RwLockExt, errors::SonorustError,
};

pub async fn style(
    handler: &Handler,
    user_id: UserId,
    lang: Lang,
) -> Result<(CreateEmbed, Vec<CreateActionRow>), SonorustError> {
    let userdata = UserData::from(user_id).await?;
    let (model_name, style_names, is_model_26_more) = {
        let client = handler.infer_client.read().await;
        match client.as_ref() {
            Either::Left(python_client) => get_python_styles(python_client.model_info(), userdata),
            Either::Right(rust_client) => get_rust_styles(handler, rust_client, userdata),
        }
    };
    
    let embed = {
        let content = style_names
            .iter()
            .enumerate()
            .map(|(idx, name)| format!("{}: {name}", idx + 1,))
            .collect::<Vec<_>>()
            .join("\n");

        CreateEmbed::new()
            .title(format_t!("style.embed.title", lang, model_name))
            .description(content)
    };

    let select_menu = {
        let mut selectoption_vec = vec![];

        for i in style_names.iter() {
            selectoption_vec.push(CreateSelectMenuOption::new(i, i));
        }

        CreateSelectMenu::new(
            lang_t!("customid.select.style"),
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

    Ok((embed, components_vec))
}

fn get_python_styles(
    model_info: &Sbv2PythonModelMap,
    userdata: UserData,
) -> (String, Vec<String>, bool) {
    // そのユーザーのモデルを取得、ないなら id が 0 の物を取得
    let model = match model_info.name_to_model.get(&userdata.model_name) {
        Some(model) => model,
        None => model_info
            .id_to_model
            .get(&0)
            .expect("ID 0 Model not found"),
    };

    let mut result = vec![];
    let mut is_model_26_more = true;

    for i in 0..=24 {
        match model.id2style.get(&i) {
            Some(spk_name) => result.push(spk_name.clone()),
            None => {
                is_model_26_more = false;
                break;
            }
        }
    }

    (model.model_name.clone(), result, is_model_26_more)
}

fn get_rust_styles(
    handler: &Handler,
    rust_client: &Sbv2RustClient,
    userdata: UserData,
) -> (String, Vec<String>, bool) {
    let default_model = handler
        .setting_json
        .with_read(|lock| lock.default_model.clone());
    let valid_model = rust_client.get_valid_model(&userdata.model_name, &default_model);

    (valid_model.name.clone(), vec!["default".to_string()], false)
}

fn create_button_row() -> CreateActionRow {
    let page_back = CreateButton::new(lang_t!("customid.page.style.back"))
        .label("<-")
        .style(ButtonStyle::Primary)
        .disabled(true);

    let page_number = CreateButton::new(lang_t!("customid.page.style.number"))
        .label("1")
        .style(ButtonStyle::Secondary)
        .disabled(true);

    let page_forward = CreateButton::new(lang_t!("customid.page.style.forward"))
        .label("->")
        .style(ButtonStyle::Primary)
        .disabled(false);

    CreateActionRow::Buttons(vec![page_back, page_number, page_forward])
}

pub fn create_command(lang: Lang) -> CreateCommand {
    CreateCommand::new("style").description(lang_t!("style.command.description", lang))
}
