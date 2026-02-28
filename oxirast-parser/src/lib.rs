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
                return Ok(HtmlNode::Element(HtmlElement {
                    tag,
                    attributes,
                    children: Vec::new(),
                }));
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
                return Err(syn::Error::new(
                    close_tag.span(),
                    format!("Mismatched tag. Expected `{}`, found `{}`", tag, close_tag),
                ));
            }

            Ok(HtmlNode::Element(HtmlElement { tag, attributes, children }))
        } else {
            let text: LitStr = input.parse()?;
            Ok(HtmlNode::Text(text))
        }
    }
}

fn generate_node(node: &HtmlNode) -> proc_macro2::TokenStream {
    match node {
        HtmlNode::Text(text) => {
            quote! {
                oxirast_core::document().create_text_node(#text)
            }
        },
        HtmlNode::Element(el) => {
            let tag = el.tag.to_string();
            
            // THE COMPONENT UPGRADE: Check if it starts with an uppercase letter
            let is_custom_component = tag.chars().next().unwrap().is_ascii_uppercase();

            if is_custom_component {
                let component_name = &el.tag;
                // If it's a custom component like <Login />, call the Rust function!
                return quote! {
                    #component_name()
                };
            }

            // STANDARD HTML ELEMENTS: Build the DOM node
            let attrs: Vec<_> = el.attributes.iter().map(|attr| {
                // RESTORED: This is the missing `key` declaration
                let key = attr.key.to_string(); 
                
                match &attr.value {
                    AttrValue::Literal(lit) => quote! { 
                        __el.set_attribute(#key, #lit).unwrap(); 
                    },
                    AttrValue::Expression(expr) => {
                        if key.starts_with("on_") {
                            let event_name = key.replace("on_", "");
                            quote! { 
                                oxirast_core::on_event(&__el, #event_name, #expr); 
                            }
                        } else {
                            quote! { 
                                __el.set_attribute(#key, &#expr.to_string()).unwrap(); 
                            }
                        }
                    }, 
                }
            }).collect();

            let children: Vec<_> = el.children.iter().map(|child| {
                let child_code = generate_node(child);
                quote! {
                    let __child: oxirast_core::web_sys::Node = #child_code.into();
                    __el.append_child(&__child).unwrap();
                }
            }).collect();

            quote! {
                {
                    let __el: oxirast_core::web_sys::Element = oxirast_core::document().create_element(#tag).unwrap();
                    #(#attrs)*
                    #(#children)*
                    __el
                }
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