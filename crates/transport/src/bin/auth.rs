use std::io::{self, BufRead, Write};

use anyhow::{Result, bail};
use transport::config::Config;
use transport::client::{LoginPrompt, AnchorClient, AnchorClientOptions};

/// script requires ANCHOR_API_ID, ANCHOR_API_HASH and ANCHOR_PHONE in .env

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;

    transport::logger::init();

    let client = AnchorClient::connect(AnchorClientOptions {
        api_id: config.api_id,
        api_hash: config.api_hash.clone(),
        session_file: config.session_file.clone(),
    })
    .await?;

    if client.is_authorized().await? {
        println!("already signed in.");
        client.shutdown().await;
        return Ok(());
    }

    client
        .  sign_in_with_prompt(&config.phone, |step| {
            prompt(match step {
                LoginPrompt::Code => "Code: ",
                LoginPrompt::Password => "2FA password: ",
            })
        })
        .await?;

    println!("ok.");

    client.shutdown().await;
    Ok(())
}

fn prompt(question: &str) -> Result<String> {
    print!("{question}");
    io::stdout().flush()?;
    let mut line = String::new();
    let n = io::stdin().lock().read_line(&mut line)?;
    if n == 0 { bail!("unexpected end of input"); }

    Ok(line.trim().to_string())
}