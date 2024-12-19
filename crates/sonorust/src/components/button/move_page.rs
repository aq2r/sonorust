use std::{borrow::Borrow, collections::HashMap};

use langrustang::{format_t, lang_t};
use sbv2_api::{Sbv2Client, Sbv2ModelInfo, SBV2_MODELINFO};
use serenity::all::{
    ButtonStyle, ComponentInteraction, Context, CreateActionRow, CreateButton, CreateEmbed,
    CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use setting_inputter::SettingsJson;
use sonorust_db::UserData;

use crate::{
    crate_extensions::{sbv2_api::Sbv2ClientExtension, SettingsJsonExtension},
    errors::SonorustError,
};

pub async fn move_page(
    ctx: &Context,
    interaction: &ComponentInteraction,
    custom_id: &str,
) -> Result<(), SonorustError> {
    // ユーザーデータとモデルを取得する
    let user_data = UserData::from(interaction.user.id).await?;

    // custom_id ごとに要素を作成
    let (embed, select_menu, button_row) = {
        let lock = SBV2_MODELINFO.read().unwrap();
        let sbv2_modelinfo = lock.borrow();

        let button_row = &interaction.message.components[1];
        let page_button = match &button_row.components[1] {
            serenity::all::ActionRowComponent::Button(button) => button,
            _ => unreachable!(),
        };

        let current_page = page_button
            .label
            .as_ref()
            .map(|i| i.parse().unwrap_or(0))
            .unwrap_or(0);

        match custom_id {
            lang_t!("customid.page.model.forward") => {
                model_pageforward(sbv2_modelinfo, current_page)
            }
            lang_t!("customid.page.model.back") => model_pageback(sbv2_modelinfo, current_page),

            lang_t!("customid.page.speaker.forward") => {
                speaker_pageforward(&user_data, sbv2_modelinfo, current_page)
            }
            lang_t!("customid.page.speaker.back") => {
                speaker_pageback(&user_data, sbv2_modelinfo, current_page)
            }

            lang_t!("customid.page.style.forward") => {
                style_pageforward(&user_data, sbv2_modelinfo, current_page)
            }
            lang_t!("customid.page.style.back") => {
                style_pageback(&user_data, sbv2_modelinfo, current_page)
            }
            _ => unreachable!(),
        }
    };

    eq_uilibrium::create_response_msg!(
        interaction,
        &ctx.http,
        embed = embed,
        components = vec![CreateActionRow::SelectMenu(select_menu), button_row],
    )
    .await?;
    Ok(())
}

/* custom_id ごとの動作 */

fn model_pageforward(
    sbv2_modelinfo: &Sbv2ModelInfo,
    current_page: u32,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow) {
    let lang = SettingsJson::get_bot_lang();

    let map: HashMap<_, _> = sbv2_modelinfo
        .id_to_model
        .iter()
        .map(|(k, v)| (*k, v.model_name.as_str()))
        .collect();

    let embed = create_embed(lang_t!("model.embed.title", lang), &map, current_page);
    let select_menu =
        create_select_menu_model(lang_t!("customid.select.model"), &map, current_page);
    let button_row = craete_button_row_forward(
        current_page,
        &map,
        lang_t!("customid.page.model.back"),
        lang_t!("customid.page.model.number"),
        lang_t!("customid.page.model.forward"),
    );

    (embed, select_menu, button_row)
}

fn model_pageback(
    sbv2_modelinfo: &Sbv2ModelInfo,
    current_page: u32,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow) {
    let lang = SettingsJson::get_bot_lang();

    let page = current_page - 2;
    let map: HashMap<_, _> = sbv2_modelinfo
        .id_to_model
        .iter()
        .map(|(k, v)| (*k, v.model_name.as_str()))
        .collect();

    let embed = create_embed(lang_t!("model.embed.title", lang), &map, page);
    let select_menu = create_select_menu_model(lang_t!("customid.select.model"), &map, page);
    let button_row = craete_button_row_back(
        current_page,
        lang_t!("customid.page.model.back"),
        lang_t!("customid.page.model.number"),
        lang_t!("customid.page.model.forward"),
    );

    (embed, select_menu, button_row)
}

fn speaker_pageforward(
    userdata: &UserData,
    sbv2_modelinfo: &Sbv2ModelInfo,
    current_page: u32,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow) {
    let lang = SettingsJson::get_bot_lang();

    let valid_model = Sbv2Client::get_valid_model_from_userdata(userdata);
    let model = sbv2_modelinfo
        .id_to_model
        .get(&valid_model.model_id)
        .unwrap_or_else(|| sbv2_modelinfo.id_to_model.get(&0).unwrap());

    let map: HashMap<_, _> = model.id2spk.iter().map(|(k, v)| (*k, v.as_str())).collect();

    let embed = create_embed(
        format_t!("speaker.embed.title", lang, model.model_name),
        &map,
        current_page,
    );
    let select_menu = create_select_menu_except_model(
        lang_t!("customid.select.speaker"),
        &model.model_name,
        &map,
        current_page,
    );
    let button_row = craete_button_row_forward(
        current_page,
        &map,
        lang_t!("customid.page.speaker.back"),
        lang_t!("customid.page.speaker.number"),
        lang_t!("customid.page.speaker.forward"),
    );

    (embed, select_menu, button_row)
}

fn speaker_pageback(
    userdata: &UserData,
    sbv2_modelinfo: &Sbv2ModelInfo,
    current_page: u32,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow) {
    let lang = SettingsJson::get_bot_lang();

    let valid_model = Sbv2Client::get_valid_model_from_userdata(userdata);
    let page = current_page - 2;
    let model = sbv2_modelinfo
        .id_to_model
        .get(&valid_model.model_id)
        .unwrap_or_else(|| sbv2_modelinfo.id_to_model.get(&0).unwrap());

    let map: HashMap<_, _> = model.id2spk.iter().map(|(k, v)| (*k, v.as_str())).collect();

    let embed = create_embed(
        format_t!("speaker.embed.title", lang, model.model_name),
        &map,
        page,
    );
    let select_menu = create_select_menu_except_model(
        lang_t!("customid.select.speaker"),
        &model.model_name,
        &map,
        page,
    );
    let button_row = craete_button_row_back(
        current_page,
        lang_t!("customid.page.speaker.back"),
        lang_t!("customid.page.speaker.number"),
        lang_t!("customid.page.speaker.forward"),
    );

    (embed, select_menu, button_row)
}

fn style_pageforward(
    userdata: &UserData,
    sbv2_modelinfo: &Sbv2ModelInfo,
    current_page: u32,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow) {
    let lang = SettingsJson::get_bot_lang();

    let valid_model = Sbv2Client::get_valid_model_from_userdata(userdata);
    let model = sbv2_modelinfo
        .id_to_model
        .get(&valid_model.model_id)
        .unwrap_or_else(|| sbv2_modelinfo.id_to_model.get(&0).unwrap());

    let map: HashMap<_, _> = model
        .id2style
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    let embed = create_embed(
        format_t!("style.embed.title", lang, model.model_name),
        &map,
        current_page,
    );
    let select_menu = create_select_menu_except_model(
        lang_t!("customid.select.style"),
        &model.model_name,
        &map,
        current_page,
    );
    let button_row = craete_button_row_forward(
        current_page,
        &map,
        lang_t!("customid.page.style.forward"),
        lang_t!("customid.page.style.number"),
        lang_t!("customid.page.style.back"),
    );

    (embed, select_menu, button_row)
}

fn style_pageback(
    userdata: &UserData,
    sbv2_modelinfo: &Sbv2ModelInfo,
    current_page: u32,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow) {
    let lang = SettingsJson::get_bot_lang();

    let valid_model = Sbv2Client::get_valid_model_from_userdata(userdata);
    let page = current_page - 2;
    let model = sbv2_modelinfo
        .id_to_model
        .get(&valid_model.model_id)
        .unwrap_or_else(|| sbv2_modelinfo.id_to_model.get(&0).unwrap());

    let map: HashMap<_, _> = model
        .id2style
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    let embed = create_embed(
        format_t!("style.embed.title", lang, model.model_name),
        &map,
        page,
    );
    let select_menu = create_select_menu_except_model(
        lang_t!("customid.select.style"),
        &model.model_name,
        &map,
        page,
    );
    let button_row = craete_button_row_back(
        current_page,
        lang_t!("customid.page.style.forward"),
        lang_t!("customid.page.style.number"),
        lang_t!("customid.page.style.back"),
    );

    (embed, select_menu, button_row)
}

/* embed, select_menu, button 作成部分の処理 */

fn create_embed<S>(title: S, map: &HashMap<u32, &str>, page: u32) -> CreateEmbed
where
    S: Into<String>,
{
    // 25個分の表示を作成 25個以下だったらそこで終了
    // page数 * 25 を足して次のページを取得する
    let mut content = String::new();
    for i in (page * 25)..=24 + (page * 25) {
        match map.get(&i) {
            Some(value) => {
                let text = format!("{}: {}\n", i + 1, value);
                content += &text
            }
            None => break,
        }
    }

    CreateEmbed::new().title(title).description(content)
}

fn create_select_menu_model(
    custom_id: &str,
    map: &HashMap<u32, &str>,
    page: u32,
) -> CreateSelectMenu {
    // 25個までのプルダウンリストを作成
    // page数 * 25 を足して次のページを取得する
    let mut selectoption_vec = vec![];
    for i in (page * 25)..=24 + (page * 25) {
        match map.get(&i) {
            Some(str_) => selectoption_vec.push(CreateSelectMenuOption::new(*str_, *str_)),
            None => break,
        }
    }
    CreateSelectMenu::new(
        custom_id,
        CreateSelectMenuKind::String {
            options: selectoption_vec,
        },
    )
}

fn create_select_menu_except_model(
    custom_id: &str,
    model_name: &str,
    map: &HashMap<u32, &str>,
    page: u32,
) -> CreateSelectMenu {
    // 25個までのプルダウンリストを作成
    // page数 * 25 を足して次のページを取得する
    let mut selectoption_vec = vec![];
    for i in (page * 25)..=24 + (page * 25) {
        match map.get(&i) {
            Some(str_) => selectoption_vec.push(CreateSelectMenuOption::new(
                *str_,
                format!("{}||{}", model_name, str_),
            )),
            None => break,
        }
    }
    CreateSelectMenu::new(
        custom_id,
        CreateSelectMenuKind::String {
            options: selectoption_vec,
        },
    )
}

fn craete_button_row_forward(
    current_page: u32,
    map: &HashMap<u32, &str>,
    custom_id_back: &str,
    custom_id_num: &str,
    custom_id_forward: &str,
) -> CreateActionRow {
    let next_page = current_page + 1;
    let nextpage_map_number = next_page * 25;

    // 次のページがないなら page_forward ボタンを無効化する
    let is_nextpage_exists = {
        match map.get(&nextpage_map_number) {
            Some(_) => false,
            None => true,
        }
    };

    let page_back = CreateButton::new(custom_id_back)
        .label("<-")
        .style(ButtonStyle::Primary)
        .disabled(false);

    let page_number = CreateButton::new(custom_id_num)
        .label(next_page.to_string())
        .style(ButtonStyle::Secondary)
        .disabled(true);

    let page_forward = CreateButton::new(custom_id_forward)
        .label("->")
        .style(ButtonStyle::Primary)
        .disabled(is_nextpage_exists);

    CreateActionRow::Buttons(vec![page_back, page_number, page_forward])
}

fn craete_button_row_back(
    current_page: u32,
    custom_id_back: &str,
    custom_id_num: &str,
    custom_id_forward: &str,
) -> CreateActionRow {
    let prev_page = current_page - 1;

    // 1ページ目なら page_back ボタンを無効化する
    let is_prevpage_exists = match prev_page {
        1 => true,
        _ => false,
    };

    let page_back = CreateButton::new(custom_id_back)
        .label("<-")
        .style(ButtonStyle::Primary)
        .disabled(is_prevpage_exists);

    let page_number = CreateButton::new(custom_id_num)
        .label(prev_page.to_string())
        .style(ButtonStyle::Secondary)
        .disabled(true);

    let page_forward = CreateButton::new(custom_id_forward)
        .label("->")
        .style(ButtonStyle::Primary)
        .disabled(false);

    CreateActionRow::Buttons(vec![page_back, page_number, page_forward])
}
