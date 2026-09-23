//! Embeddable task browser and descriptor loading boundary.
//!
//! The browser will accept the shared task descriptor source abstraction and
//! hold an owned descriptor snapshot. Loading, action handling, and rendering
//! will be separate operations. It must not select a registry backend, poll
//! terminal events, change terminal modes, or terminate the host process.
