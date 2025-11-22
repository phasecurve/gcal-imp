use std::io;

mod cli;
use cli::{CliMode, parse_cli_mode, run_agenda_mode};
mod tui;
use tui::{run_tui, check_or_setup_auth};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    setup_logging();

    let cli_mode = match parse_cli_mode() {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("Error: {}", err);
            println!("Usage: gcal-imp [--agenda [YYYY/MM/DD]]");
            return Ok(());
        }
    };

    if let CliMode::AgendaDate(_) = cli_mode {
        if let Err(e) = check_or_setup_auth().await {
            eprintln!("Authentication error: {}", e);
            tracing::error!("Authentication failed: {}", e);
            return Ok(());
        }
        let date = match cli_mode {
            CliMode::AgendaDate(d) => d,
            _ => unreachable!(),
        };
        return run_agenda_mode(date).await;
    }

    if let Err(e) = check_or_setup_auth().await {
        eprintln!("Authentication error: {}", e);
        tracing::error!("Authentication failed: {}", e);
        return Ok(());
    }

    run_tui().await
}

fn setup_logging() {
    let log_dir = dirs::config_dir()
        .map(|d| d.join("gcal-imp"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::daily(log_dir, "gcal-imp.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .init();

    std::mem::forget(_guard);

    tracing::info!("gcal-imp started");
}
