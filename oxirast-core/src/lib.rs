pub use web_sys;
pub use wasm_bindgen;

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Document, Element, Event, Node};
use serde::de::DeserializeOwned;

// ==========================================
// 1. DOM HELPERS
// ==========================================
pub fn document() -> Document { window().expect("No window").document().expect("No document") }

pub fn mount_to_body(element: &Node) {
    document()
        .body()
        .expect("Document should have a body")
        .append_child(element)
        .expect("Failed to append root element to body");
}

pub fn on_event<F>(element: &Element, event_name: &str, mut callback: F) where F: FnMut(Event) + 'static {
    let closure = Closure::wrap(Box::new(move |e: Event| { callback(e); }) as Box<dyn FnMut(_)>);
    element.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref()).expect("Failed");
    closure.forget(); 
}

// ==========================================
// 2. VIRTUAL DOM (With Event Callbacks)
// ==========================================
pub type EventCallback = Rc<RefCell<Box<dyn FnMut(Event)>>>;

#[derive(Clone)]
pub enum VNode { Element(VElement), Text(String) }

#[derive(Clone)]
pub struct VElement {
    pub tag: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<VNode>,
    pub events: HashMap<String, EventCallback>,
    pub bound_text: Option<Signal<String>>, // THE UPGRADE: VDOM can hold a text signal
}

impl VNode {
    pub fn element(tag: &str) -> VElement { 
        VElement { tag: tag.to_string(), attrs: HashMap::new(), children: Vec::new(), events: HashMap::new(), bound_text: None } 
    }
    pub fn text(s: &str) -> VNode { VNode::Text(s.to_string()) }
}

impl VElement {
    pub fn attr(mut self, key: &str, val: &str) -> Self { self.attrs.insert(key.to_string(), val.to_string()); self }
    pub fn on(mut self, event: &str, cb: EventCallback) -> Self { self.events.insert(event.to_string(), cb); self }
    pub fn child(mut self, node: VNode) -> Self { self.children.push(node); self }
    pub fn bind_text(mut self, sig: Signal<String>) -> Self { self.bound_text = Some(sig); self } // THE BINDER
    pub fn build(self) -> VNode { VNode::Element(self) }
}

// ==========================================
// 3. VDOM RENDERER & HYDRATOR
// ==========================================
thread_local! {
    static NEXT_ID: Cell<usize> = Cell::new(1);
    static EVENT_REGISTRY: RefCell<HashMap<String, Vec<Closure<dyn FnMut(Event)>>>> = RefCell::new(HashMap::new());
}

pub fn render_vnode(vnode: &VNode) -> Node {
    let doc = document();
    match vnode {
        VNode::Text(text) => doc.create_text_node(text).into(),
        VNode::Element(vel) => {
            let el = doc.create_element(&vel.tag).unwrap();
            for (k, v) in &vel.attrs { el.set_attribute(k, v).ok(); }
            
            // --- THE COMPILER MAGIC ---
            if let Some(sig) = &vel.bound_text {
                el.set_inner_html(&sig.get()); 
                let el_clone = el.clone();
                sig.subscribe(move |new_val| {
                    el_clone.set_inner_html(new_val); 
                });
            }
            // --------------------------

            if !vel.events.is_empty() {
                let mut closures = Vec::new();
                for (event_name, cb) in &vel.events {
                    let cb_clone = cb.clone();
                    let closure = Closure::wrap(Box::new(move |e: Event| { if let Ok(mut f) = cb_clone.try_borrow_mut() { f(e); } }) as Box<dyn FnMut(_)>);
                    el.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref()).unwrap();
                    closures.push(closure); 
                }
                let id = NEXT_ID.with(|n| { let val = n.get(); n.set(val + 1); val.to_string() });
                el.set_attribute("data-ox-id", &id).unwrap(); 
                EVENT_REGISTRY.with(|reg| reg.borrow_mut().insert(id, closures)); 
            }
            for child in &vel.children { el.append_child(&render_vnode(child)).ok(); }
            el.into()
        }
    }
}

pub fn hydrate_dom(dom_node: &Node, vnode: &VNode) {
    match vnode {
        VNode::Element(vel) => {
            if let Ok(el) = dom_node.clone().dyn_into::<Element>() {
                if !vel.events.is_empty() {
                    let mut closures = Vec::new();
                    for (event_name, cb) in &vel.events {
                        let cb_clone = cb.clone();
                        let closure = Closure::wrap(Box::new(move |e: Event| { if let Ok(mut f) = cb_clone.try_borrow_mut() { f(e); } }) as Box<dyn FnMut(_)>);
                        el.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref()).unwrap();
                        closures.push(closure);
                    }
                    let id = NEXT_ID.with(|n| { let val = n.get(); n.set(val + 1); val.to_string() });
                    el.set_attribute("data-ox-id", &id).unwrap();
                    EVENT_REGISTRY.with(|reg| reg.borrow_mut().insert(id, closures));
                }
                let child_nodes = el.child_nodes();
                for i in 0..vel.children.len() {
                    if let Some(child_dom) = child_nodes.item(i as u32) { hydrate_dom(&child_dom, &vel.children[i]); }
                }
            }
        }
        VNode::Text(_) => {} 
    }
}

pub fn render_to_string(vnode: &VNode) -> String {
    match vnode {
        VNode::Text(text) => text.clone(),
        VNode::Element(el) => {
            let mut attrs = String::new();
            for (k, v) in &el.attrs { attrs.push_str(&format!(" {}=\"{}\"", k, v)); }
            let mut children = String::new();
            for child in &el.children { children.push_str(&render_to_string(child)); }
            format!("<{}{}>{}</{}>", el.tag, attrs, children, el.tag)
        }
    }
}

// ==========================================
// 3.5 GARBAGE COLLECTOR
// ==========================================
pub fn cleanup_node(node: &Node) {
    if let Ok(el) = node.clone().dyn_into::<Element>() {
        if let Some(id) = el.get_attribute("data-ox-id") {
            EVENT_REGISTRY.with(|reg| { reg.borrow_mut().remove(&id); });
        }
        let children = el.child_nodes();
        for i in 0..children.length() { if let Some(child) = children.item(i) { cleanup_node(&child); } }
    }
}

// ==========================================
// 4. VDOM DIFFER
// ==========================================
fn get_child(parent: &Element, index: u32) -> Option<Node> {
    let node: &Node = parent.as_ref();
    node.child_nodes().item(index)
}

pub fn diff(parent: &Element, old: Option<&VNode>, new: Option<&VNode>, index: u32) {
    match (old, new) {
        (None, Some(new_node)) => { parent.append_child(&render_vnode(new_node)).ok(); }
        (Some(_), None) => {
            if let Some(child) = get_child(parent, index) {
                cleanup_node(&child); 
                parent.remove_child(&child).ok();
            }
        }
        (Some(old_node), Some(new_node)) => {
            match (old_node, new_node) {
                (VNode::Text(old_t), VNode::Text(new_t)) => {
                    if old_t != new_t { if let Some(child) = get_child(parent, index) { child.set_node_value(Some(new_t)); } }
                }
                (VNode::Element(old_el), VNode::Element(new_el)) => {
                    if old_el.tag != new_el.tag {
                        if let Some(child) = get_child(parent, index) {
                            cleanup_node(&child); 
                            parent.replace_child(&render_vnode(new_node), &child).ok();
                        }
                        return;
                    }
                    if let Some(child) = get_child(parent, index) {
                        if let Ok(real) = child.dyn_into::<Element>() {
                            for (k, v) in &new_el.attrs { if old_el.attrs.get(k) != Some(v) { real.set_attribute(k, v).ok(); } }
                            for k in old_el.attrs.keys() { if !new_el.attrs.contains_key(k) { real.remove_attribute(k).ok(); } }
                            
                            let is_keyed = new_el.children.first().map_or(false, |c| match c {
                                VNode::Element(e) => e.attrs.contains_key("key"),
                                _ => false,
                            });

                            if is_keyed {
                                let mut old_map = HashMap::new();
                                let child_nodes_list = real.child_nodes();
                                for (i, old_c) in old_el.children.iter().enumerate() {
                                    if let VNode::Element(e) = old_c {
                                        if let Some(key) = e.attrs.get("key") {
                                            if let Some(dom_n) = child_nodes_list.item(i as u32) {
                                                old_map.insert(key.clone(), (old_c.clone(), dom_n));
                                            }
                                        }
                                    }
                                }

                                while let Some(first) = real.first_child() { real.remove_child(&first).ok(); }

                                for new_c in &new_el.children {
                                    if let VNode::Element(new_e) = new_c {
                                        if let Some(key) = new_e.attrs.get("key") {
                                            if let Some((old_c, dom_n)) = old_map.remove(key) {
                                                real.append_child(&dom_n).ok();
                                                let new_index = real.child_nodes().length() - 1;
                                                diff(&real, Some(&old_c), Some(new_c), new_index);
                                            } else { real.append_child(&render_vnode(new_c)).ok(); }
                                        } else { real.append_child(&render_vnode(new_c)).ok(); }
                                    } else { real.append_child(&render_vnode(new_c)).ok(); }
                                }

                                for (_, (_, dom_n)) in old_map { cleanup_node(&dom_n); }
                            } else {
                                let max = old_el.children.len().max(new_el.children.len());
                                for i in 0..max { diff(&real, old_el.children.get(i), new_el.children.get(i), i as u32); }
                            }
                        }
                    }
                }
                _ => {
                    if let Some(child) = get_child(parent, index) {
                        cleanup_node(&child); 
                        parent.replace_child(&render_vnode(new_node), &child).ok();
                    }
                }
            }
        }
        (None, None) => {}
    }
}

// ==========================================
// 5. THE REACTIVITY ENGINE & CONTEXT
// ==========================================
#[derive(Clone)]
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    listeners: Rc<RefCell<Vec<Box<dyn FnMut(&T)>>>>,
}

impl<T: Clone + 'static> Signal<T> {
    pub fn new(initial_value: T) -> Self { Self { value: Rc::new(RefCell::new(initial_value)), listeners: Rc::new(RefCell::new(Vec::new())) } }
    pub fn get(&self) -> T { self.value.borrow().clone() }
    pub fn set(&self, new_value: T) {
        *self.value.borrow_mut() = new_value.clone();
        for listener in self.listeners.borrow_mut().iter_mut() { listener(&new_value); }
    }
    pub fn subscribe<F>(&self, callback: F) where F: FnMut(&T) + 'static { self.listeners.borrow_mut().push(Box::new(callback)); }
}
pub fn use_state<T: Clone + 'static>(initial: T) -> Signal<T> { Signal::new(initial) }

thread_local! { static GLOBAL_CONTEXT: RefCell<HashMap<TypeId, Rc<dyn Any>>> = RefCell::new(HashMap::new()); }
pub fn provide_context<T: 'static>(value: T) { GLOBAL_CONTEXT.with(|ctx| { ctx.borrow_mut().insert(TypeId::of::<T>(), Rc::new(value)); }); }
pub fn use_context<T: Clone + 'static>() -> Option<T> { GLOBAL_CONTEXT.with(|ctx| { ctx.borrow().get(&TypeId::of::<T>()).and_then(|rc| rc.downcast_ref::<T>().cloned()) }) }

// ==========================================
// 6. ROUTING HELPERS & ROUTER
// ==========================================
pub fn current_path() -> String { window().expect("no window").location().pathname().unwrap_or_else(|_| String::from("/")) }
pub fn push_route(path: &str) { window().unwrap().history().unwrap().push_state_with_url(&JsValue::NULL, "", Some(path)).unwrap(); }

pub struct Router {
    root_id: String,
    routes: HashMap<String, fn(Signal<String>) -> VNode>, 
}
impl Router {
    pub fn new(root_id: &str) -> Self { Self { root_id: root_id.to_string(), routes: HashMap::new() } }
    pub fn route(mut self, path: &str, component: fn(Signal<String>) -> VNode) -> Self { self.routes.insert(path.to_string(), component); self }
    pub fn start(self) {
        let route_signal = use_state(current_path());
        let routes = self.routes.clone();
        let root_id = self.root_id.clone();
        let nav_signal = route_signal.clone();
        let is_initial_load = Rc::new(std::cell::Cell::new(true));

        route_signal.subscribe(move |new_path| {
            if current_path() != *new_path { push_route(new_path); }
            let root = document().get_element_by_id(&root_id).expect("Root div not found!");
            
            let page_vnode = match routes.get(new_path.as_str()) {
                Some(component) => component(nav_signal.clone()),
                None => VNode::element("h1").child(VNode::text("404 - Page Not Found")).build()
            };

            if is_initial_load.get() {
                is_initial_load.set(false); 
                if root.child_nodes().length() > 0 {
                    if let Some(first_child) = root.child_nodes().item(0) { hydrate_dom(&first_child, &page_vnode); }
                } else {
                    cleanup_node(&root);
                    root.set_inner_html("");
                    root.append_child(&render_vnode(&page_vnode)).unwrap();
                }
            } else {
                cleanup_node(&root);
                root.set_inner_html("");
                root.append_child(&render_vnode(&page_vnode)).unwrap();
            }
        });

        let nav_signal_for_pop = route_signal.clone();
        let closure = Closure::wrap(Box::new(move |_e: JsValue| { nav_signal_for_pop.set(current_path()); }) as Box<dyn FnMut(JsValue)>);
        window().unwrap().add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();

        route_signal.set(current_path());
    }
}

// ==========================================
// 7. ASYNC FETCH & EFFECTS
// ==========================================
pub fn use_fetch<T>(url: &str) -> (Signal<Option<T>>, Signal<bool>, Signal<Option<String>>)
where T: DeserializeOwned + Clone + 'static, {
    let data: Signal<Option<T>> = use_state(None);
    let is_loading = use_state(true);
    let error: Signal<Option<String>> = use_state(None);

    let data_c = data.clone();
    let loading_c = is_loading.clone();
    let error_c = error.clone();
    let url_s = url.to_string();

    wasm_bindgen_futures::spawn_local(async move {
        match reqwest::get(&url_s).await {
            Ok(resp) => match resp.json::<T>().await {
                Ok(json) => data_c.set(Some(json)),
                Err(_)   => error_c.set(Some("Failed to parse JSON".to_string())),
            },
            Err(e) => error_c.set(Some(e.to_string())),
        }
        loading_c.set(false);
    });
    (data, is_loading, error)
}

pub async fn api_request<Req, Res>(method: &str, url: &str, body: Option<Req>) -> Result<Res, String>
where Req: serde::Serialize, Res: serde::de::DeserializeOwned, {
    let client = reqwest::Client::new();
    let mut builder = match method { "POST" => client.post(url), "PUT" => client.put(url), "DELETE" => client.delete(url), _ => client.get(url) };
    if let Some(b) = body { builder = builder.json(&b); }
    builder.send().await.map_err(|e| e.to_string())?.json::<Res>().await.map_err(|e| e.to_string())
}