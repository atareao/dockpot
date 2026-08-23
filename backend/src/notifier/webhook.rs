use reqwest::Client;

pub async fn send(
    url: &str,
    title: &str,
    message: &str,
) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let body = serde_json::json!({
        "title": title,
        "message": message,
    });

    let resp = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Webhook HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Webhook API error: {}", text));
    }

    Ok(())
}