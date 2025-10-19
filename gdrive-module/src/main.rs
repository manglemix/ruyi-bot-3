use google_drive3::{yup_oauth2, DriveHub, Error};

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider().install_default().unwrap();
    let secret: yup_oauth2::ApplicationSecret = Default::default();
    // Instantiate the authenticator. It will choose a suitable authentication flow for you,
    // unless you replace  `None` with the desired Flow.
    // Provide your own `AuthenticatorDelegate` to adjust the way it operates and get feedback about
    // what's going on. You probably want to bring in your own `TokenStorage` to persist tokens and
    // retrieve them from storage.
    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .build()
    .await
    .unwrap();

    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .unwrap()
                .https_or_http()
                .enable_http1()
                .build(),
        );
    let hub = DriveHub::new(client, auth);
    // You can configure optional parameters by calling the respective setters at will, and
    // execute the final call using `doit()`.
    // Values shown here are possibly random and not representative !
    let result = hub
        .files()
        .list()
        .team_drive_id("invidunt")
        .supports_team_drives(true)
        .supports_all_drives(true)
        .spaces("sed")
        .q("ut")
        .page_token("gubergren")
        .page_size(-16)
        .order_by("est")
        .include_team_drive_items(true)
        .include_permissions_for_view("ipsum")
        .include_labels("est")
        .include_items_from_all_drives(true)
        .drive_id("ea")
        .corpus("dolor")
        .corpora("Lorem")
        .doit()
        .await;

    match result {
        Err(e) => match e {
            // The Error enum provides details about what exactly happened.
            // You can also just use its `Debug`, `Display` or `Error` traits
            Error::HttpError(_)
            | Error::Io(_)
            | Error::MissingAPIKey
            | Error::MissingToken(_)
            | Error::Cancelled
            | Error::UploadSizeLimitExceeded(_, _)
            | Error::Failure(_)
            | Error::BadRequest(_)
            | Error::FieldClash(_)
            | Error::JsonDecodeError(_, _) => println!("{}", e),
        },
        Ok(res) => println!("Success: {:?}", res),
    }
}
