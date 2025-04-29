use crate::config::config;

pub struct EmailClient {
    mailgun_token: String,
}

impl EmailClient {
    pub fn new(mailgun_token: String) -> Self {
        EmailClient { mailgun_token }
    }

    pub async fn send_email(&self, dest_addr: &str, title: &str, body: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();

        let url = format!("{}/messages", config().mailgun.api_base_url);
        let result = client
            .post(url)
            .basic_auth("api", Some(&self.mailgun_token))
            .form(&[
                ("from", config().mailgun.email_address.as_str()),
                ("to", dest_addr),
                ("subject", title),
                ("html", body),
            ])
            .send()
            .await?;

        if !result.status().is_success() {
            Err(anyhow::anyhow!(
                "Failed to send verification email: {result:?}"
            ))
        } else {
            Ok(())
        }
    }
}
