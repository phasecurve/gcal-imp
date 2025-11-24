pub fn paste_from_clipboard() -> Result<String, String> {
    use std::process::{Command, Stdio};

    let output = Command::new("wl-paste")
        .stdout(Stdio::piped())
        .output()
        .or_else(|_| Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .stdout(Stdio::piped())
            .output())
        .or_else(|_| Command::new("xsel")
            .args(["--clipboard", "--output"])
            .stdout(Stdio::piped())
            .output());

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8(output.stdout)
                .map_err(|e| format!("Invalid UTF-8 in clipboard: {}", e))
        }
        Ok(output) => Err(format!("Clipboard command failed: {:?}", output.status)),
        Err(e) => Err(format!("No clipboard tool found (wl-paste/xclip/xsel): {}", e)),
    }
}

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};
    use std::io::Write;

    let result = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .or_else(|_| Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(Stdio::piped())
            .spawn())
        .or_else(|_| Command::new("xsel")
            .args(["--clipboard", "--input"])
            .stdin(Stdio::piped())
            .spawn());

    match result {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())
                    .map_err(|e| format!("Failed to write to clipboard: {}", e))?;
            }
            child.wait().map_err(|e| format!("Clipboard command failed: {}", e))?;
            tracing::info!("Copied {} bytes to clipboard", text.len());
            Ok(())
        }
        Err(e) => {
            let err = format!("No clipboard tool found (wl-copy/xclip/xsel): {}", e);
            tracing::error!("{}", err);
            Err(err)
        }
    }
}
