// EVIF 过程宏 - 代码生成工具

mod builder;
mod error_module;
mod node;

/// #[node] 宏 - 为结构体生成节点相关代码
#[proc_macro]
#[proc_macro_error::proc_macro_error]
pub fn node(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    node::node_impl(input)
}

/// #[builder] 宏 - 为结构体生成 Builder 模式
#[proc_macro]
#[proc_macro_error::proc_macro_error]
pub fn builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    builder::builder_impl(input)
}

/// #[error_macro] 宏 - 为错误类型生成代码
#[proc_macro]
#[proc_macro_error::proc_macro_error]
pub fn error_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    error_module::error_impl(input)
}
