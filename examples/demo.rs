use std::time::Duration;

use anyhow::Result;
use bstr::BStr;
use tokio::io::{AsyncRead, AsyncReadExt, self};
use muecke::types::QoS::QoS0;
use muecke::types::{Connect, ControlPacket, Publish, Str};

#[tokio::main]
async fn main() -> Result<()> {
    let host = "localhost";
    let port = 8883;

    let conn = muecke::tls::connect(host, port).await?;
    let (reader, mut writer) = io::split(conn);

    tokio::spawn(async move {
        if let Err(err) = print_received_data(reader).await {
            eprintln!("Failed to receive data: {err}");
        }
    });

    Connect {
        clean_start: true,
        will: None,
        username: None,
        password: None,
        keep_alive: 60,
        client_id: Str::from_static("muecke"),
    }
    .write_to(&mut writer)
    .await?;

    Publish {
        dup: false,
        qos: QoS0,
        identifier: 0,
        retain: false,
        topic: Str::from_static("rust/muecke/demo"),
        message: b"this is a demo!",
    }
    .write_to(&mut writer)
    .await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}

async fn print_received_data<R>(mut reader: R) -> Result<()>
where
    R: AsyncRead + Unpin + Send + Sync + 'static,
{
    let mut buf = Vec::new();

    loop {
        let n_read = reader.read_buf(&mut buf).await?;

        if n_read == 0 {
            return anyhow::Ok(());
        }

        println!("Recv: {}", BStr::new(&buf));

        buf.clear();
    }
}
