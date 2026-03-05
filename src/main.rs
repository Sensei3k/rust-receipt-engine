// Phase 1 — Poll Green API for new WhatsApp messages and print them to the terminal.
//
// Green API works on a "receive & delete" model:
//   1. Call /receiveNotification  → get the oldest queued notification (if any)
//   2. Call /deleteNotification   → acknowledge it so it doesn't come back
// We repeat this loop every 5 seconds.

use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use tokio::time::{sleep, Duration};

// --------------------------------------------------------------------------
// Data structures — modelled on the actual JSON Green API returns
// --------------------------------------------------------------------------

// Top-level envelope. receiptId is needed to delete the notification.
#[derive(Debug, Deserialize)]
struct Notification {
    #[serde(rename = "receiptId")]
    receipt_id: u64,
    body: NotificationBody,
}

// The body tells us who sent the message and what kind of event it is.
#[derive(Debug, Deserialize)]
struct NotificationBody {
    #[serde(rename = "typeWebhook")]
    type_webhook: String,

    #[serde(rename = "senderData")]
    sender_data: Option<SenderData>,

    #[serde(rename = "messageData")]
    message_data: Option<MessageData>,
}

// Sender information — name and phone number.
#[derive(Debug, Deserialize)]
struct SenderData {
    #[serde(rename = "senderName")]
    sender_name: Option<String>,

    // Phone number in WhatsApp format, e.g. "447427749650@c.us"
    #[serde(rename = "sender")]
    sender: Option<String>,
}

// Message content wrapper.
// Green API uses different sub-objects depending on message type:
//   "textMessage"         → textMessageData.textMessage
//   "extendedTextMessage" → extendedTextMessageData.text  (links, forwarded msgs, etc.)
//   "imageMessage"        → imageMessageData              (Phase 2)
#[derive(Debug, Deserialize)]
struct MessageData {
    #[serde(rename = "typeMessage")]
    type_message: String,

    // Plain text messages
    #[serde(rename = "textMessageData")]
    text_message_data: Option<TextMessageData>,

    // Extended text (links, forwarded messages — the common case for plain texts too)
    #[serde(rename = "extendedTextMessageData")]
    extended_text_message_data: Option<ExtendedTextMessageData>,
}

// Used when typeMessage == "textMessage"
#[derive(Debug, Deserialize)]
struct TextMessageData {
    #[serde(rename = "textMessage")]
    text_message: String,
}

// Used when typeMessage == "extendedTextMessage"
#[derive(Debug, Deserialize)]
struct ExtendedTextMessageData {
    // The actual message text lives here (not in a nested "textMessage" field)
    text: String,
}

impl MessageData {
    /// Return the message text regardless of which sub-type carried it.
    fn text(&self) -> Option<&str> {
        if let Some(t) = &self.text_message_data {
            return Some(&t.text_message);
        }
        if let Some(e) = &self.extended_text_message_data {
            return Some(&e.text);
        }
        None
    }
}

// --------------------------------------------------------------------------
// API helpers
// --------------------------------------------------------------------------

/// Fetch the oldest pending notification from Green API.
/// Returns None when the queue is empty (Green API returns the literal "null").
async fn receive_notification(
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

/// Acknowledge a notification so Green API removes it from the queue.
/// Must be called after every successful receive.
async fn delete_notification(
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

/// Print a notification to the terminal in a readable format.
fn print_notification(n: &Notification) {
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

        match msg.text() {
            Some(text) => println!("Text   : {}", text),
            None => println!("Text   : (no text — image or other attachment)"),
        }
    }

    println!("---");
}

// --------------------------------------------------------------------------
// Main
// --------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    dotenv().ok();

    let instance_id = env::var("GREEN_API_INSTANCE_ID")
        .expect("GREEN_API_INSTANCE_ID must be set in .env");

    let api_token = env::var("GREEN_API_TOKEN")
        .expect("GREEN_API_TOKEN must be set in .env");

    let client = Client::new();

    println!("Receipt engine started. Polling for messages every 5 seconds...");

    loop {
        match receive_notification(&client, &instance_id, &api_token).await {
            Ok(Some(notification)) => {
                print_notification(&notification);

                if let Err(e) = delete_notification(
                    &client,
                    &instance_id,
                    &api_token,
                    notification.receipt_id,
                )
                .await
                {
                    eprintln!("Warning: failed to delete notification: {}", e);
                }
            }

            Ok(None) => {
                println!("[tick] No new messages.");
            }

            Err(e) => {
                eprintln!("Error polling Green API: {}", e);
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}
