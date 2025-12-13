use crate::parsing::ParsedContent;
use anyhow::{Result, anyhow};
use image::ImageFormat;
use pdfium_render::prelude::*;
use std::io::{Cursor, Write};
use tempfile::NamedTempFile;

pub fn parse_pdf(bytes: &[u8]) -> Result<ParsedContent> {
    // Library is copied to target dir by build.rs, bind to system library path
    let bindings = Pdfium::bind_to_system_library()
        .map_err(|e| anyhow!("Failed to bind to Pdfium library: {:?}", e))?;

    let pdfium = Pdfium::new(bindings);

    // Write to temp file
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(bytes)?;
    let document = pdfium
        .load_pdf_from_file(temp_file.path(), None)
        .map_err(|e| anyhow!("Failed to load PDF: {}", e))?;

    // STEP 1: Try text extraction
    let mut full_text = String::new();
    for page in document.pages().iter() {
        if let Ok(text) = page.text() {
            full_text.push_str(&text.all());
            full_text.push('\n');
        }
    }

    if full_text.trim().len() > 50 {
        return Ok(ParsedContent::Text(full_text));
    }

    // STEP 2: Try embedded image extraction
    let mut extracted_images = Vec::new();
    for page in document.pages().iter() {
        for object in page.objects().iter() {
            if let PdfPageObject::Image(ref image_object) = object
                && let Ok(data) = image_object.get_raw_image_data()
                && image::load_from_memory(&data).is_ok()
            {
                extracted_images.push(data);
            }
        }
    }

    if !extracted_images.is_empty() {
        return Ok(ParsedContent::ScannedImages(extracted_images));
    }

    // STEP 3: Rasterize pages as fallback
    let mut rasterized_images = Vec::new();
    for page in document.pages().iter() {
        let render_config = PdfRenderConfig::new()
            .set_target_width(2048)
            .set_maximum_height(2048)
            .rotate_if_landscape(PdfPageRenderRotation::Degrees90, true);

        if let Ok(bitmap) = page.render_with_config(&render_config) {
            let dynamic_image = bitmap.as_image();
            let mut bytes: Vec<u8> = Vec::new();
            if dynamic_image
                .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)
                .is_ok()
            {
                rasterized_images.push(bytes);
            }
        }
    }

    if !rasterized_images.is_empty() {
        return Ok(ParsedContent::ScannedImages(rasterized_images));
    }

    Err(anyhow!("PDF appears empty"))
}
