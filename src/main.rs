use std::io::Write;
use std::net::{TcpListener, TcpStream};

use color_eyre::eyre::{self, eyre};
use env_logger::Env;
use serde::Deserialize;
use serde_json::{json, Deserializer, Value};

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    let listener = TcpListener::bind("0.0.0.0:8080")?;

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(e) => {
                log::error!("{}", e);
                continue;
            }
        };

        let message = Deserializer::from_reader(&stream)
            .into_iter::<Value>()
            .next()
            .ok_or_else(|| eyre!("no payload"))
            .and_then(|x| x.map_err(|e| eyre!(e)));

        let message = match message {
            Ok(data) => data,
            Err(e) => {
                log::error!("{}", e);
                continue;
            }
        };

        log::debug!("{}", message);

        let message = match serde_json::from_value(message) {
            Ok(message) => message,
            Err(e) => {
                log::error!("{}", e);
                continue;
            }
        };

        handle_message(message, stream);
    }

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
