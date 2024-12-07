use std::{collections::HashMap, time};

use engtokana::EngToKana;
use langrustang::{format_t, lang_t};
use regex::Regex;
use sbv2_api::Sbv2Client;
use serenity::all::{Context, CreateMessage, EditMessage, Message};
use setting_inputter::{settings_json::SETTINGS_JSON, SettingsJson};
use sonorust_db::GuildData;

use crate::{
    commands,
    crate_extensions::{
        sbv2_api::{Sbv2ClientExtension, READ_CHANNELS},
        SettingsJsonExtension,
    },
    errors::SonorustError,
};

pub async fn message(ctx: &Context, msg: &Message) -> Result<(), SonorustError> {
    // 実際の動作は commands フォルダ

    // メッセージを送信したのがBOTだった場合無視
    if msg.author.bot {
        return Ok(());
    }

    let prefix = SettingsJson::get_prefix();

    // prefix から始まっているかによって処理を変える
    match msg.content.starts_with(&prefix) {
        true => command_processing(ctx, msg, &prefix).await,
        false => other_processing(ctx, msg).await,
    }
}

/// メッセージの内容がコマンドだった場合の処理
async fn command_processing(
    ctx: &Context,
    msg: &Message,
    prefix: &str,
) -> Result<(), SonorustError> {
    let lang = SettingsJson::get_bot_lang();

    // メッセージからプレフィックスを除いたものを取得
    let msg_suffix = &msg.content[prefix.len()..];

    // コマンドが使用されたときのデバッグログ
    let debug_log = || {
        log::debug!(
            "MessageCommand used: /{msg_suffix} {{ Name: {}, ID: {} }}",
            msg.author.name,
            msg.author.id,
        )
    };

    match msg_suffix {
        "ping" => {
            debug_log();

            // pong を送信
            let embed = commands::ping::measuring_embed();
            let message = CreateMessage::new().embed(embed);

            let now = time::Instant::now();
            let mut send_msg = msg.channel_id.send_message(&ctx.http, message).await?;
            let elapsed = now.elapsed();

            // description を計測した時間に書き換え
            let embed = commands::ping::measured_embed(elapsed);
            let edit_msg = EditMessage::new().embed(embed);
            send_msg.edit(&ctx.http, edit_msg).await?;
        }

        "help" => {
            debug_log();

            let embed = commands::help(ctx).await;
            eq_uilibrium::send_msg!(&ctx.http, msg.channel_id, embed = embed).await?;
        }

        "now" => {
            debug_log();

            let embed = commands::now(&msg.author).await?;
            eq_uilibrium::send_msg!(&ctx.http, msg.channel_id, embed = embed).await?;
        }

        "join" => {
            debug_log();

            let result = commands::join(ctx, msg.guild_id, msg.channel_id, msg.author.id).await;

            let text = match result {
                Ok(str) => str,
                Err(str) => str,
            };

            let help_embed = commands::help(ctx).await;
            eq_uilibrium::send_msg!(
                &ctx.http,
                msg.channel_id,
                content = text,
                embed = help_embed
            )
            .await?;

            // すでにボイスチャンネルに参加していた場合などは返す
            if let Err(_) = result {
                return Ok(());
            };

            Sbv2Client::play_on_voice_channel(
                ctx,
                msg.guild_id,
                msg.channel_id,
                msg.author.id,
                lang_t!("join.connected", lang),
            )
            .await?;
        }

        "leave" => {
            debug_log();

            let text = commands::leave(ctx, msg.guild_id).await;
            msg.channel_id.say(&ctx.http, text).await?;
        }

        "model" => {
            debug_log();

            let (embed, components) = commands::model().await;
            eq_uilibrium::send_msg!(
                &ctx.http,
                msg.channel_id,
                embed = embed,
                components = components
            )
            .await?;
        }

        "speaker" => {
            debug_log();

            let (embed, components) = commands::speaker(msg.author.id).await?;
            eq_uilibrium::send_msg!(
                &ctx.http,
                msg.channel_id,
                embed = embed,
                components = components
            )
            .await?;
        }

        "style" => {
            debug_log();

            let (embed, components) = commands::style(msg.author.id).await?;
            eq_uilibrium::send_msg!(
                &ctx.http,
                msg.channel_id,
                embed = embed,
                components = components
            )
            .await?;
        }

        "server" => {
            debug_log();

            let (embed, components) = commands::server(ctx, msg.guild_id, msg.author.id).await?;
            eq_uilibrium::send_msg!(
                &ctx.http,
                msg.channel_id,
                embed = embed,
                components = components
            )
            .await?;
        }

        "dict" => {
            debug_log();

            let (embed, components) = commands::dict(ctx, msg.guild_id).await?;
            eq_uilibrium::send_msg!(
                &ctx.http,
                msg.channel_id,
                embed = embed,
                components = components
            )
            .await?;
        }

        "reload" => {
            debug_log();

            let content = match commands::reload(msg.author.id).await {
                Ok(text) => text,
                Err(_) => {
                    log::error!(lang_t!("log.fail_conn_api"));
                    lang_t!("msg.failed.update", lang)
                }
            };

            msg.channel_id.say(&ctx.http, content).await?;
        }

        _ => (),
    }

    if msg_suffix.starts_with("length") {
        debug_log();

        let args: Vec<_> = msg_suffix.split_whitespace().collect();

        // 数字部分を取得
        let Some(length) = args.get(1).map(|i| *i) else {
            msg.channel_id
                .say(&ctx.http, format_t!("length.usage", lang, prefix))
                .await?;
            return Ok(());
        };

        // 数字に変換できなければ返す
        let Ok(length): Result<f64, _> = length.parse() else {
            msg.channel_id
                .say(&ctx.http, lang_t!("length.not_num", lang))
                .await?;
            return Ok(());
        };

        // ユーザーデータを変更してメッセージを送信
        let content = commands::length(msg.author.id, length).await?;
        msg.channel_id.say(&ctx.http, content).await?;
    }

    if msg_suffix.starts_with("wav") {
        debug_log();

        let args: Vec<_> = msg_suffix.split_whitespace().collect();

        // 生成部分を取得
        let Some(content) = args.get(1).map(|i| *i) else {
            msg.channel_id
                .say(&ctx.http, format_t!("wav.usage", lang, prefix))
                .await?;
            return Ok(());
        };

        match commands::wav(msg.author.id, content).await? {
            (None, Some(s)) => {
                eq_uilibrium::send_msg!(&ctx.http, msg.channel_id, content = s).await?;
            }
            (Some(attachment), None) => {
                eq_uilibrium::send_msg!(&ctx.http, msg.channel_id, add_file = attachment).await?;
            }

            _ => (),
        };
    }

    Ok(())
}

/// メッセージの内容がコマンド以外だった場合の処理
async fn other_processing(ctx: &Context, msg: &Message) -> Result<(), SonorustError> {
    // セミコロンから始まっていた場合無視
    if msg.content.starts_with(";") {
        return Ok(());
    }

    // ギルド上でない場合無視
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };

    // 読み上げるチャンネルか確認
    let read_target_ch = {
        let read_channels = READ_CHANNELS.read().unwrap();
        read_channels.get(&guild_id).map(|i| *i)
    };

    if read_target_ch != Some(msg.channel_id) {
        return Ok(());
    }

    // ギルドの設定を取得
    let guilddata = GuildData::from(guild_id).await?;

    let lang = SettingsJson::get_bot_lang();

    // 読み上げ用に文字を置換する
    let mut text_replace = TextReplace::new(&msg.content);

    text_replace.remove_codeblock();
    text_replace.remove_discord_obj();
    text_replace.remove_url();

    text_replace.replace_from_guilddict(&guilddata);

    // 日本語の時のみ英語を日本語読みに変換
    {
        use crate::_langrustang_autogen::Lang::*;

        match lang {
            Ja => text_replace.eng_to_kana(),
            _ => (),
        }
    }

    text_replace.remove_emoji();

    let replaced_text = text_replace.as_string();

    let read_limit = {
        let settings_json = SETTINGS_JSON.read().unwrap();
        settings_json.read_limit
    };

    // read_limit よりも長い場合はその長さに制限する
    let content = match replaced_text.char_indices().nth(read_limit as _) {
        Some((idx, _)) => format_t!("msg.omitted", lang, &msg.content[..idx]),
        None => replaced_text,
    };

    let lang = SettingsJson::get_bot_lang();

    // 設定で ON になっていて添付ファイルがあるなら添付ファイルがあることを知らせる
    if !msg.attachments.is_empty() && guilddata.options.is_notice_attachment {
        Sbv2Client::play_on_voice_channel(
            ctx,
            msg.guild_id,
            msg.channel_id,
            msg.author.id,
            lang_t!("msg.attachments", lang),
        )
        .await?;
    }

    Sbv2Client::play_on_voice_channel(ctx, msg.guild_id, msg.channel_id, msg.author.id, &content)
        .await?;

    Ok(())
}

// 読み方を編集する用 (サーバー辞書など)
#[derive(Debug, Clone)]
pub struct TextReplace {
    text: String,
}

impl TextReplace {
    pub fn new<S>(text: S) -> Self
    where
        S: Into<String>,
    {
        Self { text: text.into() }
    }

    pub fn as_string(self) -> String {
        self.text
    }

    pub fn remove_codeblock(&mut self) {
        // ``` が含まれていた場合全体をコードブロックと読む
        if self.text.contains("```") {
            self.text = "コードブロック".to_string();
            return;
        }

        let re = Regex::new(r"`.*?`").unwrap();
        self.text = re.replace_all(&self.text, "コード").to_string()
    }

    pub fn remove_url(&mut self) {
        let re = Regex::new(r"https?://[\w/:%#\$&\?\(\)~\.=\+\-]+").unwrap();
        self.text = re.replace_all(&self.text, "URL").to_string()
    }

    /// チャンネルやメンション、カスタム絵文字などの置換
    pub fn remove_discord_obj(&mut self) {
        let re = Regex::new(r"<.*?>").unwrap();
        self.text = re.replace_all(&self.text, "").to_string()
    }

    pub fn remove_emoji(&mut self) {
        let re = Regex::new(r"[^\p{L}\p{N}\p{Pd}\p{Sm}\p{Sc}]").unwrap();
        self.text = re.replace_all(&self.text, "").to_string()
    }

    /// 指定したサーバー辞書をもとに置換する
    pub fn replace_from_guilddict(&mut self, guilddata: &GuildData) {
        let map = &guilddata.dict;

        self.replace_from_hashmap(map);
    }

    /// HashMap をもとに HashMap の Key を Value に置換する
    pub fn replace_from_hashmap(&mut self, map: &HashMap<String, String>) {
        let mut replace_texts = HashMap::new();

        for (i, (before, after)) in map.iter().enumerate() {
            let mark = format!("{{|{}|}}", i);

            self.text = self.text.replace(before, &mark).to_string();
            replace_texts.insert(mark, after);
        }

        for (before, after) in replace_texts {
            self.text = self.text.replace(&before, after)
        }
    }

    /// 英語をカタカナ読みに変換する
    pub fn eng_to_kana(&mut self) {
        self.text = EngToKana::convert_all(&self.text);
    }
}
