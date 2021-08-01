mod monitors;
mod powrprof;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

use color_eyre::eyre::{self, Context};
use env_logger::Env;
use serde::Deserialize;
use serde_json::{json, Value};

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
    GetMonitorList,
    SetMonitorPower {
        id: i32,
        mode: MonitorPowerMode,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum MonitorPowerMode {
    On,
    Off,
}

#[derive(Default, Deserialize)]
struct SuspendOptions {
    hibernate: bool,
}

fn handle_message(message: Message, stream: TcpStream) {
    let res = match message {
        Message::Suspend { options } => return handle_suspend(options, stream),
        Message::GetMonitorList => handle_list_monitors(),
        Message::SetMonitorPower { id, mode } => handle_set_monitor_power(id, mode),
    };

    fn send_res(mut stream: TcpStream, res: &Value) -> eyre::Result<()> {
        let res = serde_json::to_vec(res)?;

        stream.write_all(&res)?;
        stream.flush()?;

        Ok(())
    }

    if let Err(e) = send_res(stream, &res) {
        log::error!("{}", e);
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
    powrprof::set_suspend_state(options.hibernate);
}

fn handle_list_monitors() -> Value {
    let monitors = monitors::get_monitors().into_iter().map(|monitor| {
        json!({
            "id": monitor.id(),
            "name": monitor.name(),
            "power_mode": match monitor.power_mode() {
                monitors::PowerMode::On => "on",
                monitors::PowerMode::Off => "off",
            },
        })
    });

    json!(monitors.collect::<Vec<_>>())
}

fn handle_set_monitor_power(id: i32, mode: MonitorPowerMode) -> Value {
    let res = monitors::get_monitors()
        .into_iter()
        .find(|monitor| monitor.id() == id)
        .iter()
        .try_for_each(|monitor| {
            monitor.set_power_mode(match mode {
                MonitorPowerMode::On => monitors::PowerMode::On,
                MonitorPowerMode::Off => monitors::PowerMode::Off,
            })
        });

    if let Err(e) = &res {
        log::error!("{}", e);
    }

    json!({
        "success": res.is_ok(),
    })
}
