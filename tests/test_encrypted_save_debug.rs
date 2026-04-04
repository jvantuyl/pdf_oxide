//! Debug test for encrypted PDF save functionality
//!
//! Run with: cargo test test_save_encrypted_pdf_debug -- --nocapture

use pdf_oxide::document::PdfDocument;
use pdf_oxide::editor::{DocumentEditor, EditableDocument};
use std::fs;
use tempfile::tempdir;

fn extract_object_ids(data: &[u8]) -> Vec<i32> {
    // Extract object IDs from "N N obj" patterns
    let mut ids = Vec::new();
    let s = String::from_utf8_lossy(data);
    for line in s.lines() {
        let line = line.trim();
        if line.ends_with("obj") || line.ends_with("obj ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[2] == "obj" {
                if let Ok(id) = parts[0].parse::<i32>() {
                    ids.push(id);
                }
            }
        }
    }
    ids.sort();
    ids
}

#[test]
fn test_save_encrypted_pdf_debug() {
    // Load the encrypted tax return PDF
    let input_path = std::path::Path::new("tests/fixtures/encrypted_tax_return.pdf");
    if !input_path.exists() {
        eprintln!("Skipping test - fixture not found at {:?}", input_path);
        return;
    }

    let input_data = fs::read(&input_path).unwrap();
    let input_size = input_data.len() as u64;
    let input_obj_ids = extract_object_ids(&input_data);
    eprintln!("[test] Input PDF: {} bytes, {} 'N N obj' patterns", input_size, input_obj_ids.len());

    // Open the PDF and authenticate with password
    let password = "222111111newyork10005";
    let doc = PdfDocument::open(&input_path).expect("Failed to open PDF");
    
    // Check xref table size vs object patterns
    let xref_ids = doc.get_all_object_ids();
    eprintln!("[test] Xref table has {} entries", xref_ids.len());
    
    // Authenticate with password
    let auth_result = doc.authenticate(password.as_bytes());
    eprintln!("[test] Authentication result: {:?}", auth_result);
    if !auth_result.unwrap_or(false) {
        panic!("Failed to authenticate with password");
    }
    
    let mut editor = DocumentEditor::from_document(doc).expect("Failed to create editor");
    eprintln!("[test] Successfully opened encrypted PDF");

    // Save without making any changes
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("output.pdf");
    
    eprintln!("[test] Saving to {:?}", output_path);
    editor.save(&output_path).unwrap();

    let output_data = fs::read(&output_path).unwrap();
    let output_size = output_data.len() as u64;
    let output_obj_ids = extract_object_ids(&output_data);
    
    eprintln!("[test] Output PDF: {} bytes, {} objects", output_size, output_obj_ids.len());
    eprintln!("[test] Size: {}% of original", output_size * 100 / input_size);

    // Find missing objects
    let max_input_id = input_obj_ids.iter().max().copied().unwrap_or(0);
    let input_set: std::collections::HashSet<i32> = input_obj_ids.into_iter().collect();
    let output_set: std::collections::HashSet<i32> = output_obj_ids.iter().copied().collect();
    let missing: Vec<i32> = input_set.difference(&output_set).copied().collect();
    let output_count = output_obj_ids.len();
    
    eprintln!("[test] Max input object ID: {}", max_input_id);
    
    if !missing.is_empty() {
        // Check if missing objects are newly allocated IDs (not in original)
        let newly_allocated: Vec<i32> = missing.iter().filter(|&&id| id > max_input_id).copied().collect();
        if !newly_allocated.is_empty() {
            eprintln!("[test] Missing {} objects are newly allocated (not in original): {:?}", newly_allocated.len(), newly_allocated);
        } else {
            eprintln!("[test] Missing {} objects from original: {:?}", missing.len(), missing);
        }
    }

    // Check that we preserved a reasonable amount of data
    assert!(output_size > input_size / 2, 
        "Output too small: {} bytes vs {} bytes original", output_size, input_size);
    
    // We should have at least 90% of objects
    assert!(output_count >= input_set.len() * 90 / 100,
        "Too few objects: {} vs {} original", output_count, input_set.len());
    
    // Verify the output PDF can actually be opened
    let mut output_doc = PdfDocument::open(&output_path).expect("Output PDF should be readable");
    let output_page_count = output_doc.page_count().expect("Should have pages");
    eprintln!("[test] Output PDF has {} pages", output_page_count);
    assert!(output_page_count > 0, "Output PDF should have at least one page");
}
