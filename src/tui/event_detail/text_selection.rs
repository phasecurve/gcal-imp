use arboard;

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            match clipboard.set_text(text) {
                Ok(_) => {
                    tracing::info!("Successfully copied to clipboard (cross-platform)");
                    Ok(())
                }
                Err(e) => {
                    let err = format!("Failed to set clipboard text: {}", e);
                    tracing::error!("{}", err);
                    Err(err)
                }
            }
        }
        Err(e) => {
            let err = format!("Failed to access clipboard: {}", e);
            tracing::error!("{}", err);
            Err(err)
        }
    }
}
