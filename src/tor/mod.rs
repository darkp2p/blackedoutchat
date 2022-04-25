pub mod control;
pub mod onion;

use std::{
    env,
    fs::{self, OpenOptions},
    io::Error,
    os::unix::{fs::PermissionsExt, net::UnixStream},
    process::{exit, Command, ExitStatus},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::{net::UnixStream as AsyncUnixStream, time::sleep};

use crate::config::Config;
use crate::error::{BlackedoutError, Result};

use self::control::execute_control;

pub fn spawn_tor(config: &Config) -> Result<UnixStream> {
    create_dirs()?;
    create_tmprc(config)?;

    let _pid = spawn_process().and_then(|(pid, mut handle)| {
        ctrlc::set_handler(move || {
            handle.take().map(|handle| exit_handler((pid, handle)));
            exit(0);
        })
        .map(|_| pid)
        .map_err(|e| match e {
            ctrlc::Error::System(n) => n.into(),
            n => panic!("Failed to set exit handler: {:?}", n),
        })
    })?;

    thread::sleep(Duration::from_secs(5));

    control::connect_to_control()
}

pub async fn handle_tor(control: UnixStream) -> Result<()> {
    let mut control = control
        .set_nonblocking(true)
        .and_then(|_| AsyncUnixStream::from_std(control))?;

    loop {
        execute_control(&mut control, "GETINFO status/circuit-established\r\n")
            .await
            .map_err(|e| BlackedoutError::TorShutdown(e.into()))?;

        sleep(Duration::from_secs(2)).await;
    }
}

fn create_dirs() -> Result<()> {
    fs::create_dir_all("data")?;
    fs::metadata("data").and_then(|metadata| {
        let mut perm = metadata.permissions();
        perm.set_mode(0o700);

        fs::set_permissions("data", perm)
    })?;

    fs::create_dir_all("data/tor")?;
    fs::create_dir_all("data/incoming")?;
    fs::create_dir_all("data/logs")?;
    fs::create_dir_all("data/torrc.d")?;

    Ok(())
}

fn create_tmprc(config: &Config) -> Result<()> {
    let mut tmprc = String::new();
    let current = env::current_dir()?;

    tmprc.push_str("DataDirectory data/tor\n");
    tmprc.push_str("ControlSocket unix:");
    tmprc.push_str(&current.join("data").join("control.sock").to_string_lossy());
    tmprc.push_str("\n");
    tmprc.push_str("SOCKSPort unix:");
    tmprc.push_str(&current.join("data").join("tor.sock").to_string_lossy());
    tmprc.push_str("\n");

    config.addresses.addresses.iter().for_each(|x| {
        tmprc.push_str("HiddenServiceDir data/");
        tmprc.push_str(&x.name);
        tmprc.push_str("\n");
        tmprc.push_str("HiddenServicePort 21761 unix:");
        tmprc.push_str(
            &current
                .join("data")
                .join(&x.name)
                .join("incoming")
                .with_extension("sock")
                .to_string_lossy(),
        );
        tmprc.push_str("\n");
        tmprc.push_str("HiddenServiceVersion 3\n");
    });

    Ok(fs::write("data/tmprc", tmprc)?)
}

fn spawn_process() -> Result<(u32, Option<JoinHandle<std::io::Result<ExitStatus>>>)> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!(
            "data/logs/{:016}.log",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ))
        .and_then(|log| {
            Command::new("tor")
                .arg("-f")
                .arg("data/tmprc")
                .stdout(log)
                .spawn()
        })
        .map(|x| (x.id(), x))
        .and_then(|(pid, mut child)| Ok((pid, thread::spawn(move || child.wait()))))
        .map(|(a, b)| (a, Some(b)))
        .map_err(From::from)
}

fn exit_handler((pid, handle): (u32, JoinHandle<std::io::Result<ExitStatus>>)) {
    // TODO: Log Tor about to stop

    if handle.is_finished() {
        // TODO: Log Tor already stopped
        return;
    }

    if let Err(_e) = unsafe { libc::kill(pid.try_into().unwrap(), libc::SIGTERM) }
        .ne(&-1)
        .then(|| ())
        .ok_or(Error::from_raw_os_error(unsafe {
            *libc::__errno_location()
        }))
    {
        // TODO: Log error
    }

    if let Err(_e) = handle.join() {
        // TODO: Log error
    }
}
