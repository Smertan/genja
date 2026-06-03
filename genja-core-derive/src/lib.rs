//! Procedural macros used by `genja-core`.
//!
//! `DerefMacro` and `DerefMutMacro` generate `Deref` and `DerefMut`
//! implementations for tuple-wrapper types. `Task` generates `TaskInfo` and
//! `SubTasks` implementations for task structs used by `genja-core`.
//!
//! `Task` does not implement the core `genja_core::task::Task` trait. Callers
//! still implement the async `start` method themselves.
//!
//! # Deref Example
//! ```
//! use genja_core_derive::{DerefMacro, DerefMutMacro};
//!
//! pub trait DerefTarget {
//!     type Target;
//! }
//!
//! pub type DefaultListTarget = Vec<String>;
//!
//! impl DerefTarget for DefaultsList {
//!     type Target = DefaultListTarget;
//! }
//!
//! #[derive(DerefMacro, DerefMutMacro, PartialEq)]
//! pub struct DefaultsList(DefaultListTarget);
//!
//! let mut defaults_list = DefaultsList(DefaultListTarget::new());
//!
//! defaults_list.push("default1".to_string());
//!
//! assert_eq!(defaults_list.as_ref(), vec!["default1".to_string()]);
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, Expr, ExprArray, ExprLit, FnArg, GenericArgument, ImplItem, ItemImpl, Lit,
    LitStr, PathArguments, ReturnType, Token, Type, TypePath, bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

/// Generates an implementation of the `Deref` trait for the given type.
///
/// This function is used as a procedural macro to automatically derive the `Deref` trait
/// for a struct. It creates an implementation that dereferences to the first field of the struct.
///
/// # Parameters
///
/// * `input`: A `TokenStream` representing the input tokens of the derive macro.
///
/// # Returns
///
/// A `TokenStream` containing the generated implementation of the `Deref` trait.
#[proc_macro_derive(DerefMacro)]
pub fn derive_deref(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    if let Err(error) = reject_generics(&input, "DerefMacro") {
        return error.to_compile_error().into();
    }
    if let Err(error) = require_tuple_wrapper(&input, "DerefMacro") {
        return error.to_compile_error().into();
    }

    let expanded = quote! {
        impl std::ops::Deref for #name {
            /*
            * Define the Target type. To ensure the correct implementation is
            * to specify `<#name as .. >` which results to the name of the
            * struct. Otherwise it will result in an **ambiguous error**
            * if only `DerefTarget::Target` is used.
            */
            type Target = <#name as DerefTarget>::Target; //

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
    TokenStream::from(expanded)
}

/// Generates an implementation of the `DerefMut` trait for the given type.
///
/// This function is used as a procedural macro to automatically derive the `DerefMut` trait
/// for a struct. It creates an implementation that allows mutable dereferencing to the first
/// field of the struct.
///
/// # Parameters
///
/// * `input`: A `TokenStream` representing the input tokens of the derive macro.
///
/// # Returns
///
/// A `TokenStream` containing the generated implementation of the `DerefMut` trait.
#[proc_macro_derive(DerefMutMacro)]
pub fn derive_deref_mut(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    if let Err(error) = reject_generics(&input, "DerefMutMacro") {
        return error.to_compile_error().into();
    }
    if let Err(error) = require_tuple_wrapper(&input, "DerefMutMacro") {
        return error.to_compile_error().into();
    }

    let expanded = quote! {
        impl std::ops::DerefMut for #name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derives task metadata and sub-task wiring for a struct.
///
/// This procedural macro generates implementations of `TaskInfo` and `SubTasks` traits
/// for structs that represent tasks in the task execution system. It validates the struct's
/// fields and generates appropriate getter methods and subtask collection logic.
///
/// This macro does **not** generate the core `genja_core::task::Task` implementation.
/// Users must still implement `Task` manually and provide the async `start()`
/// method that accepts `TaskRuntimeContext` and returns
/// `Result<HostTaskResult, TaskError>`.
///
/// The macro expects the struct to have:
/// - A `name` field of type `String` or `&'static str` (required)
/// - An optional `connection_plugin_name` field of type `String`, `&'static str`, `Option<String>`, or `Option<&'static str>`
/// - An optional `options` field of type `Option<serde_json::Value>`
/// - An optional `processor_names` field of type `Vec<String>`
/// - Or a struct-level `#[task(processors = ["processor_name"])]` attribute
/// - Zero or more fields marked with `#[task(subtask)]` using a supported `Arc<dyn Task>` form:
///   `Arc<dyn Task>`, `std::sync::Arc<dyn Task>`, `Arc<dyn Task + Send + Sync>`,
///   or `std::sync::Arc<dyn Task + Send + Sync>`
///
/// After deriving, the generated behavior is:
/// - `name()` reads from the struct's `name` field
/// - `connection_plugin_name()` reads from `connection_plugin_name` if present, otherwise returns `None`
/// - `options()` returns the `options` field if present, otherwise `None`
/// - `processor_names()` returns the configured processor names if present, otherwise an empty vector
/// - `with_processor()` and `with_processors()` are generated when `processor_names` is present
/// - `sub_tasks()` returns all fields marked with `#[task(subtask)]` in declaration order
/// - `get_connection_key(hostname)` builds a `ConnectionKey` from `hostname` and `connection_plugin_name()` when a connection plugin is set
///
/// # Parameters
///
/// * `input` - A `TokenStream` representing the input tokens of the derive macro, containing
///   the struct definition for which `TaskInfo` and `SubTasks` should be generated.
///
/// # Returns
///
/// A `TokenStream` containing the generated implementations of `TaskInfo` and `SubTasks` traits.
/// Returns a compile error if:
/// - The macro is applied to a non-struct type
/// - The struct doesn't have named fields
/// - The struct has generic parameters, lifetimes, or a where clause
/// - Required fields are missing or have incorrect types
/// - Unknown `#[task(...)]` helper attributes are used
/// - Subtask fields are not one of the supported `Arc<dyn Task>` forms
///
/// # Examples
///
/// ```ignore
/// use async_trait::async_trait;
/// use std::sync::Arc;
/// use genja_core::inventory::Host;
/// use genja_core::task::{
///     HostTaskResult, SubTasks, Task, TaskError, TaskInfo, TaskRuntimeContext, TaskSuccess,
/// };
/// use genja_core_derive::Task as TaskDerive;
///
/// #[derive(TaskDerive)]
/// struct ChildTask {
///     name: &'static str,
/// }
///
/// #[async_trait]
/// impl Task for ChildTask {
///     async fn start(
///         &self,
///         _host: &Host,
///         _context: &TaskRuntimeContext,
///     ) -> Result<HostTaskResult, TaskError> {
///         Ok(HostTaskResult::passed(TaskSuccess::new()))
///     }
/// }
///
/// #[derive(TaskDerive)]
/// #[task(processors = ["audit"])]
/// struct MyTask {
///     name: String,
///     connection_plugin_name: Option<String>,
///     options: Option<serde_json::Value>,
///     #[task(subtask)]
///     child_task: Arc<dyn Task>,
/// }
///
/// let task = MyTask {
///     name: "deploy".to_string(),
///     connection_plugin_name: Some("ssh".to_string()),
///     options: Some(serde_json::json!({"dry_run": true})),
///     child_task: Arc::new(ChildTask { name: "validate" }),
/// };
///
/// assert_eq!(task.name(), "deploy");
/// assert_eq!(task.connection_plugin_name(), Some("ssh"));
/// assert_eq!(task.processor_names(), vec!["audit"]);
/// assert_eq!(task.sub_tasks()[0].name(), "validate");
/// ```
#[proc_macro_derive(Task, attributes(task))]
pub fn derive_task(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    if let Err(error) = reject_generics(&input, "Task") {
        return error.to_compile_error().into();
    }

    let attr_processor_names = match task_processor_attrs(&input.attrs) {
        Ok(processor_names) => processor_names,
        Err(error) => return error.to_compile_error().into(),
    };

    let data = match input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return syn::Error::new_spanned(name, "Task can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    let fields = match data.fields {
        syn::Fields::Named(fields) => fields.named,
        _ => {
            return syn::Error::new_spanned(name, "Task requires named fields")
                .to_compile_error()
                .into();
        }
    };

    let mut name_field = None;
    let mut connection_plugin_name_field = None;
    let mut options_field = None;
    let mut processor_names_field: Option<(syn::Ident, Type)> = None;
    let mut subtask_fields: Vec<(syn::Ident, SubtaskKind)> = Vec::new();

    for field in fields.iter() {
        let ident = match &field.ident {
            Some(ident) => ident,
            None => continue,
        };

        let has_subtask = match has_subtask_attr(&field.attrs) {
            Ok(has_subtask) => has_subtask,
            Err(error) => return error.to_compile_error().into(),
        };

        if has_subtask {
            match subtask_kind(&field.ty) {
                Some(kind) => subtask_fields.push((ident.clone(), kind)),
                None => {
                    return syn::Error::new_spanned(
                        &field.ty,
                        "subtask fields must be `Arc<dyn Task>`, `std::sync::Arc<dyn Task>`, or `Arc<dyn Task + Send + Sync>`",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }

        match ident.to_string().as_str() {
            "name" => name_field = Some(field.ty.clone()),
            "connection_plugin_name" => connection_plugin_name_field = Some(field.ty.clone()),
            "options" => options_field = Some(field.ty.clone()),
            "processor_names" => {
                processor_names_field = Some((ident.clone(), field.ty.clone()));
            }
            "processors" => {
                return syn::Error::new_spanned(
                    ident,
                    "use `processor_names: Vec<String>` to select processor plugins by name",
                )
                .to_compile_error()
                .into();
            }
            _ => {}
        }
    }

    let name_ty = match name_field {
        Some(ty) => ty,
        None => {
            return syn::Error::new_spanned(
                name,
                "Task requires a `name` field of type `String` or `&'static str`",
            )
            .to_compile_error()
            .into();
        }
    };

    if !is_string_type(&name_ty) && !is_static_str_type(&name_ty) {
        return syn::Error::new_spanned(name_ty, "`name` must be `String` or `&'static str`")
            .to_compile_error()
            .into();
    }

    let connection_plugin_name_ty = connection_plugin_name_field.clone();
    if let Some(ty) = &connection_plugin_name_ty
        && !is_string_or_static_str(ty)
        && !is_option_of(ty, is_string_or_static_str)
    {
        return syn::Error::new_spanned(
            ty,
            "`connection_plugin_name` must be `String`, `&'static str`, `Option<String>`, or `Option<&'static str>`",
        )
        .to_compile_error()
        .into();
    }

    if let Some(options_ty) = &options_field
        && !is_option_of(options_ty, is_value_type)
    {
        return syn::Error::new_spanned(
            options_ty,
            "`options` must be `Option<serde_json::Value>`",
        )
        .to_compile_error()
        .into();
    }

    if let Some((_, processor_names_ty)) = &processor_names_field {
        if !attr_processor_names.is_empty() {
            return syn::Error::new_spanned(
                processor_names_ty,
                "use either `processor_names: Vec<String>` or `#[task(processors = [...])]`, not both",
            )
            .to_compile_error()
            .into();
        }

        if !is_vec_of(processor_names_ty, is_string_type) {
            return syn::Error::new_spanned(
                processor_names_ty,
                "`processor_names` must be `Vec<String>`",
            )
            .to_compile_error()
            .into();
        }
    }

    let name_getter = if is_string_type(&name_ty) {
        quote! { self.name.as_str() }
    } else {
        quote! { self.name }
    };

    let connection_plugin_name_getter = match connection_plugin_name_ty {
        Some(ty) if is_string_type(&ty) => quote! {
            if self.connection_plugin_name.trim().is_empty() {
                None
            } else {
                Some(self.connection_plugin_name.as_str())
            }
        },
        Some(ty) if is_static_str_type(&ty) => quote! {
            if self.connection_plugin_name.trim().is_empty() {
                None
            } else {
                Some(self.connection_plugin_name)
            }
        },
        Some(_) => quote! {
            self.connection_plugin_name
                .as_deref()
                .filter(|plugin_name| !plugin_name.trim().is_empty())
        },
        None => quote! { None },
    };

    let options_getter = if options_field.is_some() {
        quote! { self.options.as_ref() }
    } else {
        quote! { None }
    };

    let processor_names_getter = match processor_names_field.as_ref() {
        Some((ident, _)) => quote! { self.#ident.iter().map(String::as_str).collect() },
        None if !attr_processor_names.is_empty() => quote! { vec![#(#attr_processor_names),*] },
        None => quote! { Vec::new() },
    };

    let processor_setters = match processor_names_field.as_ref() {
        Some((ident, _)) => quote! {
            impl #name {
                pub fn with_processor(mut self, processor_name: impl Into<String>) -> Self {
                    self.#ident.push(processor_name.into());
                    self
                }

                pub fn with_processors<I, S>(mut self, processor_names: I) -> Self
                where
                    I: IntoIterator<Item = S>,
                    S: Into<String>,
                {
                    self.#ident.extend(processor_names.into_iter().map(Into::into));
                    self
                }
            }
        },
        None => quote! {},
    };

    // Generates token streams for pushing subtask fields into a task vector.
    //
    // This function creates a vector of `proc_macro2::TokenStream` objects, where each
    // token stream represents a statement that pushes a subtask field (wrapped in `Arc<dyn Task>`)
    // into a `tasks` vector. If there are no subtask fields, an empty vector is returned.
    //
    // # Parameters
    //
    // * `subtask_fields` - A slice of tuples containing the field identifier and its subtask kind.
    //                      Each tuple represents a field marked with the `#[task(subtask)]` attribute.
    //
    // # Returns
    //
    // A `Vec<proc_macro2::TokenStream>` containing the generated push statements for each subtask field.
    // Returns an empty vector if `subtask_fields` is empty.
    let subtask_pushes = if subtask_fields.is_empty() {
        Vec::new()
    } else {
        let mut pushes = Vec::new();
        for (ident, _kind) in subtask_fields {
            pushes.push(quote! { tasks.push(self.#ident.clone()); });
        }
        pushes
    };

    let expanded = quote! {
        impl genja_core::task::TaskInfo for #name {
            fn name(&self) -> &str {
                #name_getter
            }

            fn connection_plugin_name(&self) -> Option<&str> {
                #connection_plugin_name_getter
            }

            fn get_connection_key(
                &self,
                hostname: &str,
            ) -> Option<genja_core::inventory::ConnectionKey> {
                self.connection_plugin_name().map(|plugin_name| {
                    genja_core::inventory::ConnectionKey::new(hostname, plugin_name)
                })
            }

            fn options(&self) -> Option<&serde_json::Value> {
                #options_getter
            }

            fn processor_names(&self) -> Vec<&str> {
                #processor_names_getter
            }
        }

        /// Implementation of the `SubTasks` trait for the derived type.
        ///
        /// This implementation collects all fields marked with the `#[task(subtask)]` attribute
        /// and returns them as a vector of `Arc<dyn Task>`. This allows the task system to
        /// traverse and execute subtasks in a hierarchical manner.
        ///
        /// # Returns
        ///
        /// A `Vec<std::sync::Arc<dyn genja_core::task::Task>>` containing all subtasks
        /// associated with this task instance.
        impl genja_core::task::SubTasks for #name {
            fn sub_tasks(&self) -> Vec<std::sync::Arc<dyn genja_core::task::Task>> {
                let mut tasks: Vec<std::sync::Arc<dyn genja_core::task::Task>> = Vec::new();
                #(#subtask_pushes)*
                tasks
            }
        }

        #processor_setters
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn genja_task(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as GenjaTaskArgs);
    let item_impl = parse_macro_input!(input as ItemImpl);

    match expand_genja_task(args, item_impl) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[derive(Default)]
struct GenjaTaskArgs {
    name: Option<LitStr>,
    connection_plugin_name: Option<LitStr>,
    processors: Vec<LitStr>,
}

impl Parse for GenjaTaskArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut args = Self::default();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "name" => {
                    if args.name.is_some() {
                        return Err(syn::Error::new_spanned(key, "duplicate `name`"));
                    }
                    args.name = Some(input.parse()?);
                }
                "connection_plugin_name" => {
                    if args.connection_plugin_name.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate `connection_plugin_name`",
                        ));
                    }
                    args.connection_plugin_name = Some(input.parse()?);
                }
                "processors" => {
                    if !args.processors.is_empty() {
                        return Err(syn::Error::new_spanned(key, "duplicate `processors`"));
                    }
                    let array: ExprArray = input.parse()?;
                    args.processors = parse_processor_exprs(&array)?;
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unsupported key; expected `name`, `connection_plugin_name`, or `processors`",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        if args.name.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "`name = \"...\"` is required",
            ));
        }

        Ok(args)
    }
}

fn expand_genja_task(args: GenjaTaskArgs, item_impl: ItemImpl) -> syn::Result<proc_macro2::TokenStream> {
    if item_impl.trait_.is_some() {
        return Err(syn::Error::new_spanned(
            &item_impl.self_ty,
            "`#[genja_task(...)]` can only be applied to inherent impl blocks",
        ));
    }

    if !item_impl.generics.params.is_empty() || item_impl.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &item_impl.generics,
            "`genja_task` does not support generic parameters or where clauses",
        ));
    }

    let self_ty = &item_impl.self_ty;
    let mut has_start = false;
    let mut has_start_async = false;
    let mut has_options = false;
    let mut has_sub_tasks = false;

    for item in &item_impl.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        match method.sig.ident.to_string().as_str() {
            "start" => {
                validate_start_method(method, false)?;
                has_start = true;
            }
            "start_async" => {
                validate_start_method(method, true)?;
                has_start_async = true;
            }
            "options" => {
                validate_options_method(method)?;
                has_options = true;
            }
            "sub_tasks" => {
                validate_sub_tasks_method(method)?;
                has_sub_tasks = true;
            }
            _ => {}
        }
    }

    if has_start == has_start_async {
        return Err(syn::Error::new_spanned(
            &item_impl.self_ty,
            if has_start {
                "define exactly one of `fn start(...)` or `async fn start_async(...)`"
            } else {
                "define one of `fn start(...)` or `async fn start_async(...)`"
            },
        ));
    }

    let name = args.name.expect("validated above");
    let connection_plugin_name = args.connection_plugin_name;
    let processors = args.processors;

    let connection_impl = match connection_plugin_name {
        Some(plugin_name) => quote! { Some(#plugin_name) },
        None => quote! { None },
    };

    let options_impl = if has_options {
        quote! {
            fn options(&self) -> Option<&serde_json::Value> {
                #self_ty::options(self)
            }
        }
    } else {
        quote! {}
    };

    let sub_tasks_impl = if has_sub_tasks {
        quote! {
            fn sub_tasks(&self) -> Vec<std::sync::Arc<dyn genja_core::task::Task>> {
                #self_ty::sub_tasks(self)
            }
        }
    } else {
        quote! {}
    };

    let processor_names_impl = if processors.is_empty() {
        quote! {}
    } else {
        quote! {
            fn processor_names(&self) -> Vec<&str> {
                vec![#(#processors),*]
            }
        }
    };

    let task_impl = if has_start {
        quote! {
            #[genja_core::async_trait]
            impl genja_core::task::Task for #self_ty {
                fn start(
                    &self,
                    host: &genja_core::inventory::Host,
                    context: &genja_core::task::BlockingTaskRuntimeContext,
                ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
                    #self_ty::start(self, host, context)
                }

                #sub_tasks_impl

                fn execution_mode(&self) -> genja_core::task::TaskExecutionMode {
                    genja_core::task::TaskExecutionMode::Blocking
                }
            }
        }
    } else {
        quote! {
            #[genja_core::async_trait]
            impl genja_core::task::Task for #self_ty {
                async fn start_async(
                    &self,
                    host: &genja_core::inventory::Host,
                    context: &genja_core::task::TaskRuntimeContext,
                ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
                    #self_ty::start_async(self, host, context).await
                }

                #sub_tasks_impl

                fn execution_mode(&self) -> genja_core::task::TaskExecutionMode {
                    genja_core::task::TaskExecutionMode::Async
                }
            }
        }
    };

    Ok(quote! {
        #item_impl

        impl genja_core::task::TaskInfo for #self_ty {
            fn name(&self) -> &str {
                #name
            }

            fn connection_plugin_name(&self) -> Option<&str> {
                #connection_impl
            }

            #options_impl

            #processor_names_impl
        }

        #task_impl
    })
}

fn reject_generics(input: &DeriveInput, macro_name: &str) -> syn::Result<()> {
    if input.generics.params.is_empty() && input.generics.where_clause.is_none() {
        return Ok(());
    }

    Err(syn::Error::new_spanned(
        &input.generics,
        format!("`{macro_name}` does not support generic parameters or where clauses"),
    ))
}

fn parse_processor_exprs(array: &ExprArray) -> syn::Result<Vec<LitStr>> {
    array
        .elems
        .iter()
        .map(|expr| match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Str(value),
                ..
            }) => Ok(value.clone()),
            _ => Err(syn::Error::new_spanned(
                expr,
                "`processors` must be an array of string literals",
            )),
        })
        .collect()
}

fn validate_start_method(method: &syn::ImplItemFn, is_async: bool) -> syn::Result<()> {
    if method.sig.asyncness.is_some() != is_async {
        let expected = if is_async {
            "`start_async` must be declared as `async fn`"
        } else {
            "`start` must be declared as `fn`, not `async fn`"
        };
        return Err(syn::Error::new_spanned(&method.sig.ident, expected));
    }

    validate_shared_method_shape(method)?;

    if method.sig.inputs.len() != 3 {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "task start methods must take `&self`, `host`, and `context`",
        ));
    }

    let mut inputs = method.sig.inputs.iter();
    validate_receiver(inputs.next().unwrap())?;
    validate_typed_arg(inputs.next().unwrap(), is_host_ref, "`host` must be `&Host`")?;
    validate_typed_arg(
        inputs.next().unwrap(),
        if is_async {
            is_async_context_ref
        } else {
            is_blocking_context_ref
        },
        if is_async {
            "`context` must be `&TaskRuntimeContext`"
        } else {
            "`context` must be `&BlockingTaskRuntimeContext`"
        },
    )?;

    validate_return_type(&method.sig.output, is_result_host_task_error, if is_async {
        "`start_async` must return `Result<HostTaskResult, TaskError>`"
    } else {
        "`start` must return `Result<HostTaskResult, TaskError>`"
    })
}

fn validate_options_method(method: &syn::ImplItemFn) -> syn::Result<()> {
    if method.sig.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            &method.sig.ident,
            "`options` must not be async",
        ));
    }

    validate_shared_method_shape(method)?;

    if method.sig.inputs.len() != 1 {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "`options` must take only `&self`",
        ));
    }

    validate_receiver(method.sig.inputs.first().unwrap())?;
    validate_return_type(
        &method.sig.output,
        is_option_value_ref,
        "`options` must return `Option<&serde_json::Value>`",
    )
}

fn validate_sub_tasks_method(method: &syn::ImplItemFn) -> syn::Result<()> {
    if method.sig.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            &method.sig.ident,
            "`sub_tasks` must not be async",
        ));
    }

    validate_shared_method_shape(method)?;

    if method.sig.inputs.len() != 1 {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "`sub_tasks` must take only `&self`",
        ));
    }

    validate_receiver(method.sig.inputs.first().unwrap())?;
    validate_return_type(
        &method.sig.output,
        is_vec_of_task_arcs,
        "`sub_tasks` must return `Vec<Arc<dyn Task>>`",
    )
}

fn validate_shared_method_shape(method: &syn::ImplItemFn) -> syn::Result<()> {
    if method.sig.constness.is_some()
        || method.sig.unsafety.is_some()
        || method.sig.abi.is_some()
        || method.sig.variadic.is_some()
        || !method.sig.generics.params.is_empty()
        || method.sig.generics.where_clause.is_some()
    {
        return Err(syn::Error::new_spanned(
            &method.sig,
            "Genja task hook methods cannot be const, unsafe, generic, extern, or variadic",
        ));
    }

    Ok(())
}

fn validate_receiver(arg: &FnArg) -> syn::Result<()> {
    match arg {
        FnArg::Receiver(receiver)
            if receiver.reference.is_some() && receiver.mutability.is_none() =>
        {
            Ok(())
        }
        _ => Err(syn::Error::new_spanned(arg, "first argument must be `&self`")),
    }
}

fn validate_typed_arg(
    arg: &FnArg,
    predicate: fn(&Type) -> bool,
    message: &str,
) -> syn::Result<()> {
    match arg {
        FnArg::Typed(typed) if predicate(&typed.ty) => Ok(()),
        FnArg::Typed(typed) => Err(syn::Error::new_spanned(&typed.ty, message)),
        FnArg::Receiver(_) => Err(syn::Error::new_spanned(arg, message)),
    }
}

fn validate_return_type(
    output: &ReturnType,
    predicate: fn(&Type) -> bool,
    message: &str,
) -> syn::Result<()> {
    match output {
        ReturnType::Type(_, ty) if predicate(ty) => Ok(()),
        ReturnType::Type(_, ty) => Err(syn::Error::new_spanned(ty, message)),
        ReturnType::Default => Err(syn::Error::new(proc_macro2::Span::call_site(), message)),
    }
}

fn is_result_host_task_error(ty: &Type) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };
    let Some(seg) = path.segments.last() else {
        return false;
    };
    if seg.ident != "Result" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return false;
    };
    if args.args.len() != 2 {
        return false;
    }

    let mut args_iter = args.args.iter();
    let ok = match args_iter.next() {
        Some(GenericArgument::Type(ty)) => type_ends_with(ty, "HostTaskResult"),
        _ => false,
    };
    let err = match args_iter.next() {
        Some(GenericArgument::Type(ty)) => type_ends_with(ty, "TaskError"),
        _ => false,
    };
    ok && err
}

fn is_option_value_ref(ty: &Type) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };
    let Some(seg) = path.segments.last() else {
        return false;
    };
    if seg.ident != "Option" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return false;
    };
    if args.args.len() != 1 {
        return false;
    }
    match args.args.first() {
        Some(GenericArgument::Type(Type::Reference(reference))) => {
            type_ends_with(&reference.elem, "Value")
        }
        _ => false,
    }
}

fn is_vec_of_task_arcs(ty: &Type) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };
    let Some(seg) = path.segments.last() else {
        return false;
    };
    if seg.ident != "Vec" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return false;
    };
    if args.args.len() != 1 {
        return false;
    }
    match args.args.first() {
        Some(GenericArgument::Type(inner)) => subtask_kind(inner).is_some(),
        _ => false,
    }
}

fn is_host_ref(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if type_ends_with(&reference.elem, "Host"))
}

fn is_async_context_ref(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if type_ends_with(&reference.elem, "TaskRuntimeContext"))
}

fn is_blocking_context_ref(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if type_ends_with(&reference.elem, "BlockingTaskRuntimeContext"))
}

fn type_ends_with(ty: &Type, ident: &str) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => path
            .segments
            .last()
            .map(|segment| segment.ident == ident)
            .unwrap_or(false),
        _ => false,
    }
}

fn require_tuple_wrapper(input: &DeriveInput, macro_name: &str) -> syn::Result<()> {
    match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Unnamed(fields) if !fields.unnamed.is_empty() => Ok(()),
            _ => Err(syn::Error::new_spanned(
                &input.ident,
                format!("`{macro_name}` requires a tuple struct with the wrapped value in field 0"),
            )),
        },
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            format!("`{macro_name}` can only be derived for tuple structs"),
        )),
    }
}

fn is_string_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => path
            .segments
            .last()
            .map(|seg| seg.ident == "String")
            .unwrap_or(false),
        _ => false,
    }
}

fn is_static_str_type(ty: &Type) -> bool {
    match ty {
        Type::Reference(reference) => {
            if let Some(lifetime) = &reference.lifetime {
                if lifetime.ident != "static" {
                    return false;
                }
            } else {
                return false;
            }
            matches!(&*reference.elem, Type::Path(TypePath { path, .. }) if path.segments.last().map(|seg| seg.ident == "str").unwrap_or(false))
        }
        _ => false,
    }
}

fn is_option_of(ty: &Type, inner_check: fn(&Type) -> bool) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let seg = match path.segments.last() {
                Some(seg) => seg,
                None => return false,
            };
            if seg.ident != "Option" {
                return false;
            }
            match &seg.arguments {
                PathArguments::AngleBracketed(args) => args
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    })
                    .any(inner_check),
                _ => false,
            }
        }
        _ => false,
    }
}

fn is_vec_of(ty: &Type, inner_check: fn(&Type) -> bool) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let seg = match path.segments.last() {
                Some(seg) => seg,
                None => return false,
            };
            if seg.ident != "Vec" {
                return false;
            }
            match &seg.arguments {
                PathArguments::AngleBracketed(args) => args
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    })
                    .any(inner_check),
                _ => false,
            }
        }
        _ => false,
    }
}

fn is_value_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let mut segments = path.segments.iter();
            let last = segments.next_back().map(|seg| seg.ident.to_string());
            let second_last = segments.next_back().map(|seg| seg.ident.to_string());

            matches!(
                (second_last.as_deref(), last.as_deref()),
                (Some("serde_json"), Some("Value")) | (None, Some("Value"))
            )
        }
        _ => false,
    }
}

fn is_string_or_static_str(ty: &Type) -> bool {
    is_string_type(ty) || is_static_str_type(ty)
}

struct ProcessorList {
    names: Punctuated<LitStr, Token![,]>,
}

impl Parse for ProcessorList {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);
        Ok(Self {
            names: content.parse_terminated(|input| input.parse::<LitStr>(), Token![,])?,
        })
    }
}

fn task_processor_attrs(attrs: &[syn::Attribute]) -> syn::Result<Vec<syn::LitStr>> {
    let mut processor_names = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("task") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("processors") {
                return Err(meta.error(
                    "unsupported struct-level `task` attribute; expected `processors = [...]`",
                ));
            }

            let value = meta.value()?;
            let processor_list = value.parse::<ProcessorList>()?;
            for processor_name in processor_list.names {
                processor_names.push(processor_name);
            }

            Ok(())
        })?;
    }

    Ok(processor_names)
}

#[derive(Copy, Clone)]
enum SubtaskKind {
    SingleArc,
}

fn has_subtask_attr(attrs: &[syn::Attribute]) -> syn::Result<bool> {
    let mut has_subtask = false;

    for attr in attrs {
        if !attr.path().is_ident("task") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("subtask") {
                has_subtask = true;
                return Ok(());
            }

            Err(meta.error("unsupported field-level `task` attribute; expected `subtask`"))
        })?;
    }

    Ok(has_subtask)
}

fn subtask_kind(ty: &Type) -> Option<SubtaskKind> {
    if is_arc_task(ty) {
        return Some(SubtaskKind::SingleArc);
    }
    None
}

fn is_arc_task(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let seg = match path.segments.last() {
                Some(seg) => seg,
                None => return false,
            };
            if seg.ident != "Arc" {
                return false;
            }
            match &seg.arguments {
                PathArguments::AngleBracketed(args) => args
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    })
                    .any(is_task_trait_object),
                _ => false,
            }
        }
        _ => false,
    }
}

fn is_task_trait_object(ty: &Type) -> bool {
    match ty {
        Type::TraitObject(obj) => obj.bounds.iter().any(|bound| match bound {
            syn::TypeParamBound::Trait(trait_bound) => trait_bound
                .path
                .segments
                .last()
                .map(|seg| seg.ident == "Task")
                .unwrap_or(false),
            _ => false,
        }),
        _ => false,
    }
}
