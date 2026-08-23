use reqwest::Client;

pub async fn send(
    bot_token: &str,
    chat_id: &str,
    title: &str,
    message: &str,
) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": format!("*{}*\n{}", title, message),
        "parse_mode": "Markdown",
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Telegram HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Telegram API error: {}", text));
    }

    Ok(())
}