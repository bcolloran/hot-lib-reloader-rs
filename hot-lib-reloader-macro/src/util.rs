use proc_macro2::Span;
use std::path::PathBuf;
use syn::{spanned::Spanned, Error, ForeignItemFn, LitStr, Result};

pub fn ident_from_pat(
    pat: &syn::Pat,
    func_name: &proc_macro2::Ident,
    span: proc_macro2::Span,
) -> syn::Result<syn::Ident> {
    match pat {
        syn::Pat::Ident(pat) => Ok(pat.ident.clone()),
        _ => Err(syn::Error::new(
            span,
            format!("generating call for library function: signature of function {func_name} cannot be converted"),
        )),
    }
}

/// Reads the contents of a Rust source file and finds the top-level functions that have
/// - visibility public
/// - #[no_mangle] attribute
/// It converts these functions into a [syn::ForeignItemFn] so that those can
/// serve as lib function declarations of the lib reloader.
pub fn read_unmangled_functions_from_file(file_name: LitStr) -> Result<Vec<(ForeignItemFn, Span)>> {
    let span = file_name.span();
    let path: PathBuf = file_name.value().into();
    let path = if path.is_relative() {
        let file_with_macro = proc_macro::Span::call_site().source_file();
        file_with_macro
            .path()
            .parent()
            .map(|dir| dir.join(&path))
            .unwrap_or(path)
    } else {
        path
    };

    if !path.exists() {
        return Err(Error::new(span, format!("file does not exist: {path:?}")));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|err| Error::new(span, format!("Error reading file {path:?}: {err}")))?;

    let ast = syn::parse_file(&content)?;

    let mut functions = Vec::new();

    for item in ast.items {
        match item {
            syn::Item::Fn(fun) => {
                match fun.vis {
                    syn::Visibility::Public(_) => {}
                    _ => continue,
                };

                let no_mangle = fun
                    .attrs
                    .iter()
                    .filter_map(|attr| attr.path.get_ident())
                    .any(|ident| *ident == "no_mangle");

                if !no_mangle {
                    continue;
                };

                let fun = ForeignItemFn {
                    attrs: Vec::new(),
                    vis: fun.vis,
                    sig: fun.sig,
                    semi_token: syn::token::Semi(span),
                };

                functions.push((fun, file_name.span()));
            }
            _ => continue,
        }
    }

    Ok(functions)
}
