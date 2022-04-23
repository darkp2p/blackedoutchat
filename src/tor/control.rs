use std::{
    io::{ErrorKind, Read, Write},
    os::unix::net::UnixStream,
    thread::sleep,
    time::Duration,
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::Result;

pub fn connect_to_control() -> Result<UnixStream> {
    let mut control = loop {
        match UnixStream::connect("data/control.sock") {
            Ok(n) => break n,
            Err(e) => match e.kind() {
                ErrorKind::Interrupted => {}
                ErrorKind::PermissionDenied | ErrorKind::NotFound => return Err(e.into()),
                _ => sleep(Duration::from_millis(5)),
            },
        }
    };

    _execute_control(&mut control, "authenticate \"\"\r\n")?
        .contains("250 OK")
        .then(|| ())
        .unwrap(); // TODO: Return proper error type

    loop {
        if _execute_control(&mut control, "GETINFO status/circuit-established\r\n")?
            .contains("250-status/circuit-established=1")
        {
            break Ok(control);
        }

        sleep(Duration::from_secs(1));
    }
}

fn _execute_control(control: &mut UnixStream, instruction: &str) -> Result<String> {
    control.write_all(instruction.as_bytes())?;

    let mut buf = [0u8; 512];
    let nbytes = control.read(&mut buf)?;

    Ok(String::from_utf8_lossy(&buf[..nbytes]).to_string())
}

pub async fn execute_control(
    control: &mut tokio::net::UnixStream,
    instruction: &str,
) -> Result<String> {
    control.write_all(instruction.as_bytes()).await?;

    let mut buf = [0u8; 512];
    let nbytes = control.read(&mut buf).await?;

    Ok(String::from_utf8_lossy(&buf[..nbytes]).to_string())
}
