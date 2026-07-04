//! Procedural macros used by `genja-core`.
//!
//! `DerefMacro` and `DerefMutMacro` generate `Deref` and `DerefMut`
//! implementations for tuple-wrapper types.
//!
//! `genja_task` is the public task-authoring macro. It generates both
//! `TaskInfo` and `Task` implementations from an inherent `impl` block and
//! infers execution mode from `fn start(...)` versus `async fn start_async(...)`.
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
    LitBool, LitInt, LitStr, PathArguments, ReturnType, Token, Type, TypePath, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
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
    retry: Option<RetryArgs>,
}

#[derive(Default)]
struct RetryArgs {
    allow: Option<LitBool>,
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
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unsupported key; expected `name`, `connection_plugin_name`, `processors`, or `retry(...)`",
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
    let retry = args.retry;

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

    let retry_config_impl = if let Some(retry) = retry {
        let allow = match retry.allow {
            Some(allow) => quote! { Some(#allow) },
            None => quote! { None },
        };
        let max_attempts = match retry.max_attempts {
            Some(max_attempts) => quote! { Some(#max_attempts) },
            None => quote! { None },
        };
        let delay_ms = match retry.delay_ms {
            Some(delay_ms) => quote! { Some(#delay_ms) },
            None => quote! { None },
        };
        quote! {
            fn retry_config(&self) -> Option<&genja_core::task::RetryConfig> {
                static RETRY_CONFIG: genja_core::task::RetryConfig =
                    genja_core::task::RetryConfig::new(#allow, #max_attempts, #delay_ms);
                Some(&RETRY_CONFIG)
            }
        }
    } else {
        quote! {}
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

            #retry_config_impl
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
