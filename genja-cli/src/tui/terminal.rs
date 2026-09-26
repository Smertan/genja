//! Crossterm terminal ownership for the full-screen runner.
//!
//! Setup and restoration are confined here. The guard restores terminal modes
//! during normal exit, setup failure, returned errors, and panic unwinding.
//! Embedded browser usage never acquires this session or changes panic hooks.

use std::io::{self, IsTerminal, Stdout, stdout};

use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        is_raw_mode_enabled,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};

trait TerminalOps {
    fn enable_raw(&mut self) -> io::Result<()>;
    fn disable_raw(&mut self) -> io::Result<()>;
    fn enter_alternate(&mut self) -> io::Result<()>;
    fn leave_alternate(&mut self) -> io::Result<()>;
    fn hide_cursor(&mut self) -> io::Result<()>;
    fn show_cursor(&mut self) -> io::Result<()>;
}

struct CrosstermOps {
    output: Stdout,
}

impl TerminalOps for CrosstermOps {
    fn enable_raw(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }

    fn enter_alternate(&mut self) -> io::Result<()> {
        execute!(self.output, EnterAlternateScreen)
    }

    fn leave_alternate(&mut self) -> io::Result<()> {
        execute!(self.output, LeaveAlternateScreen)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        execute!(self.output, Hide)
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        execute!(self.output, Show)
    }
}

struct TerminalGuard<O: TerminalOps> {
    ops: O,
    raw_mode: bool,
    alternate_screen: bool,
    hidden_cursor: bool,
}

impl<O: TerminalOps> TerminalGuard<O> {
    fn new(ops: O) -> io::Result<Self> {
        let mut guard = Self {
            ops,
            raw_mode: false,
            alternate_screen: false,
            hidden_cursor: false,
        };

        // Mark each step before executing it. If a command writes partially
        // before failing, Drop still attempts the matching restoration.
        guard.raw_mode = true;
        guard.ops.enable_raw()?;
        guard.alternate_screen = true;
        guard.ops.enter_alternate()?;
        guard.hidden_cursor = true;
        guard.ops.hide_cursor()?;
        Ok(guard)
    }

    fn restore(&mut self) -> io::Result<()> {
        let mut first_error = None;

        if self.hidden_cursor {
            match self.ops.show_cursor() {
                Ok(()) => self.hidden_cursor = false,
                Err(error) => first_error = Some(error),
            }
        }
        if self.alternate_screen {
            match self.ops.leave_alternate() {
                Ok(()) => self.alternate_screen = false,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        if self.raw_mode {
            match self.ops.disable_raw() {
                Ok(()) => self.raw_mode = false,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }

        first_error.map_or(Ok(()), Err)
    }
}

impl<O: TerminalOps> Drop for TerminalGuard<O> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub(super) struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    guard: TerminalGuard<CrosstermOps>,
}

impl TerminalSession {
    pub(super) fn new() -> io::Result<Self> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "the TUI requires an interactive terminal",
            ));
        }
        if is_raw_mode_enabled()? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the TUI cannot take ownership while raw mode is already enabled",
            ));
        }

        let guard = TerminalGuard::new(CrosstermOps { output: stdout() })?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.clear()?;
        Ok(Self { terminal, guard })
    }

    pub(super) fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    pub(super) fn restore(&mut self) -> io::Result<()> {
        self.guard.restore()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    struct FakeOps {
        calls: Rc<RefCell<Vec<&'static str>>>,
        fail_on: Option<&'static str>,
    }

    impl FakeOps {
        fn call(&self, name: &'static str) -> io::Result<()> {
            self.calls.borrow_mut().push(name);
            if self.fail_on == Some(name) {
                Err(io::Error::other(format!("{name} failed")))
            } else {
                Ok(())
            }
        }
    }

    impl TerminalOps for FakeOps {
        fn enable_raw(&mut self) -> io::Result<()> {
            self.call("enable_raw")
        }
        fn disable_raw(&mut self) -> io::Result<()> {
            self.call("disable_raw")
        }
        fn enter_alternate(&mut self) -> io::Result<()> {
            self.call("enter_alternate")
        }
        fn leave_alternate(&mut self) -> io::Result<()> {
            self.call("leave_alternate")
        }
        fn hide_cursor(&mut self) -> io::Result<()> {
            self.call("hide_cursor")
        }
        fn show_cursor(&mut self) -> io::Result<()> {
            self.call("show_cursor")
        }
    }

    fn fake(fail_on: Option<&'static str>) -> (FakeOps, Rc<RefCell<Vec<&'static str>>>) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        (
            FakeOps {
                calls: Rc::clone(&calls),
                fail_on,
            },
            calls,
        )
    }

    #[test]
    fn normal_exit_restores_once_in_reverse_order() {
        let (ops, calls) = fake(None);
        let mut guard = TerminalGuard::new(ops).unwrap_or_else(|_| panic!("setup failed"));
        guard.restore().unwrap();
        drop(guard);
        assert_eq!(
            *calls.borrow(),
            [
                "enable_raw",
                "enter_alternate",
                "hide_cursor",
                "show_cursor",
                "leave_alternate",
                "disable_raw",
            ]
        );
    }

    #[test]
    fn failed_setup_restores_modes_that_may_have_changed() {
        let cases: &[(&str, &[&str])] = &[
            ("enable_raw", &["enable_raw", "disable_raw"]),
            (
                "enter_alternate",
                &[
                    "enable_raw",
                    "enter_alternate",
                    "leave_alternate",
                    "disable_raw",
                ],
            ),
            (
                "hide_cursor",
                &[
                    "enable_raw",
                    "enter_alternate",
                    "hide_cursor",
                    "show_cursor",
                    "leave_alternate",
                    "disable_raw",
                ],
            ),
        ];
        for &(failure, expected) in cases {
            let (ops, calls) = fake(Some(failure));
            let error = TerminalGuard::new(ops).err().expect("setup should fail");
            assert_eq!(error.to_string(), format!("{failure} failed"));
            assert_eq!(calls.borrow().as_slice(), expected, "{failure}");
        }
    }

    #[test]
    fn restoration_continues_after_one_command_fails() {
        for failure in ["show_cursor", "leave_alternate", "disable_raw"] {
            let (ops, calls) = fake(Some(failure));
            let mut guard = TerminalGuard::new(ops).unwrap_or_else(|_| panic!("setup failed"));
            let error = guard.restore().unwrap_err();
            assert_eq!(error.to_string(), format!("{failure} failed"));
            assert_eq!(
                *calls.borrow(),
                [
                    "enable_raw",
                    "enter_alternate",
                    "hide_cursor",
                    "show_cursor",
                    "leave_alternate",
                    "disable_raw",
                ],
                "{failure}"
            );
            calls.borrow_mut().clear();
            drop(guard);
            assert_eq!(
                *calls.borrow(),
                [failure],
                "Drop should retry only {failure}"
            );
        }
    }

    #[test]
    fn unwind_restores_terminal_modes() {
        let (ops, calls) = fake(None);
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = TerminalGuard::new(ops).unwrap_or_else(|_| panic!("setup failed"));
            panic!("test unwind");
        }));
        assert!(unwind.is_err());
        assert_eq!(calls.borrow().last(), Some(&"disable_raw"));
    }
}
