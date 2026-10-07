use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::error_handler::{ErrorType, RenderError, RenderErrorHandler, RenderErrorPolicy};

const LOG_EVERY: Duration = Duration::from_secs(5);
const OOM_WINDOW: Duration = Duration::from_secs(10);
const OOM_QUIT_AFTER: u32 = 30;

static GATE: Mutex<LogGate> = Mutex::new(LogGate::new());
static OOM: Mutex<OomCounter> = Mutex::new(OomCounter::new());
static QUITTING: AtomicBool = AtomicBool::new(false);

pub(crate) fn install(app: &mut App) {
    app.insert_resource(RenderErrorHandler(policy));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Response {
    Ignore,
    Quit,
}

fn respond(ty: ErrorType, recent_oom: u32) -> Response {
    match ty {
        ErrorType::Validation | ErrorType::Internal => Response::Ignore,
        ErrorType::OutOfMemory if recent_oom < OOM_QUIT_AFTER => Response::Ignore,
        _ => Response::Quit,
    }
}

struct LogGate {
    last: Option<Instant>,
    suppressed: u32,
}

impl LogGate {
    const fn new() -> Self {
        Self {
            last: None,
            suppressed: 0,
        }
    }

    fn admit(&mut self, now: Instant) -> Option<u32> {
        match self.last {
            Some(last) if now.saturating_duration_since(last) < LOG_EVERY => {
                self.suppressed = self.suppressed.saturating_add(1);
                None
            }
            _ => {
                self.last = Some(now);
                Some(std::mem::take(&mut self.suppressed))
            }
        }
    }
}

struct OomCounter {
    window_start: Option<Instant>,
    count: u32,
}

impl OomCounter {
    const fn new() -> Self {
        Self {
            window_start: None,
            count: 0,
        }
    }

    fn record(&mut self, now: Instant) -> u32 {
        match self.window_start {
            Some(start) if now.saturating_duration_since(start) < OOM_WINDOW => {}
            _ => {
                self.window_start = Some(now);
                self.count = 0;
            }
        }
        self.count = self.count.saturating_add(1);
        self.count
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn policy(
    error: &RenderError,
    main_world: &mut World,
    _render_world: &mut World,
) -> RenderErrorPolicy {
    let now = Instant::now();
    let recent_oom = if error.ty == ErrorType::OutOfMemory {
        lock(&OOM).record(now)
    } else {
        0
    };
    match respond(error.ty, recent_oom) {
        Response::Ignore => {
            if let Some(suppressed) = lock(&GATE).admit(now) {
                diag::error!(
                    World,
                    "render error ignored ({:?}): {} ({} more since the last report)",
                    error.ty,
                    error.description,
                    suppressed
                );
            }
            RenderErrorPolicy::Ignore
        }
        Response::Quit => {
            if !QUITTING.swap(true, Ordering::Relaxed) {
                diag::error!(
                    World,
                    "render error quits the game ({:?}): {}",
                    error.ty,
                    error.description
                );
            }
            main_world.write_message(AppExit::error());
            RenderErrorPolicy::StopRendering
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_validation_error_is_ignored() {
        assert_eq!(respond(ErrorType::Validation, 0), Response::Ignore);
    }

    #[test]
    fn an_internal_error_is_ignored() {
        assert_eq!(respond(ErrorType::Internal, 0), Response::Ignore);
    }

    #[test]
    fn a_lost_device_still_quits() {
        assert_eq!(respond(ErrorType::DeviceLost, 0), Response::Quit);
    }

    #[test]
    fn out_of_memory_is_ignored_until_it_repeats() {
        assert_eq!(respond(ErrorType::OutOfMemory, 0), Response::Ignore);
        assert_eq!(
            respond(ErrorType::OutOfMemory, OOM_QUIT_AFTER - 1),
            Response::Ignore
        );
        assert_eq!(
            respond(ErrorType::OutOfMemory, OOM_QUIT_AFTER),
            Response::Quit
        );
    }

    #[test]
    fn out_of_memory_counts_only_the_recent_window() {
        let mut counter = OomCounter::new();
        let t0 = Instant::now();
        assert_eq!(counter.record(t0), 1);
        assert_eq!(counter.record(t0 + Duration::from_secs(9)), 2);
        assert_eq!(counter.record(t0 + Duration::from_secs(11)), 1);
    }

    #[test]
    fn thirty_out_of_memory_errors_in_ten_seconds_quit() {
        let mut counter = OomCounter::new();
        let t0 = Instant::now();
        let responses: Vec<Response> = (0..OOM_QUIT_AFTER)
            .map(|i| {
                let recent = counter.record(t0 + Duration::from_millis(u64::from(i) * 100));
                respond(ErrorType::OutOfMemory, recent)
            })
            .collect();
        assert!(
            responses[..responses.len() - 1]
                .iter()
                .all(|r| *r == Response::Ignore)
        );
        assert_eq!(responses.last(), Some(&Response::Quit));
    }

    #[test]
    fn the_log_gate_reports_the_first_error_then_waits() {
        let mut gate = LogGate::new();
        let t0 = Instant::now();
        assert_eq!(gate.admit(t0), Some(0));
        assert_eq!(gate.admit(t0 + Duration::from_secs(1)), None);
        assert_eq!(gate.admit(t0 + Duration::from_secs(2)), None);
        assert_eq!(gate.admit(t0 + Duration::from_secs(6)), Some(2));
        assert_eq!(gate.admit(t0 + Duration::from_secs(7)), None);
    }
}
