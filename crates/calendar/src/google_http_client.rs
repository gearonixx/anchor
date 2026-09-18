use std::sync::LazyLock;
use std::time::Duration;

use reqwest::Client;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) static GOOGLE_HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .unwrap_or_default()
});
