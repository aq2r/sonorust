use std::future::Future;

use either::Either;
use langrustang::{format_t, lang_t};
use serenity::all::{
    ButtonStyle, ComponentInteraction, Context, CreateActionRow, CreateButton, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use sonorust_db::UserData;

use crate::{
    crate_extensions::{rwlock::RwLockExt, sonorust_setting::SettingJsonExt},
    errors::SonorustError,
    Handler,
    _langrustang_autogen::Lang,
};

pub async fn move_page(
    handler: &Handler,
    ctx: &Context,
    interaction: &ComponentInteraction,
    custom_id: &str,
) -> Result<(), SonorustError> {
    let lang = handler.setting_json.get_bot_lang();
    let userdata = UserData::from(interaction.user.id).await?;
    let default_model = handler
        .setting_json
        .with_read(|lock| lock.default_model.clone());

    // 現在何ページ目かを取得
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

    let get_model_names = || async {
        let client = handler.infer_client.read().await;
        let mut result = vec![];

        match client.as_ref() {
            Either::Left(python_client) => {
                let model_info = python_client.model_info();
                result.reserve_exact(model_info.id_to_model.len());

                for i in 0.. {
                    match model_info.id_to_model.get(&i) {
                        Some(model) => result.push(model.model_name.clone()),
                        None => break,
                    }
                }
            }
            Either::Right(rust_client) => {
                let model_info = rust_client.get_modelinfo();
                result.reserve_exact(model_info.len());

                for i in model_info {
                    result.push(i.name.clone());
                }
            }
        }

        result
    };

    let get_model_speaker_names = || async {
        let client = handler.infer_client.read().await;

        let model_name;
        let mut style_names = vec![];

        match client.as_ref() {
            Either::Left(python_client) => {
                let valid_model = python_client
                    .get_valid_model(
                        &userdata.model_name,
                        &userdata.speaker_name,
                        &userdata.style_name,
                        &default_model,
                    )
                    .await;

                let model_info = python_client.model_info();
                let model = model_info
                    .id_to_model
                    .get(&valid_model.model_id)
                    .unwrap_or_else(|| model_info.id_to_model.get(&0).unwrap());

                for i in 0.. {
                    match model.id2spk.get(&i) {
                        Some(spk_name) => style_names.push(spk_name.clone()),
                        None => break,
                    }
                }

                model_name = model.model_name.clone();
            }
            Either::Right(rust_client) => {
                let valid_model = rust_client.get_valid_model(&userdata.model_name, &default_model);

                style_names.push("Default".to_string());
                model_name = valid_model.name.clone();
            }
        }

        (model_name, style_names)
    };

    let get_model_style_names = || async {
        let client = handler.infer_client.read().await;

        let model_name;
        let mut style_names = vec![];

        match client.as_ref() {
            Either::Left(python_client) => {
                let valid_model = python_client
                    .get_valid_model(
                        &userdata.model_name,
                        &userdata.speaker_name,
                        &userdata.style_name,
                        &default_model,
                    )
                    .await;

                let model_info = python_client.model_info();
                let model = model_info
                    .id_to_model
                    .get(&valid_model.model_id)
                    .unwrap_or_else(|| model_info.id_to_model.get(&0).unwrap());

                for i in 0.. {
                    match model.id2style.get(&i) {
                        Some(style_name) => style_names.push(style_name.clone()),
                        None => break,
                    }
                }

                model_name = model.model_name.clone();
            }
            Either::Right(rust_client) => {
                let valid_model = rust_client.get_valid_model(&userdata.model_name, &default_model);

                style_names.push("Default".to_string());
                model_name = valid_model.name.clone();
            }
        }

        (model_name, style_names)
    };

    let (embed, select_menu, button_row) = match custom_id {
        lang_t!("customid.page.model.forward") => {
            model_pageforward(get_model_names, current_page, lang).await
        }
        lang_t!("customid.page.model.back") => {
            model_pageback(get_model_names, current_page, lang).await
        }

        lang_t!("customid.page.speaker.forward") => {
            speaker_pageforward(get_model_speaker_names, current_page, lang).await
        }
        lang_t!("customid.page.speaker.back") => {
            speaker_pageback(get_model_speaker_names, current_page, lang).await
        }

        lang_t!("customid.page.style.forward") => {
            style_pageforward(get_model_style_names, current_page, lang).await
        }
        lang_t!("customid.page.style.back") => {
            style_pageback(get_model_style_names, current_page, lang).await
        }

        _ => unreachable!(),
    };

    let builder = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![CreateActionRow::SelectMenu(select_menu), button_row]),
    );
    interaction.create_response(&ctx.http, builder).await?;

    Ok(())
}

/* custom_id ごとの動作 */

async fn model_pageforward<F, FR>(
    get_model_names: F,
    current_page: usize,
    lang: Lang,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow)
where
    F: FnOnce() -> FR,
    FR: Future<Output = Vec<String>>,
{
    let model_names = get_model_names().await;

    let embed = create_embed(
        lang_t!("model.embed.title", lang),
        &model_names,
        current_page,
    );
    let select_menu =
        create_select_menu_model(lang_t!("customid.select.model"), &model_names, current_page);
    let button_row = create_button_row_forward(
        current_page,
        &model_names,
        lang_t!("customid.page.model.back"),
        lang_t!("customid.page.model.number"),
        lang_t!("customid.page.model.forward"),
    );

    (embed, select_menu, button_row)
}

async fn model_pageback<F, FR>(
    get_model_names: F,
    current_page: usize,
    lang: Lang,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow)
where
    F: FnOnce() -> FR,
    FR: Future<Output = Vec<String>>,
{
    let model_names = get_model_names().await;
    let page = current_page - 2;

    let embed = create_embed(lang_t!("model.embed.title", lang), &model_names, page);
    let select_menu =
        create_select_menu_model(lang_t!("customid.select.model"), &model_names, page);
    let button_row = create_button_row_back(
        current_page,
        lang_t!("customid.page.model.back"),
        lang_t!("customid.page.model.number"),
        lang_t!("customid.page.model.forward"),
    );

    (embed, select_menu, button_row)
}

async fn speaker_pageforward<F, FR>(
    get_model_speaker_names: F,
    current_page: usize,
    lang: Lang,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow)
where
    F: FnOnce() -> FR,
    FR: Future<Output = (String, Vec<String>)>,
{
    let (model_name, speaker_names) = get_model_speaker_names().await;

    let embed = create_embed(
        &format_t!("speaker.embed.title", lang, model_name),
        &speaker_names,
        current_page,
    );
    let select_menu = create_select_menu_otherthan_model(
        lang_t!("customid.select.speaker"),
        &model_name,
        &speaker_names,
        current_page,
    );
    let button_row = create_button_row_forward(
        current_page,
        &speaker_names,
        lang_t!("customid.page.speaker.back"),
        lang_t!("customid.page.speaker.number"),
        lang_t!("customid.page.speaker.forward"),
    );

    (embed, select_menu, button_row)
}

async fn speaker_pageback<F, FR>(
    get_model_speaker_names: F,
    current_page: usize,
    lang: Lang,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow)
where
    F: FnOnce() -> FR,
    FR: Future<Output = (String, Vec<String>)>,
{
    let page = current_page - 2;
    let (model_name, speaker_names) = get_model_speaker_names().await;

    let embed = create_embed(
        &format_t!("speaker.embed.title", lang, model_name),
        &speaker_names,
        page,
    );
    let select_menu = create_select_menu_otherthan_model(
        lang_t!("customid.select.speaker"),
        &model_name,
        &speaker_names,
        page,
    );
    let button_row = create_button_row_back(
        current_page,
        lang_t!("customid.page.speaker.back"),
        lang_t!("customid.page.speaker.number"),
        lang_t!("customid.page.speaker.forward"),
    );

    (embed, select_menu, button_row)
}

async fn style_pageforward<F, FR>(
    get_model_style_names: F,
    current_page: usize,
    lang: Lang,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow)
where
    F: FnOnce() -> FR,
    FR: Future<Output = (String, Vec<String>)>,
{
    let (model_name, style_names) = get_model_style_names().await;

    let embed = create_embed(
        &format_t!("style.embed.title", lang, model_name),
        &style_names,
        current_page,
    );
    let select_menu = create_select_menu_otherthan_model(
        lang_t!("customid.select.style"),
        &model_name,
        &style_names,
        current_page,
    );
    let button_row = create_button_row_forward(
        current_page,
        &style_names,
        lang_t!("customid.page.style.forward"),
        lang_t!("customid.page.style.number"),
        lang_t!("customid.page.style.back"),
    );

    (embed, select_menu, button_row)
}

async fn style_pageback<F, FR>(
    get_model_style_names: F,
    current_page: usize,
    lang: Lang,
) -> (CreateEmbed, CreateSelectMenu, CreateActionRow)
where
    F: FnOnce() -> FR,
    FR: Future<Output = (String, Vec<String>)>,
{
    let page = current_page - 2;
    let (model_name, style_names) = get_model_style_names().await;

    let embed = create_embed(
        &format_t!("style.embed.title", lang, model_name),
        &style_names,
        page,
    );
    let select_menu = create_select_menu_otherthan_model(
        lang_t!("customid.select.style"),
        &model_name,
        &style_names,
        page,
    );
    let button_row = create_button_row_back(
        current_page,
        lang_t!("customid.page.style.forward"),
        lang_t!("customid.page.style.number"),
        lang_t!("customid.page.style.back"),
    );

    (embed, select_menu, button_row)
}

/* embed, select_menu, button 作成部分の処理 */

fn create_embed(title: &str, name_vec: &Vec<String>, page: usize) -> CreateEmbed {
    // 25個分の表示を作成 25個以下だったらそこで終了
    // page数 * 25 を足して次のページを取得する
    let mut content = String::new();
    for i in (page * 25)..=24 + (page * 25) {
        match name_vec.get(i) {
            Some(s) => {
                let text = format!("{}: {}\n", i + 1, s);
                content += &text
            }
            None => break,
        }
    }

    CreateEmbed::new().title(title).description(content)
}

fn create_select_menu_model(
    custom_id: &str,
    name_vec: &Vec<String>,
    page: usize,
) -> CreateSelectMenu {
    // 25個までのプルダウンリストを作成
    // page数 * 25 を足して次のページを取得する
    let mut selectoption_vec = vec![];
    for i in (page * 25)..=24 + (page * 25) {
        match name_vec.get(i) {
            Some(s) => selectoption_vec.push(CreateSelectMenuOption::new(s, s)),
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

fn create_select_menu_otherthan_model(
    custom_id: &str,
    model_name: &str,
    name_vec: &Vec<String>,
    page: usize,
) -> CreateSelectMenu {
    // 25個までのプルダウンリストを作成
    // page数 * 25 を足して次のページを取得する
    let mut selectoption_vec = vec![];
    for i in (page * 25)..=24 + (page * 25) {
        match name_vec.get(i) {
            Some(s) => selectoption_vec.push(CreateSelectMenuOption::new(
                s,
                format!("{}||{}", model_name, s),
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

fn create_button_row_forward(
    current_page: usize,
    name_vec: &Vec<String>,
    custom_id_back: &str,
    custom_id_num: &str,
    custom_id_forward: &str,
) -> CreateActionRow {
    let next_page = current_page + 1;
    let nextpage_map_number = next_page * 25;

    // 次のページがないなら page_forward ボタンを無効化する
    let is_nextpage_exists = {
        match name_vec.get(nextpage_map_number) {
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

fn create_button_row_back(
    current_page: usize,
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
