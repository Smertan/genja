//! Task browser layout and rendering boundary.
//!
//! Rendering will use a caller-provided Ratatui frame and area, tolerate small
//! or empty areas, and avoid descriptor loading or terminal lifecycle changes.
//! The initial shell will reserve space for navigation, inspection, and status;
//! task lists and detail widgets belong to subsequent browser work.
