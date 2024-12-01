use std::{
    io::{self, stdin, stdout, Write},
    path::PathBuf,
};

use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    execute,
    terminal::{Clear, ClearType},
};
use tokio::{
    fs::{self, create_dir_all, File},
    io::AsyncWriteExt,
};

const TOKEN_FILEPATH: &str = "./appdata/BOT_TOKEN";

pub async fn get_or_set_token() -> anyhow::Result<String> {
    match fs::read_to_string(TOKEN_FILEPATH).await {
        Ok(string) => Ok(string),
        Err(_) => input_token().await,
    }
}

pub async fn input_token() -> anyhow::Result<String> {
    log::info!("Input Bot Token");
    print!("Your Bot Token: ");
    stdout().flush()?;

    let mut buffer = String::new();
    stdin().read_line(&mut buffer)?;

    let input = buffer.trim().to_string();

    create_dir_all(PathBuf::from(TOKEN_FILEPATH).ancestors().nth(1).unwrap()).await?;
    let mut file = File::create(TOKEN_FILEPATH).await?;
    file.write_all(input.as_bytes()).await?;

    // 入力内容を書き換え
    execute!(
        io::stdout(),
        MoveUp(1),
        Clear(ClearType::FromCursorDown),
        MoveToColumn(0)
    )?;
    println!("Your Bot Token: {}", "*".repeat(input.len()));

    Ok(input)
}
