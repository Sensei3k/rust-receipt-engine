use crate::models::ParsedReceipt;
use regex::Regex;

/// Extracts sender name, bank name, and amount from raw OCR text.
/// Returns a ParsedReceipt with whichever fields could be identified.
pub fn parse_receipt(text: &str) -> ParsedReceipt {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    // --- Amount ---
    // Tesseract often renders ₦ as #, so both are matched and # is normalised to ₦.
    // Handles patterns like: #97,800.00  ₦97,800.00  NGN 97,800.00
    let amount_re = Regex::new(r"(?:[#₦]|NGN\s*)[\d,]+(?:\.\d{1,2})?").unwrap();
    let amount = amount_re
        .find(text)
        .map(|m| m.as_str().trim().replace('#', "₦"));

    // --- Sender + Bank ---
    // Primary pattern (OPay / Moniepoint style):
    //   "Sender Details  FULL NAME"  ← name on same line as label
    //   "BankName | account"         ← bank on the very next line
    let sender_label_re = Regex::new(r"(?i)sender\s+details?\s+(.+)").unwrap();
    let fallback_sender_re = Regex::new(
        r"(?i)(?:sender(?:\s+name)?|from|originator)\s*[:\-]?\s*([A-Za-z][A-Za-z\s]{2,40})",
    )
    .unwrap();

    let mut sender: Option<String> = None;
    let mut bank: Option<String> = None;

    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = sender_label_re.captures(line) {
            sender = caps.get(1).map(|m| m.as_str().trim().to_string());

            // The bank name is on the next line, before any "|" separator.
            if let Some(next_line) = lines.get(i + 1) {
                let bank_name = next_line.split('|').next().unwrap_or(next_line).trim();
                if !bank_name.is_empty() {
                    bank = Some(bank_name.to_string());
                }
            }
            break;
        }
    }

    // Fallback: try a simpler label pattern if the primary one didn't match.
    if sender.is_none() {
        sender = fallback_sender_re
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string());
    }

    // Fallback: scan for any known Nigerian bank or fintech name in the text.
    if bank.is_none() {
        let known_banks_re = Regex::new(
            r"(?i)\b(gtbank|access bank|zenith bank|first bank|uba|opay|kuda|palmpay|moniepoint|monie point|sterling|polaris|fidelity|union bank|wema|stanbic|ecobank|providus)\b",
        )
        .unwrap();
        bank = known_banks_re
            .find(text)
            .map(|m| m.as_str().trim().to_string());
    }

    ParsedReceipt { sender, bank, amount }
}

/// Prints the fields of a ParsedReceipt to the terminal in a readable format.
pub fn print_parsed(parsed: &ParsedReceipt) {
    println!("--- Parsed Receipt ---");
    println!("Sender : {}", parsed.sender.as_deref().unwrap_or("not found"));
    println!("Bank   : {}", parsed.bank.as_deref().unwrap_or("not found"));
    println!("Amount : {}", parsed.amount.as_deref().unwrap_or("not found"));
    println!("----------------------");
}
