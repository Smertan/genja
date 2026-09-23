//! Terminal-independent task browser state boundary.
//!
//! State will hold loaded descriptors, an optional selected index, reserved
//! filter text, the active panel, and discovery errors. Selection must remain
//! valid for the descriptor snapshot, with no selection for an empty snapshot.
//! Full-screen quit state belongs to the host app rather than the browser.
