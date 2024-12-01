use std::borrow::Borrow;

use langrustang::{format_t, lang_t};
use sbv2_api::{ModelInfo, SBV2_MODELINFO};
use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateCommand, CreateEmbed, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption, UserId,
};
use setting_inputter::SettingsJson;
use sonorust_db::UserData;

use crate::{crate_extensions::SettingsJsonExtension, errors::SonorustError};

pub async fn style(user_id: UserId) -> Result<(CreateEmbed, Vec<CreateActionRow>), SonorustError> {
    let userdata = UserData::from(user_id).await?;

    // API のモデルデータを取得
    let lock = SBV2_MODELINFO.read().unwrap();
    let sbv2_modelinfo = lock.borrow();

    // そのユーザーのモデルを取得、ないなら id が 0 の物を取得
    let model = match sbv2_modelinfo.name_to_model.get(&userdata.model_name) {
        Some(model) => model,
        None => sbv2_modelinfo.id_to_model.get(&0).unwrap(),
    };

    // embed とプルダウンリスト作成
    let embed = create_embed(&model);
    let select_menu = create_select_menu(&model);

    // コンポーネントの行を作成
    let row0 = CreateActionRow::SelectMenu(select_menu);
    let mut components_vec = vec![row0];

    // スタイルの数が 26 以上ならページ移動ボタンを追加
    if model.id2style.len() >= 26 {
        components_vec.push(create_button_row());
    }

    Ok((embed, components_vec))
}

fn create_embed(model: &ModelInfo) -> CreateEmbed {
    // model 25個分の表示を作成 25個以下だったらそこで終了
    let mut content = String::new();
    for i in 0..=24 {
        match model.id2style.get(&i) {
            Some(spk_name) => {
                let text = format!("{}: {}\n", i + 1, spk_name);
                content += &text
            }
            None => break,
        }
    }

    let lang = SettingsJson::get_bot_lang();
    let title = format_t!("style.embed.title", lang, model.model_name);

    CreateEmbed::new().title(title).description(content)
}

fn create_select_menu(model: &ModelInfo) -> CreateSelectMenu {
    // model 25個までのプルダウンリストを作成
    let mut selectoption_vec = vec![];
    for i in 0..=24 {
        match model.id2style.get(&i) {
            Some(style_name) => selectoption_vec.push(CreateSelectMenuOption::new(
                style_name.as_str(),
                format!("{}||{}", model.model_name, style_name),
            )),
            None => break,
        }
    }

    CreateSelectMenu::new(
        lang_t!("customid.select.style"),
        CreateSelectMenuKind::String {
            options: selectoption_vec,
        },
    )
}

fn create_button_row() -> CreateActionRow {
    let page_back = CreateButton::new(lang_t!("customid.page.style.forward"))
        .label("<-")
        .style(ButtonStyle::Primary)
        .disabled(true);

    let page_number = CreateButton::new(lang_t!("customid.page.style.number"))
        .label("1")
        .style(ButtonStyle::Secondary)
        .disabled(true);

    let page_forward = CreateButton::new(lang_t!("customid.page.style.back"))
        .label("->")
        .style(ButtonStyle::Primary)
        .disabled(false);

    CreateActionRow::Buttons(vec![page_back, page_number, page_forward])
}

pub fn create_command() -> CreateCommand {
    let lang = SettingsJson::get_bot_lang();

    CreateCommand::new("style").description(lang_t!("style.command.description", lang))
}
