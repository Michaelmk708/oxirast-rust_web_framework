use proc_macro::TokenStream;
use quote::quote;
use syn::{
    ext::IdentExt, 
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, Ident, LitStr, Result, Token,
};

enum AttrValue {
    Literal(LitStr),
    Expression(Expr),
}

struct HtmlAttribute {
    original_ident: Ident, 
    full_key: String,      
    value: AttrValue,
}

impl Parse for HtmlAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let original_ident = Ident::parse_any(input)?;
        let mut full_key = original_ident.to_string();

        while input.peek(Token![-]) || input.peek(Token![:]) {
            if input.peek(Token![-]) {
                input.parse::<Token![-]>()?;
                full_key.push('-');
            } else if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                full_key.push(':');
            }
            let next_ident = Ident::parse_any(input)?;
            full_key.push_str(&next_ident.to_string());
        }

        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            if input.peek(syn::token::Brace) {
                let content;
                syn::braced!(content in input);
                AttrValue::Expression(content.parse()?)
            } else {
                AttrValue::Literal(input.parse::<LitStr>()?)
            }
        } else {
            AttrValue::Literal(syn::LitStr::new("true", original_ident.span()))
        };

        Ok(Self { original_ident, full_key, value })
    }
}

struct HtmlElement {
    tag: syn::Path, 
    attributes: Vec<HtmlAttribute>,
    children: Vec<HtmlNode>,
}

enum HtmlNode {
    Element(HtmlElement),
    Text(LitStr),
    Expression(Expr), 
}

impl Parse for HtmlNode {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![<]) {
            input.parse::<Token![<]>()?;
            let tag = input.parse::<syn::Path>()?; 

            let mut attributes = Vec::new();
            while !input.peek(Token![>]) && !input.peek(Token![/]) {
                attributes.push(input.parse()?);
            }

            if input.peek(Token![/]) {
                input.parse::<Token![/]>()?;
                input.parse::<Token![>]>()?;
                return Ok(HtmlNode::Element(HtmlElement { tag, attributes, children: Vec::new() }));
            }

            input.parse::<Token![>]>()?;

            let mut children = Vec::new();
            while !(input.peek(Token![<]) && input.peek2(Token![/])) {
                children.push(input.parse()?);
            }

            input.parse::<Token![<]>()?;
            input.parse::<Token![/]>()?;
            let close_tag = input.parse::<syn::Path>()?;
            input.parse::<Token![>]>()?;

            let tag_str = quote!(#tag).to_string();
            let close_tag_str = quote!(#close_tag).to_string();

            if tag_str != close_tag_str {
                return Err(syn::Error::new_spanned(close_tag, format!("Mismatched tag. Expected `{}`, found `{}`", tag_str, close_tag_str)));
            }

            Ok(HtmlNode::Element(HtmlElement { tag, attributes, children }))
            
        } else if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            Ok(HtmlNode::Expression(content.parse()?))
        } else {
            let text: LitStr = input.parse()?;
            Ok(HtmlNode::Text(text))
        }
    }
}

fn generate_node(node: &HtmlNode) -> proc_macro2::TokenStream {
    match node {
        HtmlNode::Text(text) => quote! { oxirast_core::VNode::text(#text) },
        HtmlNode::Expression(expr) => quote! { oxirast_core::VNode::text(&(#expr).to_string()) },
        HtmlNode::Element(el) => {
            let tag_path = &el.tag;
            let last_segment = tag_path.segments.last().unwrap().ident.to_string();
            let is_custom_component = last_segment.chars().next().unwrap().is_ascii_uppercase();

            if is_custom_component {
                let mut props_path = tag_path.clone();
                let last = props_path.segments.last_mut().unwrap();
                last.ident = syn::Ident::new(&format!("{}Props", last.ident), last.ident.span());

                if el.attributes.is_empty() { return quote! { #tag_path() }; }

                let props_fields: Vec<_> = el.attributes.iter().map(|attr| {
                    let key = &attr.original_ident; 
                    match &attr.value {
                        AttrValue::Literal(lit) => quote! { #key: String::from(#lit) },
                        AttrValue::Expression(expr) => quote! { #key: #expr },
                    }
                }).collect();

                return quote! { #tag_path(#props_path { #(#props_fields),* }) };
            }

            let tag_str = last_segment;
            let mut attr_calls = Vec::new();

            for attr in &el.attributes {
                let key = &attr.full_key; 
                
                match &attr.value {
                    AttrValue::Literal(lit) => attr_calls.push(quote! { .attr(#key, #lit) }),
                    AttrValue::Expression(expr) => {
                        if key == "bind_text" {
                            attr_calls.push(quote! { .bind_text(#expr) });
                        } else if key.starts_with("bind_attr:") {
                            let attr_name = key.replace("bind_attr:", "");
                            attr_calls.push(quote! { .bind_attr(#attr_name, #expr) });
                            
                        // --- NEW: LIFECYCLE HOOKS ---
                        } else if key == "on_mount" {
                            attr_calls.push(quote! { 
                                .on_mount(std::rc::Rc::new(std::cell::RefCell::new(Box::new(#expr)))) 
                            });
                        } else if key == "on_cleanup" {
                            attr_calls.push(quote! { 
                                .on_cleanup(std::rc::Rc::new(std::cell::RefCell::new(Box::new(#expr)))) 
                            });
                            
                        } else if key.starts_with("on_") {
                            let event_name = key.replace("on_", "");
                            attr_calls.push(quote! { 
                                .on(#event_name, std::rc::Rc::new(std::cell::RefCell::new(Box::new(#expr)))) 
                            });
                        } else {
                            attr_calls.push(quote! { .attr(#key, &(#expr).to_string()) });
                        }
                    }, 
                }
            }

            let children: Vec<_> = el.children.iter().map(|child| {
                let child_code = generate_node(child);
                quote! { .child(#child_code) }
            }).collect();

            quote! {
                oxirast_core::VNode::element(#tag_str)
                #(#attr_calls)*
                #(#children)*
                .build()
            }
        }
    }
}

#[proc_macro]
pub fn rsx(input: TokenStream) -> TokenStream {
    let root_node = parse_macro_input!(input as HtmlNode);
    let expanded = generate_node(&root_node);
    TokenStream::from(expanded)
}