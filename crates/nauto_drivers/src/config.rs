use once_cell::sync::Lazy;
use std::time::Duration;

const DEFAULT_SSH_TIMEOUT_SECS: u64 = 30;
const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 15;
const DEFAULT_HTTP_RETRIES: usize = 2;

static SSH_TIMEOUT: Lazy<Duration> = Lazy::new(|| {
    env_duration(
        "NAUTO_SSH_TIMEOUT_SECS",
        Duration::from_secs(DEFAULT_SSH_TIMEOUT_SECS),
    )
});

static HTTP_TIMEOUT: Lazy<Duration> = Lazy::new(|| {
    env_duration(
        "NAUTO_HTTP_TIMEOUT_SECS",
        Duration::from_secs(DEFAULT_HTTP_TIMEOUT_SECS),
    )
});

static HTTP_RETRIES: Lazy<usize> = Lazy::new(|| {
    std::env::var("NAUTO_HTTP_RETRIES")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_HTTP_RETRIES)
});

pub fn ssh_command_timeout() -> Duration {
    *SSH_TIMEOUT
}

pub fn http_timeout() -> Duration {
    *HTTP_TIMEOUT
}

pub fn http_retry_limit() -> usize {
    *HTTP_RETRIES
}

fn env_duration(var: &str, default: Duration) -> Duration {
    std::env::var(var)
        .ok()
        .and_then(|value| value.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(default)
}
