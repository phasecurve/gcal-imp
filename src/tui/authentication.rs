use gcal_imp::storage::config::Config;
use gcal_imp::sync::google_auth::GoogleAuthenticator;

pub async fn check_or_setup_auth() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load_or_create()?;

    if config.google.client_id.is_empty() || config.google.client_secret.is_empty() {
        println!("Configuration incomplete. Please edit the config file at:");
        println!("{}", Config::config_path().display());
        println!("\nYou need to set:");
        println!("  - google.client_id: Your Google OAuth2 client ID");
        println!("  - google.client_secret: Your Google OAuth2 client secret");
        println!("\nGet these from: https://console.cloud.google.com/apis/credentials");
        return Err("Missing Google OAuth credentials in config".into());
    }

    let mut auth = GoogleAuthenticator::new(config);

    match auth.get_valid_token().await {
        Ok(_) => {
            println!("Authentication successful! Starting calendar...\n");
            Ok(())
        }
        Err(_) => {
            println!("No valid authentication found. Setting up Google Calendar access...\n");
            auth.print_auth_instructions();

            println!("Enter the authorization code: ");
            let mut code = String::new();
            std::io::stdin().read_line(&mut code)?;
            let code = code.trim();

            auth.exchange_code_for_token(code).await?;
            println!("\nAuthentication successful! You can now use gcal-imp.\n");

            Ok(())
        }
    }
}
