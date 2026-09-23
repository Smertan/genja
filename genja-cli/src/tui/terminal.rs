//! Crossterm terminal ownership boundary for the full-screen runner.
//!
//! Setup and restoration will be confined here, including raw mode, alternate
//! screen, and cursor state. Cleanup must cover normal exit, setup or loop
//! errors, and panic unwinding. Embedded browser usage must not acquire this
//! terminal session or install process-wide hooks.
