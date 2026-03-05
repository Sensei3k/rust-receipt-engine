use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs Tesseract OCR on the image file at the given path and returns the extracted text.
pub fn ocr_image(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let path_str = path
        .to_str()
        .ok_or("file path contains invalid UTF-8")?;
    let text = tesseract::ocr(path_str, "eng")?;
    Ok(text)
}

/// Converts each page of a PDF to JPEG using pdftoppm, then runs OCR on each page.
/// Returns the combined text from all pages concatenated together.
pub fn ocr_pdf(pdf_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let dir = pdf_path
        .parent()
        .ok_or("PDF path has no parent directory")?;

    let stem = pdf_path
        .file_stem()
        .ok_or("PDF path has no file stem")?
        .to_str()
        .ok_or("file stem contains invalid UTF-8")?;

    let prefix = dir.join(stem);
    let prefix_str = prefix
        .to_str()
        .ok_or("prefix path contains invalid UTF-8")?;

    let pdf_path_str = pdf_path
        .to_str()
        .ok_or("PDF path contains invalid UTF-8")?;

    // Convert all PDF pages to JPEG: produces <prefix>-1.jpg, <prefix>-2.jpg, etc.
    let status = Command::new("pdftoppm")
        .args(["-jpeg", pdf_path_str, prefix_str])
        .status()?;

    if !status.success() {
        return Err("pdftoppm failed to convert PDF".into());
    }

    let mut all_text = String::new();
    let mut page = 1u32;

    loop {
        // pdftoppm zero-pads page numbers differently depending on total page count.
        // Check padding widths 1, 2, and 3 to find whichever file was generated.
        let candidates: Vec<PathBuf> = (1..=3)
            .map(|width| dir.join(format!("{}-{:0width$}.jpg", stem, page, width = width)))
            .collect();

        match candidates.iter().find(|p| p.exists()) {
            None => break,
            Some(page_path) => {
                println!("  OCR page {}...", page);
                match ocr_image(page_path) {
                    Ok(text) => all_text.push_str(&text),
                    Err(e) => eprintln!("  OCR failed on page {}: {}", page, e),
                }
                page += 1;
            }
        }
    }

    Ok(all_text)
}
