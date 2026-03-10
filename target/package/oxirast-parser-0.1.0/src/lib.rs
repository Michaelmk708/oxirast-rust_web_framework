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
    key: Ident,
    value: AttrValue,
}

impl Parse for HtmlAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let key = Ident::parse_any(input)?;
        input.parse::<Token![=]>()?;
        
        let value = if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            AttrValue::Expression(content.parse()?)
        } else {
            AttrValue::Literal(input.parse::<LitStr>()?)
        };

        Ok(Self { key, value })
    }
}

struct HtmlElement {
    tag: Ident,
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
            let tag = Ident::parse_any(input)?;

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
            let close_tag = Ident::parse_any(input)?;
            input.parse::<Token![>]>()?;

            if tag != close_tag {
                return Err(syn::Error::new(close_tag.span(), format!("Mismatched tag. Expected `{}`, found `{}`", tag, close_tag)));
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
        HtmlNode::Text(text) => {
            quote! { oxirast_core::VNode::text(#text) }
        },
        HtmlNode::Expression(expr) => {
            quote! { oxirast_core::VNode::text(&(#expr).to_string()) }
        },
        HtmlNode::Element(el) => {
            let tag = el.tag.to_string();
            let is_custom_component = tag.chars().next().unwrap().is_ascii_uppercase();

            if is_custom_component {
                let component_name = &el.tag;
                
                if el.attributes.is_empty() {
                    return quote! {
                        #component_name()
                    };
                }

                let props_struct_name = syn::Ident::new(&format!("{}Props", component_name), component_name.span());
                
                let props_fields: Vec<_> = el.attributes.iter().map(|attr| {
                    let key = &attr.key;
                    match &attr.value {
                        AttrValue::Literal(lit) => quote! { #key: String::from(#lit) },
                        AttrValue::Expression(expr) => quote! { #key: #expr },
                    }
                }).collect();

                return quote! {
                    #component_name(#props_struct_name {
                        #(#props_fields),*
                    })
                };
            }

            let mut attr_calls = Vec::new();

            for attr in &el.attributes {
                let key = attr.key.to_string(); 
                
                match &attr.value {
                    AttrValue::Literal(lit) => {
                        attr_calls.push(quote! { .attr(#key, #lit) });
                    },
                    AttrValue::Expression(expr) => {
                        // THE MACRO UPGRADE: Catch bind_text and compile it into .bind_text()
                        if key == "bind_text" {
                            attr_calls.push(quote! { .bind_text(#expr) });
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
                oxirast_core::VNode::element(#tag)
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