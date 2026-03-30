//! This crate provides two procedural macros: `DerefMacro` and `DerefMutMacro`.
//! These macros allow you to implement the `Deref` and `DerefMut` traits
//! for your custom types.
//!
//! # Example
//! ```
//! use genja_core_derive::{DerefMacro, DerefMutMacro};
//!
//! pub trait DerefTarget {
//!     type Target;
//! }
//!
//! pub type DefaultListTarget = Vec<String>;;
//!
//! impl DerefTarget for DefaultsList {
//!     type Target = DefaultListTarget;
//! }
//!
//! #[derive(DerefMacro, DerefMutMacro, PartialEq)]
//! // #[serde(deny_unknown_fields)]
//! pub struct DefaultsList(DefaultListTarget);
//!
//! let mut defaults_list = DefaultsList(DefaultListTarget::new());
//!
//! defaults_list.push("default1".to_string());
//!
//! assert_eq!(defaults_list.as_ref(), vec!["default1".to_string()]);
//!```

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, GenericArgument, Meta, PathArguments, Type, TypePath, parse_macro_input,
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

    let expanded = quote! {
        impl std::ops::DerefMut for #name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derives the `Task` trait implementation for a struct.
///
/// This procedural macro generates implementations of `TaskInfo` and `SubTasks` traits
/// for structs that represent tasks in the task execution system. It validates the struct's
/// fields and generates appropriate getter methods and subtask collection logic.
///
/// The macro expects the struct to have:
/// - A `name` field of type `String` or `&'static str` (required)
/// - An optional `plugin_name` field of type `String`, `&'static str`, `Option<String>`, or `Option<&'static str>`
/// - An optional `options` field of type `Option<serde_json::Value>`
/// - Zero or more fields marked with `#[task(subtask)]` attribute of type `Arc<dyn Task>`
///
/// # Parameters
///
/// * `input` - A `TokenStream` representing the input tokens of the derive macro, containing
///             the struct definition to which the `Task` trait should be applied.
///
/// # Returns
///
/// A `TokenStream` containing the generated implementations of `TaskInfo` and `SubTasks` traits.
/// Returns a compile error if:
/// - The macro is applied to a non-struct type
/// - The struct doesn't have named fields
/// - Required fields are missing or have incorrect types
/// - Subtask fields are not of type `Arc<dyn Task>`
///
/// # Examples
///
/// ```ignore
/// #[derive(Task)]
/// struct MyTask {
///     name: String,
///     plugin_name: Option<String>,
///     options: Option<serde_json::Value>,
///     #[task(subtask)]
///     child_task: Arc<dyn Task>,
/// }
/// ```
#[proc_macro_derive(Task, attributes(task))]
pub fn derive_task(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let data = match input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return syn::Error::new_spanned(
                name,
                "Task can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    let fields = match data.fields {
        syn::Fields::Named(fields) => fields.named,
        _ => {
            return syn::Error::new_spanned(
                name,
                "Task requires named fields",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut name_field = None;
    let mut plugin_name_field = None;
    let mut options_field = None;
    let mut subtask_fields: Vec<(syn::Ident, SubtaskKind)> = Vec::new();

    for field in fields.iter() {
        let ident = match &field.ident {
            Some(ident) => ident,
            None => continue,
        };

        if has_subtask_attr(&field.attrs) {
            match subtask_kind(&field.ty) {
                Some(kind) => subtask_fields.push((ident.clone(), kind)),
                None => {
                    return syn::Error::new_spanned(
                        &field.ty,
                        "subtask fields must be `Arc<dyn Task>`",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }

        match ident.to_string().as_str() {
            "name" => name_field = Some(field.ty.clone()),
            "plugin_name" => plugin_name_field = Some(field.ty.clone()),
            "options" => options_field = Some(field.ty.clone()),
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
        return syn::Error::new_spanned(
            name_ty,
            "`name` must be `String` or `&'static str`",
        )
        .to_compile_error()
        .into();
    }

    let plugin_name_ty = plugin_name_field.clone();
    if let Some(ty) = &plugin_name_ty {
        if !is_string_or_static_str(ty) && !is_option_of(ty, is_string_or_static_str) {
            return syn::Error::new_spanned(
                ty,
                "`plugin_name` must be `String`, `&'static str`, `Option<String>`, or `Option<&'static str>`",
            )
            .to_compile_error()
            .into();
        }
    }


    if let Some(options_ty) = &options_field {
        if !is_option_of(options_ty, is_value_type) {
            return syn::Error::new_spanned(
                options_ty,
                "`options` must be `Option<serde_json::Value>`",
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

    let plugin_name_getter = match plugin_name_ty {
        Some(ty) if is_string_type(&ty) => quote! { self.plugin_name.as_str() },
        Some(ty) if is_static_str_type(&ty) => quote! { self.plugin_name },
        Some(_) => quote! { self.plugin_name.as_deref().unwrap_or("") },
        None => quote! { "" },
    };

    let options_getter = if options_field.is_some() {
        quote! { self.options.as_ref() }
    } else {
        quote! { None }
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

            fn plugin_name(&self) -> &str {
                #plugin_name_getter
            }

            fn get_connection_key(
                &self,
                hostname: &str,
            ) -> genja_core::inventory::ConnectionKey {
                genja_core::inventory::ConnectionKey::new(hostname, #plugin_name_getter)
            }

            fn options(&self) -> Option<&serde_json::Value> {
                #options_getter
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
    };

    TokenStream::from(expanded)
}

fn is_string_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            path.segments.last().map(|seg| seg.ident == "String").unwrap_or(false)
        }
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

fn is_value_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let mut segments = path.segments.iter();
            let last = segments.next_back().map(|seg| seg.ident.to_string());
            let second_last = segments.next_back().map(|seg| seg.ident.to_string());

            match (second_last.as_deref(), last.as_deref()) {
                (Some("serde_json"), Some("Value")) => true,
                (None, Some("Value")) => true,
                _ => false,
            }
        }
        _ => false,
    }
}

fn is_string_or_static_str(ty: &Type) -> bool {
    is_string_type(ty) || is_static_str_type(ty)
}

#[derive(Copy, Clone)]
enum SubtaskKind {
    SingleArc,
}

fn has_subtask_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("task") {
            return false;
        }
        match attr.meta {
            Meta::List(ref list) => list.tokens.to_string().contains("subtask"),
            _ => false,
        }
    })
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
