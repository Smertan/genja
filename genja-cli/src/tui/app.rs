//! Full-screen application orchestration boundary.
//!
//! The runner will load descriptors, own the terminal session, draw the browser,
//! and dispatch events until its quit state is set. The process helper will
//! select compiled Rust discovery and map runner results to an exit code.
//! Neither entrypoint is implemented in this foundation phase.
