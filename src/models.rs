use serde::Deserialize;

/// A single notification returned by the Green API poll endpoint.
#[derive(Debug, Deserialize)]
pub struct Notification {
    #[serde(rename = "receiptId")]
    pub receipt_id: u64,
    pub body: NotificationBody,
}

/// The outer body of a Green API notification, containing the event type and message details.
#[derive(Debug, Deserialize)]
pub struct NotificationBody {
    #[serde(rename = "typeWebhook")]
    pub type_webhook: String,

    #[serde(rename = "senderData")]
    pub sender_data: Option<SenderData>,

    #[serde(rename = "messageData")]
    pub message_data: Option<MessageData>,
}

/// Information about who sent the message and which chat it belongs to.
#[derive(Debug, Deserialize)]
pub struct SenderData {
    #[serde(rename = "senderName")]
    pub sender_name: Option<String>,

    #[serde(rename = "sender")]
    pub sender: Option<String>,

    #[serde(rename = "chatId")]
    pub chat_id: Option<String>,
}

/// The message payload — structure varies depending on the message type.
#[derive(Debug, Deserialize)]
pub struct MessageData {
    #[serde(rename = "typeMessage")]
    pub type_message: String,

    #[serde(rename = "textMessageData")]
    pub text_message_data: Option<TextMessageData>,

    #[serde(rename = "extendedTextMessageData")]
    pub extended_text_message_data: Option<ExtendedTextMessageData>,

    /// Present for imageMessage and documentMessage types.
    #[serde(rename = "fileMessageData")]
    pub file_message_data: Option<FileMessageData>,
}

impl MessageData {
    /// Returns the plain text of the message, regardless of which sub-type carried it.
    pub fn text(&self) -> Option<&str> {
        if let Some(t) = &self.text_message_data {
            return Some(&t.text_message);
        }
        if let Some(e) = &self.extended_text_message_data {
            return Some(&e.text);
        }
        None
    }
}

/// Payload for a plain text message.
#[derive(Debug, Deserialize)]
pub struct TextMessageData {
    #[serde(rename = "textMessage")]
    pub text_message: String,
}

/// Payload for an extended text message (e.g. a message with a link preview).
#[derive(Debug, Deserialize)]
pub struct ExtendedTextMessageData {
    pub text: String,
}

/// Metadata for an incoming file attachment — either an image or a PDF.
#[derive(Debug, Deserialize)]
pub struct FileMessageData {
    /// Direct URL to download the file bytes from.
    #[serde(rename = "downloadUrl")]
    pub download_url: String,

    /// MIME type of the file, e.g. "image/jpeg" or "application/pdf".
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,

    /// Optional caption the sender typed alongside the file.
    pub caption: Option<String>,
}

/// The structured data extracted from a receipt after OCR and parsing.
#[derive(Debug)]
pub struct ParsedReceipt {
    pub sender: Option<String>,
    pub bank: Option<String>,
    pub amount: Option<String>,
}
