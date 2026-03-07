use libloading::Library;
use serde::Deserialize;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;

use genja_core::inventory::{Hosts, TransformFunction};
use genja_core::task::{Task, Tasks};
pub type PathString = String;
pub type GroupOrName = String;
pub type PluginName = String;
pub type PluginResult = Result<(Library, Vec<Box<dyn Plugin>>), Box<dyn std::error::Error>>;
pub type PluginCreate = unsafe fn() -> Vec<Box<dyn Plugin>>;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum PluginEntry {
    Individual(PathString),
    Group(HashMap<String, PathString>),
}

/// Information about a loaded plugin, including the plugin itself and its group.
pub struct PluginInfo {
    pub plugin: Box<dyn Plugin>,
    pub group: Option<String>,
}

/// Manages the lifecycle of loaded plugins.
pub struct PluginManager {
    pub plugins: HashMap<String, PluginInfo>,
    // plugin_path: Vec<String>
    pub plugin_path: Vec<HashMap<GroupOrName, PluginEntry>>,
}

pub trait Plugin: Send + Sync + Any {
    /// The name of the plugin. This is used to identify the plugin and
    /// to associate it with the context.
    fn name(&self) -> String;

    /// Executes a single function with the provided context.
    fn execute(&self, context: &dyn Any) -> Result<(), Box<dyn std::error::Error>>;

    /// Returns the group name
    fn group(&self) -> String {
        String::from("BasePlugin")
    }
}

pub trait PluginInventory: Plugin {
    // loads the inventory
    fn load(&self);

    /// Returns the group name
    fn group(&self) -> String {
        String::from("InventoryPlugin")
    }
}

impl Debug for dyn Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {{ name: {} }}", Plugin::group(self), self.name())
    }
}

impl Debug for dyn PluginInventory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {{ name: {} }}",
            PluginInventory::group(self),
            self.name()
        )
    }
}

impl Debug for dyn PluginConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {{ name: {} }}",
            PluginConnection::group(self),
            self.name()
        )
    }
}

pub trait PluginRunner: Plugin {
    // Run a single task
    fn run(&self, task: Task, hosts: &Hosts);

    // Run all tasks in the task vec
    fn run_tasks(&self, tasks: Tasks, hosts: &Hosts);

    /// Returns the group name
    fn group(&self) -> String {
        String::from("RunnerPlugin")
    }
}

impl Debug for dyn PluginRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {{ name: {} }}",
            PluginRunner::group(self),
            self.name()
        )
    }
}

pub trait PluginTransformFunction: Plugin {
    /// Returns a transform function instance for inventory processing.
    fn transform_function(&self) -> TransformFunction;

    /// Returns the group name
    fn group(&self) -> String {
        String::from("TransformFunctionPlugin")
    }
}

impl Debug for dyn PluginTransformFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {{ name: {} }}",
            PluginTransformFunction::group(self),
            self.name()
        )
    }
}
pub trait PluginConnection: Plugin {
    // Open a connection to a device
    fn open(&self);

    // Close a connection to a device
    fn close(&self);

    // Run all tasks in the task vec
    fn connection(&self);

    /// Returns the group name
    fn group(&self) -> String {
        String::from("ConnectionPlugin")
    }
}
#[derive(Debug)]
pub enum Plugins {
    Connection(Box<dyn PluginConnection>),
    Inventory(Box<dyn PluginInventory>),
    Runner(Box<dyn PluginRunner>),
    TransformFunction(Box<dyn PluginTransformFunction>),
}

impl Plugins {
    pub fn name(&self) -> String {
        match self {
            Plugins::Connection(connection) => connection.name(),
            Plugins::Inventory(inventory) => inventory.name(),
            Plugins::Runner(runner) => runner.name(),
            Plugins::TransformFunction(transform) => transform.name(),
        }
    }

    pub fn group_name(&self) -> String {
        match self {
            Plugins::Connection(_) => String::from("Connection"),
            Plugins::Inventory(_) => String::from("Inventory"),
            Plugins::Runner(_) => String::from("Runner"),
            Plugins::TransformFunction(_) => String::from("TransformFunction"),
        }
    }

    pub fn execute(&self, context: &dyn Any) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Plugins::Connection(connection) => connection.execute(context),
            Plugins::Inventory(inventory) => inventory.execute(context),
            Plugins::Runner(runner) => runner.execute(context),
            Plugins::TransformFunction(transform) => transform.execute(context),
        }
    }
}
