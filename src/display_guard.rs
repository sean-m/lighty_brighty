/*
    lighty_brighty - display_guard

    On KDE Plasma, KWin/KScreen manages display topology. Rather than assume a
    single laptop panel is always the only active output, we watch for extra
    connected+enabled outputs (e.g. when docking or connecting a projector)
    and pause automatic brightness adjustment while more than one is active.

    Detection is done via `kscreen-doctor -j`, which is stable across Plasma
    versions and avoids depending on KScreen's internal DBus API surface
    (which is not considered stable/public). We poll on an interval rather
    than busy-waiting.

    To avoid brightness "flapping" back on immediately when a user briefly
    unplugs/replugs a cable, unpausing requires the display topology to have
    reported exactly one active output continuously for a debounce period
    (see `PauseState`).
*/

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{debug, info, warn};
use serde::Deserialize;

/// Default amount of time a single-display topology must be stable before
/// automatic brightness adjustment resumes. Chosen from the middle of the
/// 5-10s range suggested for avoiding dock/undock flapping.
pub const DEFAULT_DEBOUNCE_SECONDS: u64 = 8;

/// How often to poll `kscreen-doctor -j` for topology changes.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Deserialize)]
struct KScreenDoctorOutput {
    connected: bool,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct KScreenDoctorState {
    #[serde(default)]
    outputs: Vec<KScreenDoctorOutput>,
}

#[derive(Debug)]
pub enum DisplayGuardError {
    Spawn(std::io::Error),
    NonZeroExit(std::process::ExitStatus),
    Parse(serde_json::Error),
}

impl fmt::Display for DisplayGuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisplayGuardError::Spawn(e) => write!(f, "failed to run kscreen-doctor: {}", e),
            DisplayGuardError::NonZeroExit(status) => {
                write!(f, "kscreen-doctor exited with {}", status)
            }
            DisplayGuardError::Parse(e) => write!(f, "failed to parse kscreen-doctor JSON: {}", e),
        }
    }
}

impl std::error::Error for DisplayGuardError {}

/// Pure parsing helper, kept separate from process execution so it can be
/// unit tested without needing `kscreen-doctor` installed.
fn count_active_outputs_from_json(json: &str) -> Result<usize, DisplayGuardError> {
    let state: KScreenDoctorState = serde_json::from_str(json).map_err(DisplayGuardError::Parse)?;
    Ok(state
        .outputs
        .iter()
        .filter(|o| o.connected && o.enabled)
        .count())
}

/// Query `kscreen-doctor -j` and count active (connected+enabled) outputs.
async fn active_output_count() -> Result<usize, DisplayGuardError> {
    let output = async_std::process::Command::new("kscreen-doctor")
        .arg("-j")
        .output()
        .await
        .map_err(DisplayGuardError::Spawn)?;

    if !output.status.success() {
        return Err(DisplayGuardError::NonZeroExit(output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    count_active_outputs_from_json(&stdout)
}

/// Result of feeding a new output-count observation into `PauseState`, used
/// purely for logging state transitions.
#[derive(Debug, PartialEq, Eq)]
pub enum PauseTransition {
    /// Went from unpaused to paused because more than one output is active.
    PausedMultiDisplay { active_outputs: usize },
    /// Returned to a single active output; debounce countdown started.
    DebounceStarted,
    /// A second output reappeared while debouncing; countdown reset.
    DebounceCancelled,
    /// Single-display state has been stable for the full debounce duration.
    ResumedAfterDebounce,
}

/// Pure, synchronous state machine implementing the pause/debounce policy.
/// Deliberately has no knowledge of DBus/kscreen-doctor/system calls so it
/// can be unit tested by feeding it counts and instants directly.
pub struct PauseState {
    debounce: Duration,
    paused: bool,
    single_display_since: Option<Instant>,
}

impl PauseState {
    pub fn new(debounce: Duration) -> Self {
        Self {
            debounce,
            paused: false,
            single_display_since: None,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Feed a new active-output-count observation at time `now`. Returns
    /// `Some(transition)` if the pause state (or debounce countdown)
    /// changed as a result, for logging purposes.
    pub fn observe(&mut self, active_outputs: usize, now: Instant) -> Option<PauseTransition> {
        if active_outputs > 1 {
            let was_debouncing = self.single_display_since.take().is_some();
            if !self.paused {
                self.paused = true;
                return Some(PauseTransition::PausedMultiDisplay { active_outputs });
            }
            if was_debouncing {
                return Some(PauseTransition::DebounceCancelled);
            }
            return None;
        }

        // active_outputs <= 1: single (or no detected) display.
        if !self.paused {
            return None;
        }

        match self.single_display_since {
            None => {
                self.single_display_since = Some(now);
                Some(PauseTransition::DebounceStarted)
            }
            Some(since) => {
                if now.saturating_duration_since(since) >= self.debounce {
                    self.paused = false;
                    self.single_display_since = None;
                    Some(PauseTransition::ResumedAfterDebounce)
                } else {
                    None
                }
            }
        }
    }
}

/// Poll `kscreen-doctor -j` on an interval, feed observations into a
/// `PauseState`, and publish the resulting paused flag into `paused` for the
/// brightness loop to check. Runs until the process exits; logs concise
/// messages on every state transition.
pub async fn monitor_displays(
    paused: Arc<AtomicBool>,
    debounce: Duration,
    poll_interval: Duration,
) {
    let mut state = PauseState::new(debounce);

    loop {
        match active_output_count().await {
            Ok(count) => {
                debug!("kscreen-doctor reports {} active output(s).", count);
                if let Some(transition) = state.observe(count, Instant::now()) {
                    match transition {
                        PauseTransition::PausedMultiDisplay { active_outputs } => {
                            info!(
                                "Pausing automatic brightness: {} active displays detected.",
                                active_outputs
                            );
                        }
                        PauseTransition::DebounceStarted => {
                            info!(
                                "Single display detected; debouncing for {:?} before resuming automatic brightness.",
                                debounce
                            );
                        }
                        PauseTransition::DebounceCancelled => {
                            info!(
                                "Additional display reappeared during debounce; remaining paused."
                            );
                        }
                        PauseTransition::ResumedAfterDebounce => {
                            info!("Single display stable for debounce period; resuming automatic brightness.");
                        }
                    }
                    paused.store(state.is_paused(), Ordering::Relaxed);
                }
            }
            Err(e) => {
                warn!(
                    "Could not determine active display count, leaving pause state unchanged: {}",
                    e
                );
            }
        }

        async_std::task::sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_active_outputs_from_json() {
        let json = r#"{"outputs":[
            {"connected":true,"enabled":true},
            {"connected":true,"enabled":false},
            {"connected":false,"enabled":false}
        ]}"#;
        assert_eq!(count_active_outputs_from_json(json).unwrap(), 1);
    }

    #[test]
    fn parses_multiple_active_outputs() {
        let json = r#"{"outputs":[
            {"connected":true,"enabled":true},
            {"connected":true,"enabled":true}
        ]}"#;
        assert_eq!(count_active_outputs_from_json(json).unwrap(), 2);
    }

    #[test]
    fn returns_parse_error_on_invalid_json() {
        assert!(count_active_outputs_from_json("not json").is_err());
    }

    #[test]
    fn missing_outputs_field_counts_as_zero() {
        assert_eq!(count_active_outputs_from_json("{}").unwrap(), 0);
    }

    #[test]
    fn starts_unpaused() {
        let state = PauseState::new(Duration::from_secs(8));
        assert!(!state.is_paused());
    }

    #[test]
    fn pauses_immediately_when_second_display_appears() {
        let mut state = PauseState::new(Duration::from_secs(8));
        let t0 = Instant::now();
        let transition = state.observe(2, t0);
        assert_eq!(
            transition,
            Some(PauseTransition::PausedMultiDisplay { active_outputs: 2 })
        );
        assert!(state.is_paused());
    }

    #[test]
    fn stays_paused_before_debounce_elapses() {
        let mut state = PauseState::new(Duration::from_secs(8));
        let t0 = Instant::now();
        state.observe(2, t0);
        let transition = state.observe(1, t0 + Duration::from_secs(1));
        assert_eq!(transition, Some(PauseTransition::DebounceStarted));
        assert!(state.is_paused());

        // Still within debounce window: no change yet.
        let transition = state.observe(1, t0 + Duration::from_secs(5));
        assert_eq!(transition, None);
        assert!(state.is_paused());
    }

    #[test]
    fn resumes_after_debounce_elapses_with_single_display() {
        let mut state = PauseState::new(Duration::from_secs(8));
        let t0 = Instant::now();
        state.observe(2, t0);
        state.observe(1, t0 + Duration::from_secs(1));

        let transition = state.observe(1, t0 + Duration::from_secs(1) + Duration::from_secs(8));
        assert_eq!(transition, Some(PauseTransition::ResumedAfterDebounce));
        assert!(!state.is_paused());
    }

    #[test]
    fn cancels_debounce_if_second_display_returns() {
        let mut state = PauseState::new(Duration::from_secs(8));
        let t0 = Instant::now();
        state.observe(2, t0);
        state.observe(1, t0 + Duration::from_secs(1));

        // Docked again mid-debounce: remains paused, debounce cancelled.
        let transition = state.observe(2, t0 + Duration::from_secs(3));
        assert_eq!(transition, Some(PauseTransition::DebounceCancelled));
        assert!(state.is_paused());

        // Debounce must restart from scratch; old elapsed time doesn't count.
        let transition = state.observe(1, t0 + Duration::from_secs(4));
        assert_eq!(transition, Some(PauseTransition::DebounceStarted));
        assert!(state.is_paused());

        let transition = state.observe(1, t0 + Duration::from_secs(4) + Duration::from_secs(7));
        assert_eq!(transition, None);
        assert!(state.is_paused());

        let transition = state.observe(1, t0 + Duration::from_secs(4) + Duration::from_secs(8));
        assert_eq!(transition, Some(PauseTransition::ResumedAfterDebounce));
        assert!(!state.is_paused());
    }

    #[test]
    fn zero_active_outputs_treated_as_single_display() {
        // e.g. transient state during topology change / detection hiccup.
        let mut state = PauseState::new(Duration::from_secs(8));
        let t0 = Instant::now();
        state.observe(2, t0);
        state.observe(0, t0 + Duration::from_secs(1));
        let transition = state.observe(0, t0 + Duration::from_secs(9));
        assert_eq!(transition, Some(PauseTransition::ResumedAfterDebounce));
        assert!(!state.is_paused());
    }

    #[test]
    fn repeated_single_observations_do_not_restart_debounce() {
        let mut state = PauseState::new(Duration::from_secs(8));
        let t0 = Instant::now();
        state.observe(2, t0);
        state.observe(1, t0 + Duration::from_secs(1)); // DebounceStarted
        state.observe(1, t0 + Duration::from_secs(2)); // still within window, no restart
        let transition = state.observe(1, t0 + Duration::from_secs(1) + Duration::from_secs(8));
        assert_eq!(transition, Some(PauseTransition::ResumedAfterDebounce));
    }
}
