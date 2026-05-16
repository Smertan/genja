//! Inventory models and helpers for Genja Core.
//!
//! This module defines the in-memory inventory model (hosts, groups, defaults),
//! plus helpers for building inventories and applying optional transforms.
//!
//! **Key points**
//! - Inventory is immutable from the public API. Use builders to construct it.
//! - Hosts and groups are stored in `CustomTreeMap` keyed by name.
//! - Defaults share the same fields as groups, minus `groups` and `defaults`.
//! - Transforms are applied lazily when accessing hosts, groups, or defaults.
//! - Builder methods are consistent across Host/Group/Defaults for shared fields; defaults
//!   intentionally omit `groups`.
//!
//! # Examples
//!
//! ## Build a minimal inventory
//! ```
//! use genja_core::inventory::{Host, Hosts, Inventory, BaseBuilderHost};
//!
//! let mut hosts = Hosts::new();
//! let host = Host::builder().hostname("10.0.0.1").build();
//! hosts.add_host("router1", host);
//!
//! let inventory = Inventory::builder().hosts(hosts).build();
//! assert_eq!(inventory.hosts().len(), 1);
//! ```
//!
//! ## Defaults
//! ```
//! use genja_core::inventory::Inventory;
//!
//! let inventory = Inventory::default();
//! assert_eq!(inventory.hosts().len(), 0);
//! ```
//!
//! ## Apply a transform
//! ```
//! use genja_core::inventory::{Inventory, TransformFunction};
//!
//! let transform = TransformFunction::new(|host, _options| host.clone());
//! let inventory = Inventory::builder().transform_function(transform).build();
//! let _ = inventory.hosts();
//! ```

mod connections;
mod models;
mod runtime;
mod transform;

pub use connections::*;
pub use models::*;
pub use runtime::*;
pub use transform::*;

#[cfg(test)]
include!("tests.rs");
