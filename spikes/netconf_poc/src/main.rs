use anyhow::Result;

#[cfg(feature = "real-ssh")]
use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};
#[cfg(feature = "real-ssh")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}

#[cfg(feature = "real-ssh")]
async fn run() -> Result<()> {
    let host = std::env::var("NETCONF_HOST").context("set NETCONF_HOST")?;
    let username = std::env::var("NETCONF_USER").context("set NETCONF_USER")?;
    let password = std::env::var("NETCONF_PASSWORD").context("set NETCONF_PASSWORD")?;

    let client = Client::connect(
        (host.as_str(), 830),
        &username,
        AuthMethod::with_password(&password),
        ServerCheckMethod::NoCheck,
    )
    .await
    .context("ssh negotiation failed")?;

    // Request a NETCONF subsystem channel (RFC 6242).
    let channel = client.get_channel().await.context("open channel failed")?;
    channel
        .request_subsystem(true, "netconf")
        .await
        .context("netconf subsystem denied")?;

    // Client hello with basic capabilities.
    let hello = r#"<?xml version="1.0" encoding="UTF-8"?>
<hello xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">
  <capabilities>
    <capability>urn:ietf:params:netconf:base:1.0</capability>
  </capabilities>
</hello>]]>]]>"#;

    let mut stream = channel.into_stream();

    stream
        .write_all(hello.as_bytes())
        .await
        .context("failed to send hello")?;

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .await
        .context("failed to read server hello")?;

    println!(
        "Server hello:\n{}",
        String::from_utf8_lossy(&buf).replace("]]>]]>", "")
    );

    Ok(())
}

#[cfg(not(feature = "real-ssh"))]
async fn run() -> Result<()> {
    println!("[stub] NETCONF demo requires --features real-ssh.");
    println!("Set NETCONF_HOST/USER/PASSWORD env variables and re-run with the feature to talk to a controller.");
    Ok(())
}
