use serenity::all::{Context, VoiceState};

use crate::Handler;

pub async fn voice_state_update(
    handler: &Handler,
    ctx: &Context,
    old: &Option<VoiceState>,
    new: &VoiceState,
) {
}
