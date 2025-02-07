use serenity::all::{Context, Ready};

pub async fn ready(_ctx: &Context, ready: &Ready) {
    log::info!("Logged in: {}", ready.user.name)
}
