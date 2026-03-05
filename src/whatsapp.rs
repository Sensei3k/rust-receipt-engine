use crate::models::{FileMessageData, Notification};
use reqwest::Client;
use std::path::PathBuf;
use tokio::fs;

/// Directory where downloaded receipt files are saved at runtime.
const DOWNLOAD_DIR: &str = "/private/tmp/receipt_engine";

/// Fetches the oldest pending notification from the Green API queue.
/// Returns None when the queue is empty (Green API responds with "null").
pub async fn receive_notification(
    client: &Client,
    instance_id: &str,
    api_token: &str,
) -> Result<Option<Notification>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.green-api.com/waInstance{}/receiveNotification/{}",
        instance_id, api_token
    );

    let body = client.get(&url).send().await?.text().await?;

    if body.trim() == "null" {
        return Ok(None);
    }

    let notification: Notification = serde_json::from_str(&body)?;
    Ok(Some(notification))
}

/// Acknowledges a notification so Green API removes it from the queue and won't resend it.
pub async fn delete_notification(
    client: &Client,
    instance_id: &str,
    api_token: &str,
    receipt_id: u64,
) -> Result<(), reqwest::Error> {
    let url = format!(
        "https://api.green-api.com/waInstance{}/deleteNotification/{}/{}",
        instance_id, api_token, receipt_id
    );

    client.delete(&url).send().await?;
    Ok(())
}

/// Sends a text message to a WhatsApp chat via Green API.
/// Returns an error if the API responds with a non-success status code.
pub async fn send_message(
    client: &Client,
    instance_id: &str,
    api_token: &str,
    chat_id: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.green-api.com/waInstance{}/sendMessage/{}",
        instance_id, api_token
    );

    let body = serde_json::json!({
        "chatId": chat_id,
        "message": message,
    });

    let response = client.post(&url).json(&body).send().await?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await?;
        return Err(format!("Green API error {}: {}", status, text).into());
    }
    Ok(())
}

/// Downloads a file from the URL in the given FileMessageData and saves it to DOWNLOAD_DIR.
/// The filename is derived from the receipt ID to ensure uniqueness across messages.
pub async fn download_file(
    client: &Client,
    file_data: &FileMessageData,
    receipt_id: u64,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let extension = match file_data.mime_type.as_deref() {
        Some("image/png") => "png",
        Some("image/gif") => "gif",
        Some("image/webp") => "webp",
        Some("application/pdf") => "pdf",
        _ => "jpg",
    };

    let dir = PathBuf::from(DOWNLOAD_DIR);
    fs::create_dir_all(&dir).await?;

    let filename = format!("receipt_{}.{}", receipt_id, extension);
    let dest = dir.join(filename);

    let bytes = client
        .get(&file_data.download_url)
        .send()
        .await?
        .bytes()
        .await?;
    fs::write(&dest, &bytes).await?;

    Ok(dest)
}

/// Prints a human-readable summary of a notification to the terminal.
pub fn print_notification(n: &Notification) {
    let body = &n.body;
    println!("---");
    println!("Type   : {}", body.type_webhook);

    if let Some(s) = &body.sender_data {
        println!(
            "From   : {} ({})",
            s.sender_name.as_deref().unwrap_or("unknown"),
            s.sender.as_deref().unwrap_or("unknown")
        );
    }

    if let Some(msg) = &body.message_data {
        println!("MsgType: {}", msg.type_message);

        if let Some(text) = msg.text() {
            println!("Text   : {}", text);
        }

        if let Some(file) = &msg.file_message_data {
            if let Some(caption) = &file.caption {
                if !caption.is_empty() {
                    println!("Caption: {}", caption);
                }
            }
        }
    }

    println!("---");
}
