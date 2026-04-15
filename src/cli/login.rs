use crate::client;
use crate::client::diagv1;
use crate::error::TldError;
use crate::output;
use crate::workspace;
use clap::Args;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Args, Debug, Clone)]
pub struct LoginArgs {
    /// Server URL (default: $TLD_SERVER_URL or https://tldiagram.com)
    #[arg(long)]
    pub server: Option<String>,
    /// Print the URL instead of opening a browser
    #[arg(long = "no-browser", default_value = "false")]
    pub no_browser: bool,
}

#[expect(clippy::print_stdout, clippy::disallowed_methods)]
pub async fn exec(args: LoginArgs, _wdir: String) -> Result<(), TldError> {
    let server_url = args
        .server
        .or_else(|| std::env::var("TLD_SERVER_URL").ok())
        .unwrap_or_else(|| "https://tldiagram.com".to_string());

    let mut device_client = client::new_device_client(&server_url).await?;

    let req = tonic::Request::new(diagv1::DeviceAuthorizeRequest {
        client_name: "tld CLI (Rust)".to_string(),
    });

    let auth = device_client.authorize(req).await?.into_inner();

    println!("\nOpen the following URL to log in:\n\n  {uri}\n\n", uri = auth.verification_uri_complete);
    println!(
        "Or navigate to {uri} and enter the code:\n\n  {code}\n\n",
        uri = auth.verification_uri, code = auth.user_code
    );
    println!("Waiting for authorisation… (press Ctrl+C to cancel)");

    if !args.no_browser {
        let _ = opener::open(&auth.verification_uri_complete);
    }

    let interval = if auth.interval > 0 {
        Duration::from_secs(auth.interval.cast_unsigned().into())
    } else {
        Duration::from_secs(5)
    };

    // Poll for token
    let (api_key, org_id) = loop {
        sleep(interval).await;
        let poll_req = tonic::Request::new(diagv1::DevicePollTokenRequest {
            device_code: auth.device_code.clone(),
        });

        match device_client.poll_token(poll_req).await {
            Ok(res) => {
                let token = res.into_inner();
                if !token.error.is_empty() {
                    match token.error.as_str() {
                        "authorization_pending" => continue,
                        "access_denied" => {
                            return Err(TldError::Generic(
                                "Authorisation denied by user".to_string(),
                            ));
                        }
                        "expired_token" => {
                            return Err(TldError::Generic(
                                "Device code expired - run 'tld login' again".to_string(),
                            ));
                        }
                        _ => {
                            return Err(TldError::Generic(format!(
                                "Unexpected error from server: {}",
                                token.error
                            )));
                        }
                    }
                }
                break (token.api_key, token.org_id);
            }
            Err(status) => {
                // Keep polling on transient errors
                if status.code() == tonic::Code::Unavailable {
                    continue;
                }
                return Err(status.into());
            }
        }
    };

    workspace::write_config(&server_url, &api_key, &org_id)?;
    output::print_ok("Authorised! Config written.");

    Ok(())
}
