mod extractor;
mod models;
mod parser;
mod whatsapp;

use dotenv::dotenv;
use std::env;
use tokio::time::{sleep, Duration};

/// How long to wait between polls when the Green API queue is empty.
const POLL_INTERVAL_SECS: u64 = 5;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let instance_id = env::var("GREEN_API_INSTANCE_ID")
        .expect("GREEN_API_INSTANCE_ID must be set in .env");

    let api_token = env::var("GREEN_API_TOKEN")
        .expect("GREEN_API_TOKEN must be set in .env");

    let client = reqwest::Client::new();

    println!(
        "Receipt engine started. Polling for messages every {} seconds...",
        POLL_INTERVAL_SECS
    );

    loop {
        match whatsapp::receive_notification(&client, &instance_id, &api_token).await {
            Ok(Some(notification)) => {
                whatsapp::print_notification(&notification);

                if let Some(msg) = &notification.body.message_data {
                    if let Some(file_data) = &msg.file_message_data {
                        let is_pdf =
                            file_data.mime_type.as_deref() == Some("application/pdf");

                        println!(
                            "{} detected — downloading...",
                            if is_pdf { "PDF" } else { "Image" }
                        );

                        match whatsapp::download_file(&client, file_data, notification.receipt_id)
                            .await
                        {
                            Ok(path) => {
                                println!("File saved to: {}", path.display());
                                println!("Running OCR...");

                                let ocr_result = if is_pdf {
                                    extractor::ocr_pdf(&path)
                                } else {
                                    extractor::ocr_image(&path)
                                };

                                match ocr_result {
                                    Ok(text) => {
                                        println!("OCR result:\n{}", text);
                                        let parsed = parser::parse_receipt(&text);
                                        parser::print_parsed(&parsed);

                                        let chat_id = notification
                                            .body
                                            .sender_data
                                            .as_ref()
                                            .and_then(|s| s.chat_id.as_deref());

                                        if let Some(chat_id) = chat_id {
                                            let reply = format!(
                                                "✅ Sender: {} | Bank: {} | Amount: {}",
                                                parsed.sender.as_deref().unwrap_or("unknown"),
                                                parsed.bank.as_deref().unwrap_or("unknown"),
                                                parsed.amount.as_deref().unwrap_or("unknown"),
                                            );
                                            match whatsapp::send_message(
                                                &client,
                                                &instance_id,
                                                &api_token,
                                                chat_id,
                                                &reply,
                                            )
                                            .await
                                            {
                                                Ok(_) => println!("Reply sent to {}", chat_id),
                                                Err(e) => {
                                                    eprintln!("Failed to send reply: {}", e)
                                                }
                                            }
                                        } else {
                                            eprintln!("No chat_id found — cannot send reply");
                                        }
                                    }
                                    Err(e) => eprintln!("OCR failed: {}", e),
                                }
                            }
                            Err(e) => eprintln!("Failed to download file: {}", e),
                        }
                    }
                }

                if let Err(e) = whatsapp::delete_notification(
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

        sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}
