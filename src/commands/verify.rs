//! Performs user verification
//!
//! Verifaction works by first genrating an XChaCha20Poly1305 encrypted json containing the user id and email,
//! and sending it to the purdue email.
//! Then the user types in the toke and it is decrypted and added to the database.

use base64::prelude::*;
use chacha20poly1305::{
    aead::{Aead, OsRng},
    AeadCore, XChaCha20Poly1305,
};
use email_address_parser::EmailAddress;
use serde::{Deserialize, Serialize};
use serenity::all::UserId;

use super::{add_role_to_user, CmdContext, Error};
use crate::config::config;

const NONCE_SIZE: usize = 24;

#[derive(Serialize, Deserialize)]
struct TokenData<'a> {
    id: u64,
    email: &'a str,
}

#[poise::command(slash_command, subcommands("email", "token"))]
pub async fn verify(_ctx: CmdContext<'_>) -> Result<(), Error> {
    Ok(())
}

/// Enter your purdue email to recieve a verification token
#[poise::command(slash_command)]
pub async fn email(
    ctx: CmdContext<'_>,
    #[description = "Purdue email to send verification token to"] email: String,
) -> Result<(), Error> {
    // None for strict email parsing
    let parsed_email =
        EmailAddress::parse(&email, None).ok_or(anyhow::anyhow!("Invalid email address"))?;

    if parsed_email.get_domain() != "purdue.edu" {
        return Err(anyhow::anyhow!("Email is not a purdue.edu email"));
    }

    let user_id = ctx.author().id.get();
    let token_data = TokenData {
        id: user_id,
        email: &email,
    };

    let token_json =
        serde_json::to_string(&token_data).or(Err(anyhow::anyhow!("Could not generate token")))?;

    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let mut token = ctx
        .data()
        .verify_token_cipher
        .encrypt(&nonce, token_json.as_bytes())
        .or(Err(anyhow::anyhow!("Could not generate token")))?;

    token.extend(nonce);

    let token_base64 = BASE64_STANDARD.encode(token);

    let email_text = config().mailgun.verify_email_template.replace("{token}", &token_base64);
    ctx.data().email_client.send_email(
        &email,
        &config().mailgun.verify_email_title,
        &email_text,
    ).await?;

    ctx.say("Verification token has been sent to your purdue email")
        .await?;

    Ok(())
}

/// Enter the token you recieved in your purdue email to verify yourself
#[poise::command(slash_command)]
pub async fn token(
    ctx: CmdContext<'_>,
    #[description = "Verification token recieved from /verify email"] token: String,
) -> Result<(), Error> {
    let token_bytes = BASE64_STANDARD.decode(token)?;

    if token_bytes.len() < NONCE_SIZE {
        return Err(anyhow::anyhow!("Invalid token"));
    }

    // nonce is last 24 bytes of token
    let nonce = &token_bytes[token_bytes.len() - NONCE_SIZE..];
    let nonce: [u8; NONCE_SIZE] = nonce.try_into().unwrap();

    let ciphertext = &token_bytes[..token_bytes.len() - NONCE_SIZE];

    let token_bytes = ctx
        .data()
        .verify_token_cipher
        .decrypt(&nonce.into(), ciphertext)
        .or(Err(anyhow::anyhow!("Invalid token")))?;

    let token_data: TokenData<'_> = serde_json::from_slice(&token_bytes)?;
    let id = UserId::new(token_data.id);

    // make sure verify token is being run on same discord account that used verify email
    if id != ctx.author().id {
        return Err(anyhow::anyhow!(
            "Discord user id does not match token user id"
        ));
    }

    let mut conn = ctx.data().conn().await;

    // make sure email is unique
    if conn.get_user_by_email(token_data.email).await.is_ok() {
        return Err(anyhow::anyhow!("Email is already verified"));
    }

    // make sure discord user is unique
    if let Ok(user) = conn.get_user_by_id(id).await {
        if user.is_verified() {
            return Err(anyhow::anyhow!("Discord account is already verified"));
        }
    }

    conn.verify_user(id, &token_data.email).await?;
    add_role_to_user(ctx.serenity_context(), id, &config().server.member_role).await?;

    conn.commit().await?;

    ctx.say("User validated!").await?;

    Ok(())
}
