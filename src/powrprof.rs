#[link(name = "powrprof")]
extern "C" {
    fn SetSuspendState(hibernate: bool, force: bool, wakeup_events_disabled: bool) -> bool;
}

pub fn set_suspend_state(hibernate: bool) {
    unsafe { SetSuspendState(hibernate, false, false) };
}
