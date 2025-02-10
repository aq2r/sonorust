use std::sync::Arc;

use serenity::all::{Http, UserId};

pub trait SerenityHttpExt {
    async fn get_bot_owner_id(&self) -> UserId;
}

impl SerenityHttpExt for Arc<Http> {
    async fn get_bot_owner_id(&self) -> UserId {
        match self.get_current_application_info().await {
            Ok(info) => match info.owner {
                Some(owner) => owner.id,
                None => UserId::new(1),
            },
            Err(_) => UserId::new(1),
        }
    }
}
