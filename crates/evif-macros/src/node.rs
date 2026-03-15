// node 宏 - 为结构体生成节点相关代码

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput};

/// #[node] 宏的参数
#[derive(Debug, Default, FromDeriveInput)]
#[darling(attributes(node), default)]
pub struct NodeArgs {
    pub builder: Option<bool>,
    pub clone: Option<bool>,
    pub debug: Option<bool>,
}

/// #[node] 过程宏入口
pub fn node_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let args = NodeArgs::from_derive_input(&input).expect("Wrong arguments");

    let expanded = impl_node_macro(&input, &args);
    TokenStream::from(expanded)
}

/// 实现 node 宏逻辑
fn impl_node_macro(input: &DeriveInput, args: &NodeArgs) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = extract_fields(input);

    let builder_impl = if args.builder.unwrap_or(true) {
        generate_builder(name, &impl_generics, &ty_generics, where_clause, &fields)
    } else {
        proc_macro2::TokenStream::new()
    };

    let clone_impl = if args.clone.unwrap_or(true) {
        generate_clone(name, &impl_generics, &ty_generics, where_clause, &fields)
    } else {
        proc_macro2::TokenStream::new()
    };

    let debug_impl = if args.debug.unwrap_or(true) {
        generate_debug(name, &impl_generics, &ty_generics, where_clause, &fields)
    } else {
        proc_macro2::TokenStream::new()
    };

    quote! {
        #builder_impl
        #clone_impl
        #debug_impl
    }
}

/// 提取结构体字段信息
fn extract_fields(input: &DeriveInput) -> Vec<(Ident, syn::Type)> {
    let mut fields = Vec::new();

    if let Data::Struct(DataStruct {
        fields: struct_fields,
        ..
    }) = &input.data
    {
        for field in struct_fields {
            if let Some(ident) = &field.ident {
                fields.push((ident.clone(), field.ty.clone()));
            }
        }
    }

    fields
}

/// 生成 Builder 模式
fn generate_builder(
    name: &Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    fields: &[(Ident, syn::Type)],
) -> proc_macro2::TokenStream {
    let builder_name = Ident::new(&format!("{}Builder", name), name.span());

    let field_names: Vec<&Ident> = fields.iter().map(|(name, _)| name).collect();
    let field_types: Vec<&syn::Type> = fields.iter().map(|(_, ty)| ty).collect();

    let builder_methods = fields.iter().map(|(name, ty)| {
        quote! {
            pub fn #name(mut self, value: #ty) -> Self {
                self.#name = Some(value);
                self
            }
        }
    });

    let field_assignments = fields.iter().map(|(name, _)| {
        quote! {
            #name: self.#name.ok_or_else(|| {
                format!("Field '{}' is required", stringify!(#name))
            })?
        }
    });

    quote! {
        #[derive(Debug, Clone)]
        pub struct #builder_name #impl_generics {
            #(pub #field_names: Option<#field_types>,)*
        }

        impl #impl_generics #builder_name #ty_generics #where_clause {
            pub fn new() -> Self {
                Self {
                    #(#field_names: None,)*
                }
            }

            #(#builder_methods)*

            pub fn build(self) -> Result<#name #ty_generics, String> {
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

/// 生成 Clone 实现
fn generate_clone(
    name: &Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    fields: &[(Ident, syn::Type)],
) -> proc_macro2::TokenStream {
    let field_names: Vec<&Ident> = fields.iter().map(|(name, _)| name).collect();

    quote! {
        impl #impl_generics Clone for #name #ty_generics #where_clause {
            fn clone(&self) -> Self {
                Self {
                    #(self.#field_names.clone(),)*
                }
            }
        }
    }
}

/// 生成 Debug 实现
fn generate_debug(
    name: &Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
    fields: &[(Ident, syn::Type)],
) -> proc_macro2::TokenStream {
    let field_names: Vec<&Ident> = fields.iter().map(|(name, _)| name).collect();

    quote! {
        impl #impl_generics std::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!(#name))
                    #(.field(stringify!(#field_names), &self.#field_names))*
                    .finish()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_macro_expansion() {
        let input = quote::quote! {
            pub struct TestNode {
                id: u64,
                name: String,
                value: i32,
            }
        };

        let parsed = syn::parse2::<DeriveInput>(input).unwrap();
        let args = NodeArgs::default();

        let result = impl_node_macro(&parsed, &args);
        assert!(!result.is_empty());
    }
}
