use serde_json::Value;

pub mod ntfy;
pub mod telegram;
pub mod webhook;

/// Send a notification through the appropriate channel
pub async fn send_notification(
    notifier_type: &str,
    config: &Value,
    title: &str,
    message: &str,
) -> Result<(), String> {
    match notifier_type {
        "telegram" => {
            let bot_token = config
                .get("bot_token")
                .and_then(|v| v.as_str())
                .ok_or("Missing bot_token")?;
            let chat_id = config
                .get("chat_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing chat_id")?;
            telegram::send(bot_token, chat_id, title, message).await
        }
        "ntfy" => {
            let topic = config
                .get("topic")
                .and_then(|v| v.as_str())
                .ok_or("Missing topic")?;
            let server_url = config
                .get("server_url")
                .and_then(|v| v.as_str())
                .unwrap_or("https://ntfy.sh");
            let token = config.get("token").and_then(|v| v.as_str());
            ntfy::send(topic, server_url, token, title, message).await
        }
        "webhook" => {
            let url = config
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing url")?;
            webhook::send(url, title, message).await
        }
        _ => Err(format!("Unknown notifier type: {}", notifier_type)),
    }
}
