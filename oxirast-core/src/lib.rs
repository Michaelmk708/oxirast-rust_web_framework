pub use wasm_bindgen;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// ==========================================
// 0. ISOMORPHIC ARCHITECTURE (Server vs Client)
// ==========================================
#[cfg(target_arch = "wasm32")]
pub use web_sys;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, Document, Element, Event, Node, HtmlElement};
#[cfg(target_arch = "wasm32")]
use std::cell::Cell;
#[cfg(target_arch = "wasm32")]
use serde::de::DeserializeOwned;

// Dummy types for the Server so the VNode struct compiles universally
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct Event;

// THE FIX: Dummy JsValue so the server compiler doesn't panic
#[cfg(not(target_arch = "wasm32"))]
pub struct JsValue; 

#[cfg(not(target_arch = "wasm32"))]
pub struct Closure<T: ?Sized> { _marker: std::marker::PhantomData<T> }

#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized> Closure<T> { 
    pub fn wrap(_: Box<T>) -> Self { Self { _marker: std::marker::PhantomData } } 
    pub fn as_ref(&self) -> &JsValue { unimplemented!() }
    pub fn forget(self) {} 
}


// ==========================================
// 1. ERROR BOUNDARIES (Client Only)
// ==========================================
#[cfg(target_arch = "wasm32")]
pub fn init_error_boundary() {
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() { *s } 
        else if let Some(s) = panic_info.payload().downcast_ref::<String>() { s.as_str() } 
        else { "Unknown Rust Panic inside WebAssembly" };
        let location = panic_info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_else(|| String::from("Unknown location"));
        let doc = window().unwrap().document().unwrap();
        let body = doc.body().unwrap();
        let overlay = doc.create_element("div").unwrap();
        overlay.set_attribute("style", "position:fixed;top:0;left:0;width:100vw;height:100vh;background:rgba(30,0,0,0.95);color:#ff5555;padding:3rem;z-index:99999;font-family:monospace;").unwrap();
        overlay.set_inner_html(&format!(r#"<h1>⚠️ Oxirast Crash Report</h1><div style="background:#220000;border:1px solid #ff0000;padding:1.5rem;"><h3>Error:</h3><p>{}</p><hr/><h3>Location:</h3><p>{}</p></div>"#, msg, location));
        body.append_child(&overlay).unwrap();
        web_sys::console::error_1(&format!("Oxirast Panic: {} at {}", msg, location).into());
    }));
}

// ==========================================
// 2. ISOMORPHIC VDOM STRUCTURES
// ==========================================
pub type EventCallback = Rc<RefCell<Box<dyn FnMut(Event)>>>;
pub type LifecycleCallback = Rc<RefCell<Box<dyn FnMut()>>>;

#[derive(Clone)]
pub enum VNode { Element(VElement), Text(String) }

#[derive(Clone)]
pub struct VElement {
    pub tag: String,
    pub attrs: HashMap<String, String>,
    pub bound_attrs: HashMap<String, Signal<String>>, 
    pub children: Vec<VNode>,
    pub events: HashMap<String, EventCallback>,
    pub bound_text: Option<Signal<String>>, 
    pub bound_show: Option<Signal<bool>>, 
    pub on_mount: Option<LifecycleCallback>,
    pub on_cleanup: Option<LifecycleCallback>,
}

impl VNode {
    pub fn element(tag: &str) -> VElement { 
        VElement { tag: tag.to_string(), attrs: HashMap::new(), bound_attrs: HashMap::new(), children: Vec::new(), events: HashMap::new(), bound_text: None, bound_show: None, on_mount: None, on_cleanup: None } 
    }
    pub fn text(s: &str) -> VNode { VNode::Text(s.to_string()) }
}

impl VElement {
    pub fn attr(mut self, key: &str, val: &str) -> Self { self.attrs.insert(key.to_string(), val.to_string()); self }
    pub fn bind_attr(mut self, key: &str, sig: Signal<String>) -> Self { self.bound_attrs.insert(key.to_string(), sig); self }
    pub fn on(mut self, event: &str, cb: EventCallback) -> Self { self.events.insert(event.to_string(), cb); self }
    pub fn child(mut self, node: VNode) -> Self { self.children.push(node); self }
    pub fn bind_text(mut self, sig: Signal<String>) -> Self { self.bound_text = Some(sig); self } 
    pub fn bind_show(mut self, sig: Signal<bool>) -> Self { self.bound_show = Some(sig); self } 
    pub fn on_mount(mut self, cb: LifecycleCallback) -> Self { self.on_mount = Some(cb); self }
    pub fn on_cleanup(mut self, cb: LifecycleCallback) -> Self { self.on_cleanup = Some(cb); self }
    pub fn build(self) -> VNode { VNode::Element(self) }
}

// ==========================================
// 3. SSR HTML GENERATOR (Runs Everywhere)
// ==========================================
pub fn render_to_string(vnode: &VNode) -> String {
    match vnode {
        VNode::Text(text) => text.clone(),
        VNode::Element(el) => {
            let mut attrs = String::new();
            for (k, v) in &el.attrs { attrs.push_str(&format!(" {}=\"{}\"", k, v)); }
            for (k, sig) in &el.bound_attrs { attrs.push_str(&format!(" {}=\"{}\"", k, sig.get())); }
            if let Some(sig) = &el.bound_show { if !sig.get() { attrs.push_str(" style=\"display:none;\""); } }
            
            let mut children = String::new();
            if let Some(sig) = &el.bound_text { children.push_str(&sig.get()); } 
            else { for child in &el.children { children.push_str(&render_to_string(child)); } }
            format!("<{}{}>{}</{}>", el.tag, attrs, children, el.tag)
        }
    }
}

// ==========================================
// 4. CLIENT BROWSER DOM ENGINE (WASM ONLY)
// ==========================================
#[cfg(target_arch = "wasm32")]
thread_local! {
    static NEXT_ID: Cell<usize> = Cell::new(1);
    static EVENT_REGISTRY: RefCell<HashMap<String, Vec<Closure<dyn FnMut(Event)>>>> = RefCell::new(HashMap::new());
    static CLEANUP_REGISTRY: RefCell<HashMap<String, LifecycleCallback>> = RefCell::new(HashMap::new());
}

#[cfg(target_arch = "wasm32")]
pub fn document() -> Document { window().expect("No window").document().expect("No document") }
#[cfg(target_arch = "wasm32")]
pub fn mount_to_body(element: &Node) { document().body().unwrap().append_child(element).unwrap(); }

#[cfg(target_arch = "wasm32")]
pub fn render_vnode(vnode: &VNode) -> Node {
    let doc = document();
    match vnode {
        VNode::Text(text) => doc.create_text_node(text).into(),
        VNode::Element(vel) => {
            let el = doc.create_element(&vel.tag).unwrap();
            for (k, v) in &vel.attrs { el.set_attribute(k, v).ok(); }
            for (k, sig) in &vel.bound_attrs { el.set_attribute(k, &sig.get()).ok(); let el_c = el.clone(); let k_c = k.clone(); sig.subscribe(move |v| { el_c.set_attribute(&k_c, v).ok(); }); }
            if let Some(sig) = &vel.bound_text { el.set_text_content(Some(&sig.get())); let el_c = el.clone(); sig.subscribe(move |v| { el_c.set_text_content(Some(v)); }); }
            if let Some(sig) = &vel.bound_show {
                let html_el = el.clone().dyn_into::<HtmlElement>().unwrap(); html_el.style().set_property("display", if sig.get() { "" } else { "none" }).unwrap();
                let el_c = html_el.clone(); sig.subscribe(move |v| { el_c.style().set_property("display", if *v { "" } else { "none" }).unwrap(); });
            }

            let needs_id = !vel.events.is_empty() || vel.on_cleanup.is_some();
            let mut id = String::new();
            if needs_id { id = NEXT_ID.with(|n| { let val = n.get(); n.set(val + 1); val.to_string() }); el.set_attribute("data-ox-id", &id).unwrap(); }

            if !vel.events.is_empty() {
                let mut closures = Vec::new();
                for (name, cb) in &vel.events {
                    let cb_c = cb.clone(); let closure = Closure::wrap(Box::new(move |e: Event| { if let Ok(mut f) = cb_c.try_borrow_mut() { f(e); } }) as Box<dyn FnMut(_)>);
                    el.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref()).unwrap(); closures.push(closure); 
                }
                EVENT_REGISTRY.with(|reg| reg.borrow_mut().insert(id.clone(), closures)); 
            }
            if let Some(m_cb) = &vel.on_mount { if let Ok(mut f) = m_cb.try_borrow_mut() { f(); } }
            if let Some(c_cb) = &vel.on_cleanup { CLEANUP_REGISTRY.with(|reg| reg.borrow_mut().insert(id.clone(), c_cb.clone())); }
            for child in &vel.children { el.append_child(&render_vnode(child)).ok(); }
            el.into()
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn hydrate_dom(dom_node: &Node, vnode: &VNode) {
    match vnode {
        VNode::Element(vel) => {
            if let Ok(el) = dom_node.clone().dyn_into::<Element>() {
                let needs_id = !vel.events.is_empty() || vel.on_cleanup.is_some();
                let mut id = String::new();
                if needs_id { id = NEXT_ID.with(|n| { let val = n.get(); n.set(val + 1); val.to_string() }); let _ = el.set_attribute("data-ox-id", &id); }

                for (k, sig) in &vel.bound_attrs { let el_c = el.clone(); let k_c = k.clone(); sig.subscribe(move |v| { el_c.set_attribute(&k_c, v).ok(); }); }
                if let Some(sig) = &vel.bound_text { let el_c = el.clone(); sig.subscribe(move |v| { el_c.set_text_content(Some(v)); }); }
                if let Some(sig) = &vel.bound_show { if let Ok(html_el) = el.clone().dyn_into::<HtmlElement>() { let el_c = html_el.clone(); sig.subscribe(move |v| { el_c.style().set_property("display", if *v { "" } else { "none" }).unwrap(); }); } }

                if !vel.events.is_empty() {
                    let mut closures = Vec::new();
                    for (name, cb) in &vel.events {
                        let cb_c = cb.clone(); let closure = Closure::wrap(Box::new(move |e: Event| { if let Ok(mut f) = cb_c.try_borrow_mut() { f(e); } }) as Box<dyn FnMut(_)>);
                        el.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref()).unwrap(); closures.push(closure);
                    }
                    EVENT_REGISTRY.with(|reg| reg.borrow_mut().insert(id.clone(), closures));
                }
                if let Some(m_cb) = &vel.on_mount { if let Ok(mut f) = m_cb.try_borrow_mut() { f(); } }
                if let Some(c_cb) = &vel.on_cleanup { CLEANUP_REGISTRY.with(|reg| reg.borrow_mut().insert(id.clone(), c_cb.clone())); }

                let child_nodes = el.child_nodes();
                for i in 0..vel.children.len() { if let Some(child_dom) = child_nodes.item(i as u32) { hydrate_dom(&child_dom, &vel.children[i]); } }
            }
        }
        VNode::Text(_) => {} 
    }
}

#[cfg(target_arch = "wasm32")]
pub fn cleanup_node(node: &Node) {
    if let Ok(el) = node.clone().dyn_into::<Element>() {
        if let Some(id) = el.get_attribute("data-ox-id") { 
            EVENT_REGISTRY.with(|reg| { reg.borrow_mut().remove(&id); }); 
            CLEANUP_REGISTRY.with(|reg| { if let Some(c_cb) = reg.borrow_mut().remove(&id) { if let Ok(mut f) = c_cb.try_borrow_mut() { f(); } } });
        }
        let children = el.child_nodes();
        for i in 0..children.length() { if let Some(child) = children.item(i) { cleanup_node(&child); } }
    }
}

// ==========================================
// 5. REACTIVITY (Runs Everywhere)
// ==========================================
#[derive(Clone)]
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    listeners: Rc<RefCell<Vec<Box<dyn FnMut(&T)>>>>,
}

impl<T: Clone + 'static> Signal<T> {
    pub fn new(initial_value: T) -> Self { Self { value: Rc::new(RefCell::new(initial_value)), listeners: Rc::new(RefCell::new(Vec::new())) } }
    pub fn get(&self) -> T { self.value.borrow().clone() }
    pub fn set(&self, new_value: T) { *self.value.borrow_mut() = new_value.clone(); for listener in self.listeners.borrow_mut().iter_mut() { listener(&new_value); } }
    pub fn subscribe<F>(&self, callback: F) where F: FnMut(&T) + 'static { self.listeners.borrow_mut().push(Box::new(callback)); }
}
pub fn use_state<T: Clone + 'static>(initial: T) -> Signal<T> { Signal::new(initial) }
pub fn use_memo<T: Clone + 'static, U: Clone + 'static, F: Fn(&T) -> U + 'static>(dep: &Signal<T>, calc: F) -> Signal<U> {
    let memo_sig = use_state(calc(&dep.get())); let memo_clone = memo_sig.clone(); dep.subscribe(move |v| { memo_clone.set(calc(v)); }); memo_sig
}
pub fn use_effect<T: Clone + 'static, F: FnMut(&T) + 'static>(dep: &Signal<T>, mut effect: F) { effect(&dep.get()); dep.subscribe(effect); }

thread_local! { static GLOBAL_CONTEXT: RefCell<HashMap<TypeId, Rc<dyn Any>>> = RefCell::new(HashMap::new()); }
pub fn provide_context<T: 'static>(value: T) { GLOBAL_CONTEXT.with(|ctx| { ctx.borrow_mut().insert(TypeId::of::<T>(), Rc::new(value)); }); }
pub fn use_context<T: Clone + 'static>() -> Option<T> { GLOBAL_CONTEXT.with(|ctx| { ctx.borrow().get(&TypeId::of::<T>()).and_then(|rc| rc.downcast_ref::<T>().cloned()) }) }


// ==========================================
// 6. ROUTER & SSR (Isomorphic)
// ==========================================
thread_local! { static ROUTE_PARAMS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new()); }
pub fn use_params() -> HashMap<String, String> { ROUTE_PARAMS.with(|p| p.borrow().clone()) }

#[cfg(target_arch = "wasm32")]
pub fn use_query() -> HashMap<String, String> {
    let search = window().unwrap().location().search().unwrap_or_default();
    let mut query = HashMap::new();
    if search.starts_with('?') {
        for pair in search[1..].split('&') {
            if pair.is_empty() { continue; }
            let mut parts = pair.splitn(2, '=');
            query.insert(parts.next().unwrap_or_default().to_string(), parts.next().unwrap_or_default().to_string());
        }
    }
    query
}

#[cfg(target_arch = "wasm32")]
pub fn current_path() -> String { window().expect("no window").location().pathname().unwrap_or_else(|_| String::from("/")) }
#[cfg(target_arch = "wasm32")]
pub fn push_route(path: &str) { window().unwrap().history().unwrap().push_state_with_url(&JsValue::NULL, "", Some(path)).unwrap(); }

#[derive(Clone)]
pub struct RouteInfo {
    pub component: fn(Signal<String>) -> VNode,
    pub guard: Option<(fn() -> bool, String)>, 
}

pub struct Router {
    root_id: String,
    routes: HashMap<String, RouteInfo>, 
}

impl Router {
    pub fn new(root_id: &str) -> Self { Self { root_id: root_id.to_string(), routes: HashMap::new() } }
    
    pub fn route(mut self, path: &str, component: fn(Signal<String>) -> VNode) -> Self { 
        self.routes.insert(path.to_string(), RouteInfo { component, guard: None }); self 
    }
    pub fn guarded_route(mut self, path: &str, component: fn(Signal<String>) -> VNode, guard: fn() -> bool, redirect: &str) -> Self {
        self.routes.insert(path.to_string(), RouteInfo { component, guard: Some((guard, redirect.to_string())) }); self
    }

    // --- NEW: SERVER-SIDE RENDERING HOOK ---
    // Backend Axum servers call this to instantly generate the requested HTML page!
    pub fn render_route_to_string(&self, request_path: &str) -> String {
        let dummy_nav = use_state(request_path.to_string());
        for (route_path, info) in self.routes.iter() {
            if route_path == request_path {
                let vnode = (info.component)(dummy_nav);
                return render_to_string(&vnode);
            }
        }
        String::from("<h1>404 Not Found</h1>")
    }
    
    // Client-side Browser Boot
    #[cfg(target_arch = "wasm32")]
    pub fn start(self) {
        init_error_boundary();
        let route_signal = use_state(current_path());
        let routes = self.routes.clone();
        let root_id = self.root_id.clone();
        let nav_signal = route_signal.clone();
        let is_initial_load = Rc::new(std::cell::Cell::new(true));

        route_signal.subscribe(move |new_path| {
            if current_path() != *new_path { push_route(new_path); }
            let root = document().get_element_by_id(&root_id).expect("Root div not found!");
            
            let mut matched_route = None;
            let mut extracted_params = HashMap::new();

            for (route_path, info) in routes.iter() {
                let route_segments: Vec<&str> = route_path.split('/').collect();
                let url_segments: Vec<&str> = new_path.split('/').collect();
                let mut is_match = true;
                let mut local_params = HashMap::new();

                for (i, rs) in route_segments.iter().enumerate() {
                    if *rs == "*" { local_params.insert("wildcard".to_string(), url_segments[i..].join("/")); break; }
                    if i >= url_segments.len() { is_match = false; break; }
                    if rs.starts_with(':') { local_params.insert(rs[1..].to_string(), url_segments[i].to_string()); } 
                    else if *rs != url_segments[i] { is_match = false; break; }
                }
                if is_match && (route_segments.last() == Some(&"*") || route_segments.len() == url_segments.len()) {
                    matched_route = Some(info); extracted_params = local_params; break;
                }
            }

            ROUTE_PARAMS.with(|p| { *p.borrow_mut() = extracted_params; });

            let page_vnode = match matched_route {
                Some(info) => {
                    if let Some((guard_fn, redirect)) = &info.guard { if !guard_fn() { nav_signal.set(redirect.clone()); return; } }
                    (info.component)(nav_signal.clone())
                },
                None => VNode::element("h1").child(VNode::text("404 Not Found")).build()
            };

            if is_initial_load.get() {
                is_initial_load.set(false); 
                if root.child_nodes().length() > 0 { if let Some(first) = root.child_nodes().item(0) { hydrate_dom(&first, &page_vnode); } } 
                else { cleanup_node(&root); root.set_inner_html(""); root.append_child(&render_vnode(&page_vnode)).unwrap(); }
            } else { cleanup_node(&root); root.set_inner_html(""); root.append_child(&render_vnode(&page_vnode)).unwrap(); }
        });

        let nav_signal_for_pop = route_signal.clone();
        let closure = Closure::wrap(Box::new(move |_e: JsValue| { nav_signal_for_pop.set(current_path()); }) as Box<dyn FnMut(JsValue)>);
        window().unwrap().add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
        route_signal.set(current_path());
    }
}

// ==========================================
// 7. ASYNC FETCH (Client Only for now)
// ==========================================
#[cfg(target_arch = "wasm32")]
pub fn use_fetch<T>(url: &str) -> (Signal<Option<T>>, Signal<bool>, Signal<Option<String>>)
where T: DeserializeOwned + Clone + 'static, {
    let data: Signal<Option<T>> = use_state(None);
    let is_loading = use_state(true);
    let error: Signal<Option<String>> = use_state(None);

    let data_c = data.clone(); let loading_c = is_loading.clone(); let error_c = error.clone(); let url_s = url.to_string();

    wasm_bindgen_futures::spawn_local(async move {
        match reqwest::get(&url_s).await {
            Ok(resp) => match resp.json::<T>().await { Ok(json) => data_c.set(Some(json)), Err(_) => error_c.set(Some("Parse Error".to_string())), },
            Err(e) => error_c.set(Some(e.to_string())),
        }
        loading_c.set(false);
    });
    (data, is_loading, error)
}


// ==========================================
// 8. WEB3 WALLET ENGINE (The Magic Bridge)
// ==========================================
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = "
    export async function connect_evm() {
        if (window.ethereum) {
            const accounts = await window.ethereum.request({ method: 'eth_requestAccounts' });
            return accounts[0];
        }
        throw new Error('MetaMask not found');
    }
    export async function connect_sol() {
        if (window.solana && window.solana.isPhantom) {
            const resp = await window.solana.connect();
            return resp.publicKey.toString();
        }
        throw new Error('Phantom wallet not found');
    }
")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn connect_evm() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn connect_sol() -> Result<JsValue, JsValue>;
}

#[cfg(target_arch = "wasm32")]
pub fn use_wallet(chain: &str) -> (Signal<Option<String>>, Signal<bool>, Rc<dyn Fn()>) {
    let address: Signal<Option<String>> = use_state(None);
    let is_connecting = use_state(false);
    
    let addr_clone = address.clone();
    let loading_clone = is_connecting.clone();
    let chain_type = chain.to_string();

    let connect_fn = Rc::new(move || {
        let addr_c = addr_clone.clone();
        let load_c = loading_clone.clone();
        let ct = chain_type.clone();
        
        load_c.set(true);
        
        wasm_bindgen_futures::spawn_local(async move {
            let result = if ct == "solana" { connect_sol().await } else { connect_evm().await };
            match result {
                Ok(val) => if let Some(s) = val.as_string() { addr_c.set(Some(s)); },
                Err(e) => web_sys::console::error_1(&e),
            }
            load_c.set(false);
        });
    });

    (address, is_connecting, connect_fn)
}