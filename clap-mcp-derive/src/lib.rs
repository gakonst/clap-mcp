use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};

#[proc_macro_derive(McpMode, attributes(mcp))]
pub fn derive_mcp_mode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Find the field marked with #[mcp(mode_flag)]
    let mode_flag_field = find_mode_flag_field(&input.data);

    // Find the subcommand field
    let subcommand_field = find_subcommand_field(&input.data);

    let expanded = match (mode_flag_field, subcommand_field) {
        (Some(flag_field), Some((cmd_field, cmd_type))) => generate_mcp_impl(
            name,
            &impl_generics,
            &ty_generics,
            &where_clause,
            flag_field,
            cmd_field,
            cmd_type,
        ),
        _ => {
            return syn::Error::new_spanned(
                name,
                "McpMode requires a field marked with #[mcp(mode_flag)] and a subcommand field",
            )
            .to_compile_error()
            .into();
        }
    };

    TokenStream::from(expanded)
}

fn find_mode_flag_field(data: &Data) -> Option<Ident> {
    match data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields) => {
                    for field in &fields.named {
                        for attr in &field.attrs {
                            if attr.path().is_ident("mcp") {
                                // For syn 2.0, we need to parse the attribute differently
                                let attr_str = quote!(#attr).to_string();
                                if attr_str.contains("mode_flag") {
                                    return field.ident.clone();
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

fn find_subcommand_field(data: &Data) -> Option<(Ident, Type)> {
    match data {
        Data::Struct(data_struct) => {
            match &data_struct.fields {
                Fields::Named(fields) => {
                    for field in &fields.named {
                        for attr in &field.attrs {
                            if attr.path().is_ident("command") {
                                // For syn 2.0, we need to parse the attribute differently
                                let attr_str = quote!(#attr).to_string();
                                if attr_str.contains("subcommand") {
                                    let ty = &field.ty;
                                    // Extract the inner type if it's Option<T>
                                    let inner_type = if let Type::Path(type_path) = ty {
                                        if let Some(segment) = type_path.path.segments.last() {
                                            if segment.ident == "Option" {
                                                if let syn::PathArguments::AngleBracketed(args) =
                                                    &segment.arguments
                                                {
                                                    if let Some(syn::GenericArgument::Type(inner)) =
                                                        args.args.first()
                                                    {
                                                        inner.clone()
                                                    } else {
                                                        ty.clone()
                                                    }
                                                } else {
                                                    ty.clone()
                                                }
                                            } else {
                                                ty.clone()
                                            }
                                        } else {
                                            ty.clone()
                                        }
                                    } else {
                                        ty.clone()
                                    };
                                    return Some((field.ident.clone()?, inner_type));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

fn generate_mcp_impl(
    name: &Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: &Option<&syn::WhereClause>,
    mode_flag: Ident,
    _subcommand_field: Ident,
    subcommand_type: Type,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn run_mcp_server(&self) -> Result<(), Box<dyn std::error::Error>> {
                use clap_mcp::{McpServer, McpTransport};

                if !self.#mode_flag {
                    return Err("MCP mode not enabled".into());
                }

                let server = McpServer::<#subcommand_type>::new();
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(server.serve_stdio())?;

                Ok(())
            }

            pub fn run_mcp_server_with_handler(
                &self,
                handler: impl Fn(#subcommand_type) -> Result<String, String> + Send + Sync + 'static
            ) -> Result<(), Box<dyn std::error::Error>> {
                use clap_mcp::{McpServer, McpTransport};

                if !self.#mode_flag {
                    return Err("MCP mode not enabled".into());
                }

                let server = McpServer::<#subcommand_type>::new()
                    .with_handler(Box::new(handler));
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(server.serve_stdio())?;

                Ok(())
            }

            pub fn run_mcp_server_http(&self, addr: std::net::SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
                use clap_mcp::{McpServer, McpTransport};

                if !self.#mode_flag {
                    return Err("MCP mode not enabled".into());
                }

                let server = McpServer::<#subcommand_type>::new();
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(server.serve_http(addr))?;

                Ok(())
            }

            pub fn run_mcp_server_http_with_handler(
                &self,
                addr: std::net::SocketAddr,
                handler: impl Fn(#subcommand_type) -> Result<String, String> + Send + Sync + 'static
            ) -> Result<(), Box<dyn std::error::Error>> {
                use clap_mcp::{McpServer, McpTransport};

                if !self.#mode_flag {
                    return Err("MCP mode not enabled".into());
                }

                let server = McpServer::<#subcommand_type>::new()
                    .with_handler(Box::new(handler));
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(server.serve_http(addr))?;

                Ok(())
            }
        }
    }
}
