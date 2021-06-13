use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use color_eyre::eyre;
use env_logger::Env;
use trillium::{Conn, State};
use trillium_logger::Logger;
use trillium_router::Router;
use trillium_tokio::Stopper;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let stopper = Arc::new(Mutex::new(Stopper::new()));
    let quit = Arc::new(AtomicBool::new(false));

    ctrlc::set_handler({
        let stopper = stopper.clone();
        let quit = quit.clone();
        move || {
            quit.store(true, Ordering::SeqCst);
            stopper.lock().unwrap().stop();
        }
    })?;

    loop {
        let new_stopper = Stopper::new();

        // Reset stopper at the start of the loop
        *stopper.clone().lock().unwrap() = new_stopper.clone();

        trillium_tokio::config()
            .with_host("0.0.0.0")
            .with_stopper(new_stopper.clone())
            .without_signals()
            .run((State::new(new_stopper), Logger::new(), router()));

        if quit.load(Ordering::SeqCst) {
            break;
        }

        log::info!("suspending system (hibernate: false)");
        unsafe { SetSuspendState(false, false, false) };
    }

    log::info!("shutting down");

    Ok(())
}

fn router() -> Router {
    Router::build(|mut router| {
        router.get("/", ping);
        router.post("/suspend", suspend);
    })
}

async fn ping(conn: Conn) -> Conn {
    conn.ok("ok")
}

async fn suspend(conn: Conn) -> Conn {
    let stopper: &Stopper = conn.state().unwrap();
    stopper.stop();
    conn.ok("ok")
}

#[link(name = "powrprof")]
extern "C" {
    fn SetSuspendState(hibernate: bool, force: bool, wakeup_events_disabled: bool) -> bool;
}
