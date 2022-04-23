use std::{future::Future, os::unix::net::UnixStream as StdUnixStream, pin::Pin, time::Duration};

use tokio::{net::UnixStream, time::sleep};

use crate::error::{BlackedoutError, Result};
use crate::handler::Handler;

use super::control::execute_control;

pub struct TorHandler {
    control: UnixStream,
}

impl TorHandler {
    pub async fn new((_pid, control): (u32, StdUnixStream)) -> Result<Self> {
        control.set_nonblocking(true)?;

        Ok(TorHandler {
            control: UnixStream::from_std(control)?,
        })
    }
}

impl Handler for TorHandler {
    fn listen(mut self) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        Box::pin(async move {
            loop {
                execute_control(&mut self.control, "GETINFO status/circuit-established\r\n")
                    .await
                    .map_err(|e| BlackedoutError::TorShutdown(e.into()))?;

                sleep(Duration::from_secs(2)).await;
            }
        })
    }
}
