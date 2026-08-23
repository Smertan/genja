//! Procedural macros used by `genja-core`.
//!
//! `DerefMacro` and `DerefMutMacro` generate `Deref` and `DerefMut`
//! implementations for tuple-wrapper types.
//!
//! `genja_task` is the public task-authoring macro. It generates both
//! `TaskInfo` and `Task` implementations from an inherent `impl` block, submits
//! local discovery metadata to the compiled task registry, and infers execution
//! mode from `fn start(...)` versus `async fn start_async(...)`. The optional
//! `registration(...)` block gives a task a stable catalog ID and local JSON
//! construction factory for CLI, MCP, provider manifest, and catalog workflows.
//!
//! # Task Authoring Example
//! ```ignore
//! use genja_core::genja_task;
//! use genja_core::inventory::Host;
//! use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};
//!
//! struct CollectFacts;
//!
//! #[genja_task(
//!     name = "collect_facts",
//!     connection_plugin_name = "ssh",
//!     processors = ["audit"],
//!     retry(
//!         allow = true,
//!         max_attempts = 3,
//!         delay_ms = 500
//!     ),
//! )]
//! impl CollectFacts {
//!     async fn start_async(
//!         &self,
//!         host: &Host,
//!         _context: &TaskRuntimeContext,
//!     ) -> Result<HostTaskResult, genja_core::task::TaskError> {
//!         Ok(HostTaskResult::passed(
//!             TaskSuccess::new().with_summary(format!(
//!                 "collected facts for {:?}",
//!                 host.hostname()
//!             )),
//!         ))
//!     }
//! }
//! ```
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
    LitBool, LitInt, LitStr, Path, PathArguments, ReturnType, Token, Type, TypePath, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, token,
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

/// Generate `TaskInfo`, `Task`, and local discovery metadata for an inherent
/// task `impl` block.
///
/// The macro requires `name = "..."` and exactly one task entrypoint:
/// `fn start(...)` for blocking tasks or `async fn start_async(...)` for async
/// tasks. Optional metadata includes:
///
/// - `connection_plugin_name = "..."`; defaults to no task-scoped connection.
/// - `processors = ["..."]`; defaults to no processors.
/// - `retry(allow = ..., max_attempts = ..., delay_ms = ...)`; omitted fields
///   fall back to runner retry defaults, then built-in retry defaults.
/// - `idempotency = IdempotencyMode::...`; defaults to
///   `IdempotencyMode::Disabled`.
/// - `supports_dry_run = true`; defaults to `false`.
/// - `session_verification(max_attempts = ..., delay_ms = ...)`; defaults to
///   disabled when the block is absent. Inside the block, omitted
///   `max_attempts` defaults to `1` and omitted `delay_ms` defaults to `0`.
/// - `registration(id = "...", version = "...", description = "...")`; opts
///   into stable task registration and JSON construction. `id` is required and
///   `version` defaults to `env!("CARGO_PKG_VERSION")`. Use
///   `schema = "schemars"` to include a JSON Schema for the task input in the
///   descriptor. Schema generation requires the task type, and any nested field
///   types, to implement `schemars::JsonSchema`.
///
/// # Discovery and registration
///
/// Every annotated task is submitted to the compiled task registry. Tasks
/// without `registration(...)` use a generated `auto:...` ID derived from the
/// Rust type path. Generated IDs are useful for local listing, but they are not
/// stable public contracts and are not constructible from JSON input.
///
/// Tasks with `registration(id = "...")` use an explicit stable ID. Their
/// public identity is `<task-id>@<task-version>`, for example
/// `acme.network.configure_acl@2.0.0`. Explicit IDs are validated as
/// namespace-friendly lowercase identifiers and versions are validated as
/// semantic versions. Duplicate `id + version` registrations are rejected when
/// the compiled registry is built.
///
/// Registered tasks support three construction strategies:
///
/// - Omit `factory` or use `factory = "serde"` to deserialize JSON input into
///   the task type. The task type must implement `serde::de::DeserializeOwned`.
/// - Use `factory = "default"` for no-input tasks. The task type must implement
///   `Default`; input must be `null` or `{}`.
/// - Use `factory = custom(path::to::function)` for advanced input
///   preparation. The function receives `serde_json::Value` and must return
///   `Result<Self, genja_core::task::TaskRegistrationError>`, where `Self` is
///   the task type. Custom factories are intended for validation,
///   normalization, shorthand expansion, and carefully controlled
///   de-obfuscation before constructing the task. Error messages should avoid
///   exposing raw or decoded secret values.
///
/// With custom factories, `schema = "schemars"` describes `Self`. If the custom
/// factory accepts a public JSON shape that differs from the task struct, omit
/// schema generation for now or keep the custom input contract documented
/// separately until a dedicated input-type schema option is added.
///
/// `session_verification(...)` requires `connection_plugin_name = "..."`,
/// because post-change session verification must replace a declared task
/// connection.
///
/// ```ignore
/// use genja_core::genja_task;
/// use genja_core::inventory::Host;
/// use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};
///
/// struct ReplaceManagementAcl;
///
/// #[genja_task(
///     name = "replace_management_acl",
///     connection_plugin_name = "ssh",
///     session_verification(
///         max_attempts = 3,
///         delay_ms = 5000
///     )
/// )]
/// impl ReplaceManagementAcl {
///     async fn start_async(
///         &self,
///         _host: &Host,
///         _context: &TaskRuntimeContext,
///     ) -> Result<HostTaskResult, genja_core::task::TaskError> {
///         Ok(HostTaskResult::passed(TaskSuccess::new().with_changed(true)))
///     }
/// }
/// ```
///
/// The default registered-task path uses serde construction. This is the
/// common path for tasks that should be created from user-provided JSON input.
///
/// ```ignore
/// use genja_core::genja_task;
/// use genja_core::inventory::Host;
/// use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};
///
/// #[derive(serde::Deserialize)]
/// struct BackupConfig {
///     backup_path: String,
///     compress: bool,
/// }
///
/// #[genja_task(
///     name = "backup_config",
///     connection_plugin_name = "ssh",
///     registration(id = "acme.backup.backup_config")
/// )]
/// impl BackupConfig {
///     async fn start_async(
///         &self,
///         _host: &Host,
///         _context: &TaskRuntimeContext,
///     ) -> Result<HostTaskResult, genja_core::task::TaskError> {
///         Ok(HostTaskResult::passed(
///             TaskSuccess::new().with_summary(format!(
///                 "backing up to {}",
///                 self.backup_path
///             )),
///         ))
///     }
/// }
/// ```
///
/// ```ignore
/// use genja_core::genja_task;
/// use genja_core::inventory::Host;
/// use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};
///
/// #[derive(serde::Deserialize, schemars::JsonSchema)]
/// struct ConfigureAcl {
///     acl_name: String,
/// }
///
/// #[genja_task(
///     name = "configure_acl",
///     registration(
///         id = "acme.network.configure_acl",
///         version = "2.0.0",
///         description = "Configures an ACL on a network device",
///         schema = "schemars"
///     )
/// )]
/// impl ConfigureAcl {
///     async fn start_async(
///         &self,
///         _host: &Host,
///         _context: &TaskRuntimeContext,
///     ) -> Result<HostTaskResult, genja_core::task::TaskError> {
///         Ok(HostTaskResult::passed(TaskSuccess::new()))
///     }
/// }
/// ```
///
/// No-input tasks can use the default factory. The task type must implement
/// `Default`, and callers must pass `null` or `{}` as construction input.
///
/// ```ignore
/// use genja_core::genja_task;
/// use genja_core::inventory::Host;
/// use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};
///
/// #[derive(Default)]
/// struct CollectFacts;
///
/// #[genja_task(
///     name = "collect_facts",
///     registration(
///         id = "acme.inventory.collect_facts",
///         factory = "default"
///     )
/// )]
/// impl CollectFacts {
///     async fn start_async(
///         &self,
///         _host: &Host,
///         _context: &TaskRuntimeContext,
///     ) -> Result<HostTaskResult, genja_core::task::TaskError> {
///         Ok(HostTaskResult::passed(TaskSuccess::new()))
///     }
/// }
/// ```
///
/// Custom factories can keep the public JSON contract separate from the
/// internal Rust struct. This is useful when task input needs preparation before
/// execution, such as expanding shorthand or decoding obfuscated values.
///
/// ```ignore
/// use genja_core::genja_task;
/// use genja_core::inventory::Host;
/// use genja_core::task::{
///     HostTaskResult, TaskRegistrationError, TaskRuntimeContext, TaskSuccess,
/// };
///
/// struct ConfigureAcl {
///     acl_name: String,
///     secret_token: String,
/// }
///
/// fn create_configure_acl(
///     input: serde_json::Value,
/// ) -> Result<ConfigureAcl, TaskRegistrationError> {
///     let acl_name = input
///         .get("acl")
///         .and_then(serde_json::Value::as_str)
///         .ok_or_else(|| TaskRegistrationError::InvalidInput {
///             id: "acme.network.configure_acl".to_string(),
///             version: "2.0.0".to_string(),
///             message: "`acl` is required".to_string(),
///         })?;
///     let token = input
///         .get("token_obfuscated")
///         .and_then(serde_json::Value::as_str)
///         .ok_or_else(|| TaskRegistrationError::InvalidInput {
///             id: "acme.network.configure_acl".to_string(),
///             version: "2.0.0".to_string(),
///             message: "`token_obfuscated` is required".to_string(),
///         })?;
///
///     Ok(ConfigureAcl {
///         acl_name: acl_name.to_string(),
///         secret_token: token.chars().rev().collect(),
///     })
/// }
///
/// #[genja_task(
///     name = "configure_acl",
///     registration(
///         id = "acme.network.configure_acl",
///         version = "2.0.0",
///         factory = custom(create_configure_acl)
///     )
/// )]
/// impl ConfigureAcl {
///     async fn start_async(
///         &self,
///         _host: &Host,
///         _context: &TaskRuntimeContext,
///     ) -> Result<HostTaskResult, genja_core::task::TaskError> {
///         Ok(HostTaskResult::passed(TaskSuccess::new()))
///     }
/// }
/// ```
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
    supports_dry_run: Option<LitBool>,
    idempotency: Option<IdempotencyModeArg>,
    processors: Vec<LitStr>,
    retry: Option<RetryArgs>,
    registration: Option<RegistrationArgs>,
    session_verification: Option<SessionVerificationArgs>,
}

#[derive(Clone, Copy)]
enum IdempotencyModeArg {
    Disabled,
    Check,
    CheckAndVerify,
}

impl IdempotencyModeArg {
    fn requires_check(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

#[derive(Default)]
struct RetryArgs {
    allow: Option<LitBool>,
    max_attempts: Option<LitInt>,
    delay_ms: Option<LitInt>,
}

#[derive(Default)]
struct RegistrationArgs {
    id: Option<LitStr>,
    version: Option<LitStr>,
    description: Option<LitStr>,
    factory: Option<RegistrationFactoryArg>,
    schema: Option<RegistrationSchemaArg>,
}

enum RegistrationFactoryArg {
    Serde,
    Default,
    Custom(Path),
}

enum RegistrationSchemaArg {
    Schemars,
}

#[derive(Default)]
struct SessionVerificationArgs {
    max_attempts: Option<LitInt>,
    delay_ms: Option<LitInt>,
}

impl Parse for GenjaTaskArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut args = Self::default();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;

            match key.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    if args.name.is_some() {
                        return Err(syn::Error::new_spanned(key, "duplicate `name`"));
                    }
                    args.name = Some(input.parse()?);
                }
                "connection_plugin_name" => {
                    input.parse::<Token![=]>()?;
                    if args.connection_plugin_name.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate `connection_plugin_name`",
                        ));
                    }
                    args.connection_plugin_name = Some(input.parse()?);
                }
                "supports_dry_run" => {
                    input.parse::<Token![=]>()?;
                    if args.supports_dry_run.is_some() {
                        return Err(syn::Error::new_spanned(key, "duplicate `supports_dry_run`"));
                    }
                    args.supports_dry_run = Some(input.parse()?);
                }
                "idempotency" => {
                    input.parse::<Token![=]>()?;
                    if args.idempotency.is_some() {
                        return Err(syn::Error::new_spanned(key, "duplicate `idempotency`"));
                    }
                    let expr: Expr = input.parse()?;
                    args.idempotency = Some(parse_idempotency_mode_expr(&expr)?);
                }
                "processors" => {
                    input.parse::<Token![=]>()?;
                    if !args.processors.is_empty() {
                        return Err(syn::Error::new_spanned(key, "duplicate `processors`"));
                    }
                    let array: ExprArray = input.parse()?;
                    args.processors = parse_processor_exprs(&array)?;
                }
                "allow_retries" => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unsupported key `allow_retries`; did you mean `retry(allow = ...)`?",
                    ));
                }
                "max_task_attempts" => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unsupported key `max_task_attempts`; did you mean `retry(max_attempts = ...)`?",
                    ));
                }
                "retry" => {
                    if args.retry.is_some() {
                        return Err(syn::Error::new_spanned(key, "duplicate `retry`"));
                    }
                    let content;
                    parenthesized!(content in input);
                    args.retry = Some(content.parse()?);
                }
                "registration" => {
                    if args.registration.is_some() {
                        return Err(syn::Error::new_spanned(key, "duplicate `registration`"));
                    }
                    let content;
                    parenthesized!(content in input);
                    args.registration = Some(content.parse()?);
                }
                "session_verification" => {
                    if args.session_verification.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate `session_verification`",
                        ));
                    }
                    let content;
                    parenthesized!(content in input);
                    args.session_verification = Some(content.parse()?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unsupported key; expected `name`, `connection_plugin_name`, `supports_dry_run`, `idempotency`, `processors`, `retry(...)`, `registration(...)`, or `session_verification(...)`",
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

fn parse_idempotency_mode_expr(expr: &Expr) -> syn::Result<IdempotencyModeArg> {
    let Expr::Path(path) = expr else {
        return Err(syn::Error::new_spanned(
            expr,
            "`idempotency` must use IdempotencyMode::Disabled, IdempotencyMode::Check, or IdempotencyMode::CheckAndVerify",
        ));
    };
    let Some(variant) = path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            expr,
            "`idempotency` must use IdempotencyMode::Disabled, IdempotencyMode::Check, or IdempotencyMode::CheckAndVerify",
        ));
    };
    let Some(mode_type) = path.path.segments.iter().rev().nth(1) else {
        return Err(syn::Error::new_spanned(
            expr,
            "`idempotency` must use IdempotencyMode::Disabled, IdempotencyMode::Check, or IdempotencyMode::CheckAndVerify",
        ));
    };
    if mode_type.ident != "IdempotencyMode" {
        return Err(syn::Error::new_spanned(
            expr,
            "`idempotency` must use IdempotencyMode::Disabled, IdempotencyMode::Check, or IdempotencyMode::CheckAndVerify",
        ));
    }

    match variant.ident.to_string().as_str() {
        "Disabled" => Ok(IdempotencyModeArg::Disabled),
        "Check" => Ok(IdempotencyModeArg::Check),
        "CheckAndVerify" => Ok(IdempotencyModeArg::CheckAndVerify),
        _ => Err(syn::Error::new_spanned(
            expr,
            "`idempotency` must be IdempotencyMode::Disabled, IdempotencyMode::Check, or IdempotencyMode::CheckAndVerify",
        )),
    }
}

impl Parse for RegistrationArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut args = Self::default();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "id" => {
                    if args.id.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate registration key `id`",
                        ));
                    }
                    let id: LitStr = input.parse()?;
                    validate_registration_id_literal(&id)?;
                    args.id = Some(id);
                }
                "version" => {
                    if args.version.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate registration key `version`",
                        ));
                    }
                    let version: LitStr = input.parse()?;
                    validate_registration_version_literal(&version)?;
                    args.version = Some(version);
                }
                "description" => {
                    if args.description.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate registration key `description`",
                        ));
                    }
                    args.description = Some(input.parse()?);
                }
                "factory" => {
                    if args.factory.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate registration key `factory`",
                        ));
                    }
                    args.factory = Some(parse_registration_factory_arg(input)?);
                }
                "schema" => {
                    if args.schema.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate registration key `schema`",
                        ));
                    }
                    args.schema = Some(parse_registration_schema_arg(input)?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unsupported registration key; expected `id`, `version`, `description`, `factory`, or `schema`",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        if args.id.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "`registration(...)` requires `id = \"...\"`",
            ));
        }

        Ok(args)
    }
}

fn parse_registration_schema_arg(input: ParseStream<'_>) -> syn::Result<RegistrationSchemaArg> {
    let schema: LitStr = input.parse()?;
    match schema.value().as_str() {
        "schemars" => Ok(RegistrationSchemaArg::Schemars),
        _ => Err(syn::Error::new_spanned(
            schema,
            "`registration(schema = ...)` supports only `\"schemars\"`",
        )),
    }
}

fn parse_registration_factory_arg(input: ParseStream<'_>) -> syn::Result<RegistrationFactoryArg> {
    if input.peek(LitStr) {
        let factory: LitStr = input.parse()?;
        return match factory.value().as_str() {
            "serde" => Ok(RegistrationFactoryArg::Serde),
            "default" => Ok(RegistrationFactoryArg::Default),
            "custom" => Err(syn::Error::new_spanned(
                factory,
                "use `factory = custom(path)` to configure a custom registration factory",
            )),
            _ => Err(syn::Error::new_spanned(
                factory,
                "`registration(factory = ...)` supports `\"serde\"`, `\"default\"`, or `custom(path)`",
            )),
        };
    }

    let factory: syn::Ident = input.parse()?;
    if factory != "custom" {
        return Err(syn::Error::new_spanned(
            factory,
            "`registration(factory = ...)` supports `\"serde\"`, `\"default\"`, or `custom(path)`",
        ));
    }

    if !input.peek(token::Paren) {
        return Err(syn::Error::new_spanned(
            factory,
            "`factory = custom(...)` requires a factory function path",
        ));
    }

    let content;
    parenthesized!(content in input);
    let factory_path: Path = content.parse()?;
    if !content.is_empty() {
        return Err(syn::Error::new_spanned(
            factory_path,
            "`factory = custom(...)` accepts exactly one factory function path",
        ));
    }
    Ok(RegistrationFactoryArg::Custom(factory_path))
}

fn validate_registration_id_literal(id: &LitStr) -> syn::Result<()> {
    let value = id.value();
    let invalid =
        |reason: &str| syn::Error::new_spanned(id, format!("invalid registration `id`: {reason}"));

    if value.is_empty() {
        return Err(invalid("id must not be empty"));
    }

    if value.trim() != value {
        return Err(invalid("id must not have leading or trailing whitespace"));
    }

    if value.contains('@') {
        return Err(invalid("id must not contain `@`"));
    }

    for segment in value.split('.') {
        if segment.is_empty() {
            return Err(invalid("id segments must not be empty"));
        }

        let first = segment
            .bytes()
            .next()
            .expect("segment is known to be non-empty");
        if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
            return Err(invalid(
                "id segments must start with an ASCII lowercase letter or digit",
            ));
        }

        if !segment.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        }) {
            return Err(invalid(
                "id segments may contain only ASCII lowercase letters, digits, `_`, or `-`",
            ));
        }
    }

    Ok(())
}

fn validate_registration_version_literal(version: &LitStr) -> syn::Result<()> {
    let value = version.value();
    if value.is_empty() {
        return Err(syn::Error::new_spanned(
            version,
            "invalid registration `version`: version must not be empty",
        ));
    }

    semver::Version::parse(&value).map_err(|error| {
        syn::Error::new_spanned(version, format!("invalid registration `version`: {error}"))
    })?;

    Ok(())
}

impl Parse for RetryArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut args = Self::default();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "allow" => {
                    if args.allow.is_some() {
                        return Err(syn::Error::new_spanned(key, "duplicate retry key `allow`"));
                    }
                    args.allow = Some(input.parse()?);
                }
                "max_attempts" => {
                    if args.max_attempts.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate retry key `max_attempts`",
                        ));
                    }
                    let max_attempts: LitInt = input.parse()?;
                    let value = parse_usize_literal(
                        &max_attempts,
                        "`max_attempts` must be a positive integer literal",
                    )?;
                    if value == 0 {
                        return Err(syn::Error::new_spanned(
                            max_attempts,
                            "`max_attempts` must be a positive integer literal",
                        ));
                    }
                    args.max_attempts = Some(max_attempts);
                }
                "delay_ms" => {
                    if args.delay_ms.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate retry key `delay_ms`",
                        ));
                    }
                    let delay_ms: LitInt = input.parse()?;
                    parse_non_negative_u64_literal(
                        &delay_ms,
                        "`delay_ms` must be a non-negative integer literal",
                    )?;
                    args.delay_ms = Some(delay_ms);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unsupported retry key; expected `allow`, `max_attempts`, or `delay_ms`",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        Ok(args)
    }
}

impl Parse for SessionVerificationArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut args = Self::default();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "max_attempts" => {
                    if args.max_attempts.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate session_verification key `max_attempts`",
                        ));
                    }
                    let max_attempts: LitInt = input.parse()?;
                    let value = parse_usize_literal(
                        &max_attempts,
                        "`max_attempts` must be greater than 0",
                    )?;
                    if value == 0 {
                        return Err(syn::Error::new_spanned(
                            max_attempts,
                            "`max_attempts` must be greater than 0",
                        ));
                    }
                    args.max_attempts = Some(max_attempts);
                }
                "delay_ms" => {
                    if args.delay_ms.is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate session_verification key `delay_ms`",
                        ));
                    }
                    let delay_ms: LitInt = input.parse()?;
                    parse_non_negative_u64_literal(
                        &delay_ms,
                        "`delay_ms` must be a non-negative integer literal",
                    )?;
                    args.delay_ms = Some(delay_ms);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unsupported session_verification key; expected `max_attempts` or `delay_ms`",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?;
        }

        Ok(args)
    }
}

fn parse_usize_literal(lit: &LitInt, message: &str) -> syn::Result<usize> {
    if lit.to_string().starts_with('-') {
        return Err(syn::Error::new_spanned(lit, message));
    }
    lit.base10_parse::<usize>()
        .map_err(|_err| syn::Error::new_spanned(lit, message))
}

fn parse_non_negative_u64_literal(lit: &LitInt, message: &str) -> syn::Result<u64> {
    if lit.to_string().starts_with('-') {
        return Err(syn::Error::new_spanned(lit, message));
    }
    lit.base10_parse::<u64>()
        .map_err(|_err| syn::Error::new_spanned(lit, message))
}

fn expand_genja_task(
    args: GenjaTaskArgs,
    item_impl: ItemImpl,
) -> syn::Result<proc_macro2::TokenStream> {
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
    let mut has_dry_run = false;
    let mut has_dry_run_async = false;
    let mut has_check = false;
    let mut has_check_async = false;
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
            "dry_run" => {
                validate_dry_run_method(method, false)?;
                has_dry_run = true;
            }
            "dry_run_async" => {
                validate_dry_run_method(method, true)?;
                has_dry_run_async = true;
            }
            "check" => {
                validate_check_method(method, false)?;
                has_check = true;
            }
            "check_async" => {
                validate_check_method(method, true)?;
                has_check_async = true;
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

    if has_dry_run && has_dry_run_async {
        return Err(syn::Error::new_spanned(
            &item_impl.self_ty,
            "define at most one of `fn dry_run(...)` or `async fn dry_run_async(...)`",
        ));
    }

    if has_check && has_check_async {
        return Err(syn::Error::new_spanned(
            &item_impl.self_ty,
            "define at most one of `fn check(...)` or `async fn check_async(...)`",
        ));
    }

    let supports_dry_run = args
        .supports_dry_run
        .as_ref()
        .map(|value| value.value())
        .unwrap_or(false);
    if supports_dry_run {
        match (has_start, has_start_async, has_dry_run, has_dry_run_async) {
            (true, false, true, false) | (false, true, false, true) => {}
            (true, false, false, false) => {
                return Err(syn::Error::new_spanned(
                    &item_impl.self_ty,
                    "`supports_dry_run = true` requires `fn dry_run(...)` for blocking tasks",
                ));
            }
            (false, true, false, false) => {
                return Err(syn::Error::new_spanned(
                    &item_impl.self_ty,
                    "`supports_dry_run = true` requires `async fn dry_run_async(...)` for async tasks",
                ));
            }
            (true, false, false, true) => {
                return Err(syn::Error::new_spanned(
                    &item_impl.self_ty,
                    "`supports_dry_run = true` requires blocking tasks to define `fn dry_run(...)`, not `dry_run_async(...)`",
                ));
            }
            (false, true, true, false) => {
                return Err(syn::Error::new_spanned(
                    &item_impl.self_ty,
                    "`supports_dry_run = true` requires async tasks to define `async fn dry_run_async(...)`, not `dry_run(...)`",
                ));
            }
            _ => {}
        }
    }

    let idempotency = args.idempotency;
    if idempotency.is_some_and(IdempotencyModeArg::requires_check) {
        match (has_start, has_start_async, has_check, has_check_async) {
            (true, false, true, false) | (false, true, false, true) => {}
            (true, false, false, false) => {
                return Err(syn::Error::new_spanned(
                    &item_impl.self_ty,
                    "`idempotency` requires `fn check(...)` for blocking tasks",
                ));
            }
            (false, true, false, false) => {
                return Err(syn::Error::new_spanned(
                    &item_impl.self_ty,
                    "`idempotency` requires `async fn check_async(...)` for async tasks",
                ));
            }
            (true, false, false, true) => {
                return Err(syn::Error::new_spanned(
                    &item_impl.self_ty,
                    "`idempotency` requires blocking tasks to define `fn check(...)`, not `check_async(...)`",
                ));
            }
            (false, true, true, false) => {
                return Err(syn::Error::new_spanned(
                    &item_impl.self_ty,
                    "`idempotency` requires async tasks to define `async fn check_async(...)`, not `check(...)`",
                ));
            }
            _ => {}
        }
    }

    let name = args.name.expect("validated above");
    let connection_plugin_name = args.connection_plugin_name;
    let processors = args.processors;
    let retry = args.retry;
    let registration = args.registration;
    let session_verification = args.session_verification;

    if session_verification.is_some() && connection_plugin_name.is_none() {
        return Err(syn::Error::new_spanned(
            &item_impl.self_ty,
            "`session_verification(...)` requires `connection_plugin_name = \"...\"`",
        ));
    }

    let connection_impl = match &connection_plugin_name {
        Some(plugin_name) => quote! { Some(#plugin_name) },
        None => quote! { None },
    };

    let options_impl = if has_options {
        quote! {
            fn options(&self) -> Option<&genja_core::__serde_json::Value> {
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

    let retry_config_value = retry.as_ref().map(|retry| {
        let allow = match retry.allow {
            Some(ref allow) => quote! { Some(#allow) },
            None => quote! { None },
        };
        let max_attempts = match retry.max_attempts {
            Some(ref max_attempts) => quote! { Some(#max_attempts) },
            None => quote! { None },
        };
        let delay_ms = match retry.delay_ms {
            Some(ref delay_ms) => quote! { Some(#delay_ms) },
            None => quote! { None },
        };
        quote! { genja_core::task::RetryConfig::new(#allow, #max_attempts, #delay_ms) }
    });

    let retry_config_impl = if let Some(retry_config_value) = &retry_config_value {
        quote! {
            fn retry_config(&self) -> Option<&genja_core::task::RetryConfig> {
                static RETRY_CONFIG: genja_core::task::RetryConfig =
                    #retry_config_value;
                Some(&RETRY_CONFIG)
            }
        }
    } else {
        quote! {}
    };

    let supports_dry_run_impl = if supports_dry_run {
        quote! {
            fn supports_dry_run(&self) -> bool {
                true
            }
        }
    } else {
        quote! {}
    };

    let session_verification_config_impl = if let Some(session_verification) = session_verification
    {
        let max_attempts = match session_verification.max_attempts {
            Some(max_attempts) => quote! { #max_attempts },
            None => quote! { 1 },
        };
        let delay_ms = match session_verification.delay_ms {
            Some(delay_ms) => quote! { #delay_ms },
            None => quote! { 0 },
        };
        quote! {
            fn session_verification_config(&self) -> Option<&genja_core::task::SessionVerificationConfig> {
                static SESSION_VERIFICATION_CONFIG: genja_core::task::SessionVerificationConfig =
                    genja_core::task::SessionVerificationConfig::new(#max_attempts, #delay_ms);
                Some(&SESSION_VERIFICATION_CONFIG)
            }
        }
    } else {
        quote! {}
    };

    let idempotency_mode_impl = if let Some(idempotency) = idempotency {
        let mode = match idempotency {
            IdempotencyModeArg::Disabled => {
                quote! { genja_core::task::IdempotencyMode::Disabled }
            }
            IdempotencyModeArg::Check => {
                quote! { genja_core::task::IdempotencyMode::Check }
            }
            IdempotencyModeArg::CheckAndVerify => {
                quote! { genja_core::task::IdempotencyMode::CheckAndVerify }
            }
        };
        quote! {
            fn idempotency_mode(&self) -> genja_core::task::IdempotencyMode {
                #mode
            }
        }
    } else {
        quote! {}
    };

    let execution_mode = if has_start {
        quote! { genja_core::task::TaskExecutionMode::Blocking }
    } else {
        quote! { genja_core::task::TaskExecutionMode::Async }
    };

    let descriptor_connection_plugin_name = match &connection_plugin_name {
        Some(plugin_name) => quote! { Some(#plugin_name.to_string()) },
        None => quote! { None },
    };
    let descriptor_retry = match &retry_config_value {
        Some(retry_config_value) => quote! { Some(#retry_config_value) },
        None => quote! { None },
    };
    let descriptor_registration = if let Some(registration) = &registration {
        let registration_id = registration.id.as_ref().expect("validated by parser");
        let registration_version = match &registration.version {
            Some(version) => quote! { #version },
            None => quote! { env!("CARGO_PKG_VERSION") },
        };
        let registration_description = match &registration.description {
            Some(description) => quote! { Some(#description.to_string()) },
            None => quote! { None },
        };
        let registration_input_schema = match &registration.schema {
            Some(RegistrationSchemaArg::Schemars) => quote! {
                Some(
                    genja_core::__serde_json::to_value(
                        genja_core::__schemars::schema_for!(#self_ty)
                    )
                    .expect("schemars schema should serialize to JSON")
                )
            },
            None => quote! { None },
        };
        let registration_create = match registration
            .factory
            .as_ref()
            .unwrap_or(&RegistrationFactoryArg::Serde)
        {
            RegistrationFactoryArg::Serde => quote! {
                let task: #self_ty =
                    genja_core::__serde_json::from_value(input).map_err(|error| {
                        genja_core::task::TaskRegistrationError::InvalidInput {
                            id: #registration_id.to_string(),
                            version: #registration_version.to_string(),
                            message: error.to_string(),
                        }
                    })?;
                Ok(genja_core::task::TaskDefinition::new(task))
            },
            RegistrationFactoryArg::Default => quote! {
                if !input.is_null()
                    && !input
                        .as_object()
                        .is_some_and(|object| object.is_empty())
                {
                    return Err(genja_core::task::TaskRegistrationError::InvalidInput {
                        id: #registration_id.to_string(),
                        version: #registration_version.to_string(),
                        message: "default factory expects empty input (`null` or `{}`)".to_string(),
                    });
                }

                let task: #self_ty = <#self_ty as std::default::Default>::default();
                Ok(genja_core::task::TaskDefinition::new(task))
            },
            RegistrationFactoryArg::Custom(factory_path) => quote! {
                let task: #self_ty = #factory_path(input)?;
                Ok(genja_core::task::TaskDefinition::new(task))
            },
        };

        quote! {
            const _: () = {
                fn __genja_task_descriptor() -> genja_core::task::TaskDescriptor {
                    genja_core::task::TaskDescriptor::explicit(
                        #registration_id,
                        #registration_version,
                        genja_core::task::TaskDescriptorMetadata {
                            name: #name.to_string(),
                            description: #registration_description,
                            execution_mode: #execution_mode,
                            connection_plugin_name: #descriptor_connection_plugin_name,
                            processor_names: vec![#(#processors.to_string()),*],
                            retry: #descriptor_retry,
                        },
                        #registration_input_schema,
                        true,
                    )
                }

                fn __genja_task_create(
                    input: genja_core::__serde_json::Value,
                ) -> Result<genja_core::task::TaskDefinition, genja_core::task::TaskRegistrationError> {
                    #registration_create
                }

                genja_core::__inventory::submit! {
                    genja_core::task::CompiledTaskRegistration::constructible(
                        __genja_task_descriptor,
                        __genja_task_create,
                    )
                }
            };
        }
    } else {
        quote! {
            const _: () = {
                fn __genja_task_descriptor() -> genja_core::task::TaskDescriptor {
                    genja_core::task::TaskDescriptor::generated(
                        format!("auto:{}", std::any::type_name::<#self_ty>()),
                        env!("CARGO_PKG_VERSION"),
                        genja_core::task::TaskDescriptorMetadata {
                            name: #name.to_string(),
                            description: None,
                            execution_mode: #execution_mode,
                            connection_plugin_name: #descriptor_connection_plugin_name,
                            processor_names: vec![#(#processors.to_string()),*],
                            retry: #descriptor_retry,
                        },
                    )
                }

                genja_core::__inventory::submit! {
                    genja_core::task::CompiledTaskRegistration::descriptor_only(__genja_task_descriptor)
                }
            };
        }
    };

    let task_impl = if has_start {
        let dry_run_impl = if has_dry_run {
            quote! {
                fn dry_run(
                    &self,
                    host: &genja_core::inventory::Host,
                    context: &genja_core::task::BlockingTaskRuntimeContext,
                ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
                    #self_ty::dry_run(self, host, context)
                }
            }
        } else {
            quote! {}
        };
        let check_impl = if has_check {
            quote! {
                fn check(
                    &self,
                    host: &genja_core::inventory::Host,
                    context: &genja_core::task::BlockingTaskRuntimeContext,
                ) -> Result<genja_core::task::IdempotencyCheck, genja_core::task::TaskError> {
                    #self_ty::check(self, host, context)
                }
            }
        } else {
            quote! {}
        };
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

                #dry_run_impl

                #check_impl

                #sub_tasks_impl

                fn execution_mode(&self) -> genja_core::task::TaskExecutionMode {
                    #execution_mode
                }
            }
        }
    } else {
        let dry_run_impl = if has_dry_run_async {
            quote! {
                async fn dry_run_async(
                    &self,
                    host: &genja_core::inventory::Host,
                    context: &genja_core::task::TaskRuntimeContext,
                ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
                    #self_ty::dry_run_async(self, host, context).await
                }
            }
        } else {
            quote! {}
        };
        let check_impl = if has_check_async {
            quote! {
                async fn check_async(
                    &self,
                    host: &genja_core::inventory::Host,
                    context: &genja_core::task::TaskRuntimeContext,
                ) -> Result<genja_core::task::IdempotencyCheck, genja_core::task::TaskError> {
                    #self_ty::check_async(self, host, context).await
                }
            }
        } else {
            quote! {}
        };
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

                #dry_run_impl

                #check_impl

                #sub_tasks_impl

                fn execution_mode(&self) -> genja_core::task::TaskExecutionMode {
                    #execution_mode
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

            #retry_config_impl

            #supports_dry_run_impl

            #session_verification_config_impl

            #idempotency_mode_impl
        }

        #task_impl

        #descriptor_registration
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
    validate_typed_arg(
        inputs.next().unwrap(),
        is_host_ref,
        "`host` must be `&Host`",
    )?;
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

    validate_return_type(
        &method.sig.output,
        is_result_host_task_error,
        if is_async {
            "`start_async` must return `Result<HostTaskResult, TaskError>`"
        } else {
            "`start` must return `Result<HostTaskResult, TaskError>`"
        },
    )
}

fn validate_dry_run_method(method: &syn::ImplItemFn, is_async: bool) -> syn::Result<()> {
    if method.sig.asyncness.is_some() != is_async {
        let expected = if is_async {
            "`dry_run_async` must be declared as `async fn`"
        } else {
            "`dry_run` must be declared as `fn`, not `async fn`"
        };
        return Err(syn::Error::new_spanned(&method.sig.ident, expected));
    }

    validate_shared_method_shape(method)?;

    if method.sig.inputs.len() != 3 {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "task dry-run methods must take `&self`, `host`, and `context`",
        ));
    }

    let mut inputs = method.sig.inputs.iter();
    validate_receiver(inputs.next().unwrap())?;
    validate_typed_arg(
        inputs.next().unwrap(),
        is_host_ref,
        "`host` must be `&Host`",
    )?;
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

    validate_return_type(
        &method.sig.output,
        is_result_host_task_error,
        if is_async {
            "`dry_run_async` must return `Result<HostTaskResult, TaskError>`"
        } else {
            "`dry_run` must return `Result<HostTaskResult, TaskError>`"
        },
    )
}

fn validate_check_method(method: &syn::ImplItemFn, is_async: bool) -> syn::Result<()> {
    if method.sig.asyncness.is_some() != is_async {
        let expected = if is_async {
            "`check_async` must be declared as `async fn`"
        } else {
            "`check` must be declared as `fn`, not `async fn`"
        };
        return Err(syn::Error::new_spanned(&method.sig.ident, expected));
    }

    validate_shared_method_shape(method)?;

    if method.sig.inputs.len() != 3 {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "task idempotency check methods must take `&self`, `host`, and `context`",
        ));
    }

    let mut inputs = method.sig.inputs.iter();
    validate_receiver(inputs.next().unwrap())?;
    validate_typed_arg(
        inputs.next().unwrap(),
        is_host_ref,
        "`host` must be `&Host`",
    )?;
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

    validate_return_type(
        &method.sig.output,
        is_result_idempotency_check_task_error,
        if is_async {
            "`check_async` must return `Result<IdempotencyCheck, TaskError>`"
        } else {
            "`check` must return `Result<IdempotencyCheck, TaskError>`"
        },
    )
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
        _ => Err(syn::Error::new_spanned(
            arg,
            "first argument must be `&self`",
        )),
    }
}

fn validate_typed_arg(arg: &FnArg, predicate: fn(&Type) -> bool, message: &str) -> syn::Result<()> {
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
    is_result_with_ok_type(ty, "HostTaskResult")
}

fn is_result_idempotency_check_task_error(ty: &Type) -> bool {
    is_result_with_ok_type(ty, "IdempotencyCheck")
}

fn is_result_with_ok_type(ty: &Type, ok_type_name: &str) -> bool {
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
        Some(GenericArgument::Type(ty)) => type_ends_with(ty, ok_type_name),
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
        Some(GenericArgument::Type(inner)) => is_arc_task(inner),
        _ => false,
    }
}

fn is_arc_task(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let Some(seg) = path.segments.last() else {
                return false;
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
