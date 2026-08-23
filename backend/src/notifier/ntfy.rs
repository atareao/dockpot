use reqwest::Client;

pub async fn send(
    topic: &str,
    server_url: &str,
    token: Option<&str>,
    title: &str,
    message: &str,
) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = format!("{}/{}", server_url.trim_end_matches('/'), topic);

    let mut req = client.post(&url).header("Title", title).body(message.to_string());

    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("ntfy HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("ntfy API error: {}", text));
    }

    Ok(())
}