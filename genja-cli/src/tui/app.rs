//! Full-screen task browser orchestration.
//!
//! The runner loads descriptors before taking terminal ownership, then draws
//! the browser and dispatches events until the app receives a quit request.
//! The process helper selects compiled Rust discovery for project-local binaries.

use std::{error::Error, fmt, io, process::ExitCode};

use crossterm::event::{self, Event};

use super::{BrowserOutcome, TaskBrowser, terminal::TerminalSession};
use crate::discovery::{DiscoveryError, TaskDescriptorSource, rust::CompiledTaskDescriptorSource};

/// Options for the full-screen task browser runner.
///
/// No runner settings are configurable yet. Use [`TuiOptions::default()`] to
/// leave room for future options without changing the runner call shape.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TuiOptions {}

/// Error returned by the full-screen task browser runner.
#[derive(Debug)]
#[non_exhaustive]
pub enum TuiError {
    /// The descriptor source failed before terminal setup.
    Discovery(DiscoveryError),
    /// Terminal setup, drawing, event reading, or restoration failed.
    Terminal(io::Error),
    /// The event loop failed and terminal restoration also failed.
    RunAndRestore {
        /// Error from drawing or reading an event.
        run: io::Error,
        /// Error from restoring terminal modes.
        restore: io::Error,
    },
}

impl fmt::Display for TuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(error) => write!(f, "task discovery failed: {error}"),
            Self::Terminal(error) => write!(f, "terminal error: {error}"),
            Self::RunAndRestore { run, restore } => {
                write!(
                    f,
                    "terminal error: {run}; failed to restore terminal: {restore}"
                )
            }
        }
    }
}

impl Error for TuiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Discovery(error) => Some(error),
            Self::Terminal(error) => Some(error),
            Self::RunAndRestore { run, .. } => Some(run),
        }
    }
}

struct FullScreenApp {
    browser: TaskBrowser,
    quit_requested: bool,
}

impl FullScreenApp {
    fn load<S>(source: &S) -> Result<Self, TuiError>
    where
        S: TaskDescriptorSource + ?Sized,
    {
        let mut browser = TaskBrowser::new();
        browser.load_from(source).map_err(TuiError::Discovery)?;
        Ok(Self {
            browser,
            quit_requested: false,
        })
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        match self.browser.handle_event(event) {
            BrowserOutcome::QuitRequested => {
                self.quit_requested = true;
                false
            }
            BrowserOutcome::Changed => true,
            BrowserOutcome::Ignored => matches!(event, Event::Resize(_, _)),
        }
    }
}

/// Run the minimal task browser in a full-screen terminal session.
///
/// Descriptor loading is synchronous and finishes before raw mode or the
/// alternate screen is entered. On success, the runner draws once, then redraws
/// after selection changes or resize events. Press `q` or Escape to leave.
/// The terminal is restored after normal exit, returned errors, and panic
/// unwinding. A discovery error returns without touching terminal modes.
///
/// This is a minimal shell: task rows, filtering, details, and execution are
/// not implemented. [`run_main`] selects compiled Rust discovery for
/// project-local binaries. The CLI's `genja tui` command uses the same runner.
/// Applications that already own a terminal should embed [`TaskBrowser`].
pub fn run_tui<S>(source: &S, _options: TuiOptions) -> Result<(), TuiError>
where
    S: TaskDescriptorSource + ?Sized,
{
    let mut app = FullScreenApp::load(source)?;
    let mut session = TerminalSession::new().map_err(TuiError::Terminal)?;
    run_and_restore(
        &mut session,
        |session| {
            run_loop(
                &mut app,
                |browser| {
                    session
                        .terminal()
                        .draw(|frame| browser.render(frame, frame.area()))
                        .map(|_| ())
                },
                event::read,
            )
        },
        TerminalSession::restore,
    )
}

// Keep cleanup in one path, including when running and restoring both fail.
fn run_and_restore<S, F, R>(session: &mut S, run: F, restore: R) -> Result<(), TuiError>
where
    F: FnOnce(&mut S) -> io::Result<()>,
    R: FnOnce(&mut S) -> io::Result<()>,
{
    let run_result = run(session);
    let restore_result = restore(session);

    match (run_result, restore_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(run), Ok(())) => Err(TuiError::Terminal(run)),
        (Ok(()), Err(restore)) => Err(TuiError::Terminal(restore)),
        (Err(run), Err(restore)) => Err(TuiError::RunAndRestore { run, restore }),
    }
}

/// Run the full-screen task browser using compiled Rust tasks in this process.
///
/// Project-local binaries must link their task crate before calling this
/// helper. A generic installed binary only sees registrations linked into that
/// binary. The helper reports errors to stderr after terminal restoration and
/// returns a process exit code. It does not parse CLI arguments.
///
/// ```no_run
/// fn main() -> std::process::ExitCode {
///     genja_cli::tui::run_main()
/// }
/// ```
pub fn run_main() -> ExitCode {
    let source = CompiledTaskDescriptorSource::new();
    match run_tui(&source, TuiOptions::default()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_loop<D, R>(app: &mut FullScreenApp, mut draw: D, mut read_event: R) -> io::Result<()>
where
    D: FnMut(&TaskBrowser) -> io::Result<()>,
    R: FnMut() -> io::Result<Event>,
{
    draw(&app.browser)?;
    while !app.quit_requested {
        let event = read_event()?;
        if app.handle_event(&event) {
            draw(&app.browser)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, collections::VecDeque};

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::discovery::{DiscoveryResult, TaskDescriptor};
    use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};

    struct Source(DiscoveryResult<Vec<TaskDescriptor>>);

    impl TaskDescriptorSource for Source {
        fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
            self.0.clone()
        }
    }

    fn descriptor(id: &str) -> TaskDescriptor {
        TaskDescriptor::explicit(
            id,
            "1.0.0",
            TaskDescriptorMetadata {
                name: id.to_string(),
                description: None,
                execution_mode: TaskExecutionMode::Blocking,
                connection_plugin_name: None,
                processor_names: Vec::new(),
                retry: None,
            },
            None,
            false,
        )
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn discovery_fails_before_terminal_ownership() {
        let error = DiscoveryError::source_failed("registry unavailable");
        let source = Source(Err(error.clone()));
        assert!(matches!(
            run_tui(&source, TuiOptions::default()),
            Err(TuiError::Discovery(actual)) if actual == error
        ));
    }

    #[test]
    fn runner_restores_after_initial_draw_redraw_and_event_read_errors() {
        for failure in ["initial draw", "redraw", "event read"] {
            let mut app = FullScreenApp::load(&Source(Ok(Vec::new()))).unwrap();
            let mut calls = Vec::new();
            let draws = Cell::new(0);
            let result = run_and_restore(
                &mut calls,
                |calls| {
                    calls.push("run");
                    run_loop(
                        &mut app,
                        |_| {
                            draws.set(draws.get() + 1);
                            if failure == "initial draw"
                                || (failure == "redraw" && draws.get() == 2)
                            {
                                Err(io::Error::other(failure))
                            } else {
                                Ok(())
                            }
                        },
                        || {
                            if failure == "event read" {
                                Err(io::Error::other(failure))
                            } else {
                                Ok(Event::Resize(10, 4))
                            }
                        },
                    )
                },
                |calls| {
                    calls.push("restore");
                    Ok(())
                },
            );
            assert_eq!(calls, ["run", "restore"], "{failure}");
            assert!(
                matches!(result, Err(TuiError::Terminal(error)) if error.to_string() == failure)
            );
        }
    }

    #[test]
    fn runner_reports_restoration_failure_after_normal_exit() {
        let result = run_and_restore(
            &mut (),
            |_| Ok(()),
            |_| Err(io::Error::other("restore failed")),
        );
        assert!(matches!(result, Err(TuiError::Terminal(error))
            if error.to_string() == "restore failed"));
    }

    #[test]
    fn runner_preserves_both_run_and_restore_errors() {
        let error = run_and_restore(
            &mut (),
            |_| Err(io::Error::other("draw failed")),
            |_| Err(io::Error::other("restore failed")),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "terminal error: draw failed; failed to restore terminal: restore failed"
        );
        assert_eq!(error.source().unwrap().to_string(), "draw failed");
        assert!(matches!(error, TuiError::RunAndRestore { run, restore }
            if run.to_string() == "draw failed" && restore.to_string() == "restore failed"));
    }

    #[test]
    fn loop_draws_on_startup_resize_and_selection_then_quits() {
        let mut app =
            FullScreenApp::load(&Source(Ok(vec![descriptor("first"), descriptor("second")])))
                .unwrap();
        let mut terminal = Terminal::new(TestBackend::new(40, 6)).unwrap();
        let events = VecDeque::from([
            Event::Resize(40, 6),
            key(KeyCode::Down),
            key(KeyCode::Char('q')),
        ]);
        let mut events = events.into_iter();
        let draws = Cell::new(0);

        run_loop(
            &mut app,
            |browser| {
                draws.set(draws.get() + 1);
                terminal
                    .draw(|frame| browser.render(frame, frame.area()))
                    .unwrap();
                Ok(())
            },
            || Ok(events.next().expect("quit event was not handled")),
        )
        .unwrap();

        assert_eq!(draws.get(), 3);
        assert_eq!(app.browser.state().selected_index(), Some(1));
        assert!(app.quit_requested);
    }

    #[test]
    fn loop_propagates_event_read_errors() {
        let mut app = FullScreenApp::load(&Source(Ok(Vec::new()))).unwrap();
        let draws = Cell::new(0);
        let result = run_loop(
            &mut app,
            |_| {
                draws.set(draws.get() + 1);
                Ok(())
            },
            || Err(io::Error::other("event read failed")),
        );
        assert_eq!(result.unwrap_err().to_string(), "event read failed");
        assert_eq!(draws.get(), 1);
        assert!(!app.quit_requested);
    }

    #[test]
    fn loop_propagates_redraw_errors_without_reading_more_events() {
        let mut app = FullScreenApp::load(&Source(Ok(vec![descriptor("only")]))).unwrap();
        let draws = Cell::new(0);
        let reads = Cell::new(0);
        let result = run_loop(
            &mut app,
            |_| {
                draws.set(draws.get() + 1);
                if draws.get() == 2 {
                    Err(io::Error::other("draw failed"))
                } else {
                    Ok(())
                }
            },
            || {
                reads.set(reads.get() + 1);
                Ok(Event::Resize(10, 4))
            },
        );
        assert_eq!(result.unwrap_err().to_string(), "draw failed");
        assert_eq!(draws.get(), 2);
        assert_eq!(reads.get(), 1);
    }
}
