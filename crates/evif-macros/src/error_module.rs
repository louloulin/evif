// error 宏 - 为错误类型生成代码

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse_macro_input, Data, DataEnum, DeriveInput};

/// #[error] 宏的参数
#[derive(Debug, Default, FromDeriveInput)]
#[darling(attributes(error), default)]
pub struct ErrorArgs {
    pub display_fn: Option<Ident>,
}

/// #[error] 过程宏入口
pub fn error_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = ErrorArgs::from_derive_input(&input).expect("Wrong arguments");

    let expanded = impl_error_macro(&input, &args);
    TokenStream::from(expanded)
}

/// 实现 error 宏逻辑
fn impl_error_macro(input: &DeriveInput, _args: &ErrorArgs) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // 获取错误类型信息
    let variants = extract_variants(input);

    // 生成 Display 实现
    let display_impl =
        generate_display(name, &impl_generics, &ty_generics, where_clause, &variants);

    // 生成 std::error::Error 实现
    let error_impl = generate_std_error(name, &impl_generics, &ty_generics, where_clause);

    quote! {
        #display_impl
        #error_impl
    }
}

/// 提取枚举变体信息
struct VariantInfo {
    ident: Ident,
    fields: Vec<Ident>,
}

fn extract_variants(input: &DeriveInput) -> Vec<VariantInfo> {
    let mut result = Vec::new();

    if let Data::Enum(DataEnum { variants, .. }) = &input.data {
        for variant in variants {
            let ident = variant.ident.clone();

            // 提取字段名
            let fields = variant
                .fields
                .iter()
                .enumerate()
                .filter_map(|(i, f)| {
                    f.ident
                        .clone()
                        .or_else(|| Some(Ident::new(&format!("field{}", i), ident.span())))
                })
                .collect();

            result.push(VariantInfo { ident, fields });
        }
    }

    result
}

/// 生成 Display 实现
fn generate_display(
    name: &Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    variants: &[VariantInfo],
) -> proc_macro2::TokenStream {
    let match_arms = variants.iter().map(|v| {
        let variant_ident = &v.ident;
        let fields = &v.fields;

        quote! {
            #name::#variant_ident { #(#fields),* } => {
                write!(f, "{}", stringify!(#variant_ident))
            }
        }
    });

    quote! {
        impl #impl_generics std::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#match_arms)*
                }
            }
        }
    }
}

/// 生成 std::error::Error 实现
fn generate_std_error(
    name: &Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics std::error::Error for #name #ty_generics #where_clause {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_macro_expansion() {
        let input = quote::quote! {
            pub enum TestError {
                IoError,
                NotFound,
                Custom(String),
            }
        };

        let parsed = syn::parse2::<DeriveInput>(input).unwrap();
        let args = ErrorArgs::default();

        let result = impl_error_macro(&parsed, &args);
        assert!(!result.is_empty());
    }
}
