//! Browser actions and Crossterm event translation boundary.
//!
//! Event polling belongs to the host application. This module will translate
//! supplied events into actions so browser state updates can be tested without
//! terminal input and embedded hosts can retain unhandled events.
