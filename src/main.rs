use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

use color_eyre::eyre::{self, Context};
use env_logger::Env;
use serde::Deserialize;
use serde_json::json;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    log::info!("running server at tcp://0.0.0.0:8080");

    TcpListener::bind("0.0.0.0:8080")?
        .incoming()
        .filter_map(|stream| stream.ok())
        .for_each(|stream| {
            if let Err(e) = handle_connection(stream) {
                log::error!("{:?}", e);
            }
        });

    Ok(())
}

fn handle_connection(stream: TcpStream) -> eyre::Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut message = String::new();

    reader.read_line(&mut message)?;
    log::debug!("{}", message.trim());

    let message = serde_json::from_str(&message).context("failed to deserialize message")?;
    handle_message(message, stream);

    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "command")]
enum Message {
    Suspend {
        #[serde(default)]
        options: SuspendOptions,
    },
}

#[derive(Default, Deserialize)]
struct SuspendOptions {
    hibernate: bool,
}

fn handle_message(message: Message, stream: TcpStream) {
    match message {
        Message::Suspend { options } => handle_suspend(options, stream),
    }
}

fn handle_suspend(options: SuspendOptions, stream: TcpStream) {
    fn send_res(mut stream: TcpStream) -> eyre::Result<()> {
        let res = serde_json::to_vec(&json!({
            "success": true
        }))?;

        stream.write_all(&res)?;
        stream.flush()?;

        Ok(())
    }

    if let Err(e) = send_res(stream) {
        log::error!("{}", e);
        return;
    }

    log::info!("suspending system (hibernate: {})", options.hibernate);

    unsafe { SetSuspendState(options.hibernate, false, false) };
}

#[link(name = "powrprof")]
extern "C" {
    fn SetSuspendState(hibernate: bool, force: bool, wakeup_events_disabled: bool) -> bool;
}
