use langrustang::lang_t;
use serenity::all::{ComponentInteraction, Context};

use crate::{components, errors::SonorustError, Handler};

pub async fn component(
    handler: &Handler,
    ctx: &Context,
    interaction: &ComponentInteraction,
) -> Result<(), SonorustError> {
    let custom_id = &interaction.data.custom_id;

    log::debug!(
        "Component used: {custom_id} {{ Name: {}, ID: {} }}",
        interaction.user.name,
        interaction.user.id,
    );

    match custom_id.as_str() {
        lang_t!("customid.select.model") => {
            components::select_menu::model(handler, ctx, interaction).await?
        }
        lang_t!("customid.select.speaker") => {
            components::select_menu::speaker(handler, ctx, interaction).await?
        }
        lang_t!("customid.select.style") => {
            components::select_menu::style(handler, ctx, interaction).await?
        }

        lang_t!("customid.page.model.forward") => {
            components::button::move_page(handler, ctx, interaction, custom_id).await?
        }
        lang_t!("customid.page.model.back") => {
            components::button::move_page(handler, ctx, interaction, custom_id).await?
        }
        lang_t!("customid.page.speaker.forward") => {
            components::button::move_page(handler, ctx, interaction, custom_id).await?
        }
        lang_t!("customid.page.speaker.back") => {
            components::button::move_page(handler, ctx, interaction, custom_id).await?
        }
        lang_t!("customid.page.style.forward") => {
            components::button::move_page(handler, ctx, interaction, custom_id).await?
        }
        lang_t!("customid.page.style.back") => {
            components::button::move_page(handler, ctx, interaction, custom_id).await?
        }

        lang_t!("customid.change_server_settings") => {
            components::select_menu::server(handler, ctx, interaction).await?
        }

        lang_t!("customid.dict.add") => {
            components::button::dict_add(handler, ctx, interaction).await?
        }
        lang_t!("customid.dict.remove") => {
            components::button::dict_remove(handler, ctx, interaction).await?
        }

        _ => {
            log::error!(lang_t!("log.not_implemented_customid"));
        }
    }

    Ok(())
}
