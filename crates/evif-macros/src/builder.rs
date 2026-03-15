// builder 宏 - 为结构体生成 Builder 模式

use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput};

/// #[builder] 宏的参数
#[derive(Debug, Default, FromDeriveInput)]
#[darling(attributes(builder), default)]
pub struct BuilderArgs {
    pub build_fn: Option<Ident>,
    pub validate: Option<bool>,
}

/// 字段级属性
#[derive(Debug, Default, FromField)]
#[darling(attributes(builder), default)]
struct BuilderFieldAttrs {
    skip: bool,
    has_default: bool,
}

/// #[builder] 过程宏入口
pub fn builder_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = BuilderArgs::from_derive_input(&input).expect("Wrong arguments");

    let expanded = impl_builder_macro(&input, &args);
    TokenStream::from(expanded)
}

/// 实现 builder 宏逻辑
fn impl_builder_macro(input: &DeriveInput, args: &BuilderArgs) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let builder_name = Ident::new(&format!("{}Builder", name), name.span());
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = extract_fields(input);

    let build_fn_name = args
        .build_fn
        .as_ref()
        .map(|i| quote::format_ident!("{}", i))
        .unwrap_or_else(|| quote::format_ident!("build"));

    let field_names: Vec<&Ident> = fields.iter().map(|(name, _, _)| name).collect();
    let _field_types: Vec<&syn::Type> = fields.iter().map(|(_, ty, _)| ty).collect();

    let builder_fields = fields.iter().map(|(name, ty, default)| {
        if *default {
            quote! {
                pub #name: Option<#ty>,
            }
        } else {
            quote! {
                pub #name: Option<#ty>,
            }
        }
    });

    let builder_methods = fields.iter().map(|(name, ty, _)| {
        quote! {
            pub fn #name(mut self, value: #ty) -> Self {
                self.#name = Some(value);
                self
            }
        }
    });

    let field_assignments = fields.iter().map(|(name, _, _)| {
        quote! {
            #name: self.#name.ok_or_else(|| {
                format!("Field '{}' is required", stringify!(#name))
            })?
        }
    });

    quote! {
        #[derive(Debug, Clone)]
        pub struct #builder_name #impl_generics {
            #(#builder_fields)*
        }

        impl #impl_generics #builder_name #ty_generics #where_clause {
            pub fn new() -> Self {
                Self {
                    #(#field_names: None,)*
                }
            }

            #(#builder_methods)*

            pub fn #build_fn_name(self) -> Result<#name #ty_generics, String> {
                Ok(#name {
                    #(#field_assignments,)*
                })
            }
        }

        impl #impl_generics Default for #builder_name #ty_generics #where_clause {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

/// 提取字段信息
fn extract_fields(input: &DeriveInput) -> Vec<(Ident, syn::Type, bool)> {
    let mut fields = Vec::new();

    if let Data::Struct(DataStruct {
        fields: struct_fields,
        ..
    }) = &input.data
    {
        for field in struct_fields {
            if let Some(ident) = &field.ident {
                let field_attrs = BuilderFieldAttrs::from_field(field).unwrap_or_default();

                if field_attrs.skip {
                    continue;
                }

                fields.push((ident.clone(), field.ty.clone(), field_attrs.has_default));
            }
        }
    }

    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_macro_expansion() {
        let input = quote::quote! {
            pub struct TestStruct {
                name: String,
                value: i32,
            }
        };

        let parsed = syn::parse2::<DeriveInput>(input).unwrap();
        let args = BuilderArgs::default();

        let result = impl_builder_macro(&parsed, &args);
        assert!(!result.is_empty());
    }
}
