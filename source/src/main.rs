mod any_clone_partial_eq;
mod closure;
mod component;
mod foo;
mod render;

use any_clone_partial_eq::{AnyClonePartialEq, AnyClonePartialEqBox};
use closure::Closure;
use component::*;
use rayon::prelude::*;
use render::*;
use std::{
    any::Any,
    cell::RefCell,
    collections::{BTreeSet, HashSet, VecDeque},
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::{atomic::AtomicBool, Arc, Mutex, OnceLock},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

static SET_STATE_TX: OnceLock<UnboundedSender<SetStateInvoked>> = OnceLock::new();

#[tokio::main]
async fn main() {
    real_main().await;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    items: Vec<usize>,
}
impl Key {
    fn root(key_item: usize) -> Key {
        Key {
            items: vec![key_item],
        }
    }

    fn depth(&self) -> usize {
        self.items.len()
    }

    fn push(&self, key_item: usize) -> Key {
        let mut items = self.items.clone();
        items.push(key_item);
        Key { items }
    }
}

trait ComponentTree {
    fn get_child(&self, key_item: usize) -> Option<&ComponentTreeNode>;
    fn get_child_mut(&mut self, key_item: usize) -> Option<&mut ComponentTreeNode>;
    fn put_child_component(&mut self, key_item: usize, component: Box<dyn Component>);
}
struct ComponentTreeHead {
    children: Vec<ComponentTreeNode>,
}
impl ComponentTreeHead {
    fn new() -> ComponentTreeHead {
        ComponentTreeHead {
            children: Vec::new(),
        }
    }
}
unsafe impl Send for ComponentTreeHead {}
unsafe impl Sync for ComponentTreeHead {}

impl ComponentTree for ComponentTreeHead {
    fn get_child(&self, key_item: usize) -> Option<&ComponentTreeNode> {
        self.children.get(key_item)
    }

    fn get_child_mut(&mut self, key_item: usize) -> Option<&mut ComponentTreeNode> {
        self.children.get_mut(key_item)
    }

    fn put_child_component(&mut self, key_item: usize, component: Box<dyn Component>) {
        match self.children.get_mut(key_item) {
            Some(child) => child.component = component,
            None => {
                assert_eq!(key_item, self.children.len());
                self.children
                    .push(ComponentTreeNode::new(component, Key::root(key_item)))
            }
        }
    }
}

struct ComponentTreeNode {
    component: Box<dyn Component>,
    children: Vec<ComponentTreeNode>,
    key: Key,
}
unsafe impl Send for ComponentTreeNode {}
unsafe impl Sync for ComponentTreeNode {}

impl ComponentTree for ComponentTreeNode {
    fn get_child(&self, key_item: usize) -> Option<&Self> {
        self.children.get(key_item)
    }
    fn get_child_mut(&mut self, key_item: usize) -> Option<&mut Self> {
        self.children.get_mut(key_item)
    }
    fn put_child_component(&mut self, key_item: usize, component: Box<dyn Component>) {
        match self.children.get_mut(key_item) {
            Some(child) => child.component = component,
            None => {
                assert_eq!(key_item, self.children.len());
                self.children
                    .push(ComponentTreeNode::new(component, self.key.push(key_item)))
            }
        }
    }
}

impl ComponentTreeNode {
    fn new(component: Box<dyn Component>, key: Key) -> ComponentTreeNode {
        ComponentTreeNode {
            component,
            children: Vec::new(),
            key,
        }
    }

    fn key(&self) -> &Key {
        &self.key
    }

    fn update(&mut self, source: Arc<Source>) {
        let render = self.component.render(Render::new());
        for (index, child) in render.into_children().enumerate() {
            if let Some(prev_child_node) = self.get_child_mut(index) {
                if !prev_child_node.component.equals(child.as_ref()) {
                    prev_child_node.component = child;
                    invoke_update(prev_child_node.key(), source.clone());
                }
            } else {
                self.put_child_component(index, child);
                invoke_update(&self.key().push(index), source.clone());
            }
        }
    }
}

fn start(root: impl Component + 'static) {
    let key = Key::root(0);
    mount_to(&key, root);
}

fn mount_to(key: &Key, component: impl Component + 'static) {
    put_to_node(&key, component);
    invoke_update(&key, Arc::new(()));
}

fn invoke_update(key: &Key, source: Arc<Source>) {
    UPDATE_REQUEST_TX
        .get()
        .unwrap()
        .send(UpdateRequest {
            key: key.clone(),
            source,
        })
        .unwrap();
}

static COMPONENT_TREE: OnceLock<Arc<Mutex<ComponentTreeHead>>> = OnceLock::new();

fn put_to_node(key: &Key, component: impl Component + 'static) {
    let mut head = COMPONENT_TREE
        .get_or_init(|| Arc::new(Mutex::new(ComponentTreeHead::new())))
        .lock()
        .unwrap();
    let (last_key_item, rest) = key.items.split_last().unwrap();
    let mut node: &mut dyn ComponentTree = head.deref_mut();
    for key_item in rest {
        node = node.get_child_mut(*key_item).unwrap();
    }
    node.put_child_component(*last_key_item, Box::new(component));
}

#[derive(Debug, Clone)]
struct UpdateRequest {
    key: Key,
    source: Arc<()>,
}

type Source = ();

static UPDATE_REQUEST_TX: OnceLock<UnboundedSender<UpdateRequest>> = OnceLock::new();

async fn update_task(mut update_request_rx: UnboundedReceiver<UpdateRequest>) {
    let mut update_merged_queue: VecDeque<UpdateRequest> = VecDeque::new();

    let mut prev_handling_source: Option<Arc<Source>> = None;
    while let Some(request) = update_request_rx.recv().await {
        println!("update_task: {:?}", request);
        let handling_source = prev_handling_source
            .take()
            .unwrap_or_else(|| request.source.clone());

        insert_request(&mut update_merged_queue, request);

        loop {
            while let Ok(request) = update_request_rx.try_recv() {
                println!("update_task: {:?}", request);
                insert_request(&mut update_merged_queue, request);
            }
            match pop_request_of_source(&mut update_merged_queue, &handling_source) {
                Some(request) => {
                    update_component(request);

                    if is_update_of_source_finished(&handling_source) {
                        draw();
                        break;
                    }

                    tokio::task::yield_now().await;
                }
                None => {
                    prev_handling_source = Some(handling_source);
                    break;
                }
            }
        }
    }

    fn insert_request(merged_queue: &mut VecDeque<UpdateRequest>, request: UpdateRequest) {
        match merged_queue
            .iter_mut()
            .find(|merged| merged.key == request.key)
        {
            Some(merged) => {
                return;
            }
            None => {
                merged_queue.push_back(request);
            }
        }
    }
}

fn draw() {
    println!("draw!");
}

fn is_update_of_source_finished(source: &Arc<Source>) -> bool {
    Arc::strong_count(source) == 1
}

fn pop_request_of_source(
    update_merged_queue: &mut VecDeque<UpdateRequest>,
    source: &Arc<Source>,
) -> Option<UpdateRequest> {
    update_merged_queue
        .iter()
        .position(|merged| Arc::ptr_eq(&merged.source, source))
        .map(|index| update_merged_queue.remove(index).unwrap())
}

fn update_component(request: UpdateRequest) {
    let mut head = COMPONENT_TREE.get().unwrap().lock().unwrap();
    let (first, rest) = request.key.items.split_first().unwrap();
    let mut node: &mut ComponentTreeNode = head.get_child_mut(*first).unwrap();
    for key_item in rest {
        node = node
            .get_child_mut(*key_item)
            .expect(&format!("ComponentTreeNode not found: {:?}", request.key));
    }
    node.update(request.source);
}

async fn real_main() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    SET_STATE_TX.get_or_init(|| tx);

    let (update_request_tx, update_request_rx) = tokio::sync::mpsc::unbounded_channel();
    UPDATE_REQUEST_TX.get_or_init(|| update_request_tx);
    let task = tokio::spawn(update_task(update_request_rx));

    start(foo::Foo { b: 1 });

    task.await.unwrap();

    // let head = 0;
    // let stored_elements: Arc<Mutex<Vec<Option<Element>>>> = Arc::new(Mutex::new(vec![]));
    // let (state_updated_watch_tx, mut state_updated_watch_rx) = tokio::sync::watch::channel(());
    // let updated_component_ids = Arc::new(Mutex::new(BTreeSet::new()));

    // {
    //     let mut stored_elements = stored_elements.lock().unwrap();
    //     mount(root, head, &mut stored_elements);
    //     match stored_elements.last().unwrap() {
    //         Some(element) => match element {
    //             Element::Native(native) => {
    //                 let button = native.as_any().downcast_ref::<Button>().unwrap();
    //                 button.on_click.invoke(())
    //             }
    //             Element::Component { props, render } => todo!(),
    //         },
    //         None => todo!(),
    //     };
    // }

    // let state_update_task = tokio::spawn({
    //     let updated_component_ids = updated_component_ids.clone();
    //     async move {
    //         while let Some(set_state) = rx.recv().await {
    //             let component_id = set_state.component_id;
    //             set_state.update_state();
    //             {
    //                 updated_component_ids.lock().unwrap().insert(component_id);
    //             }
    //             state_updated_watch_tx.send(()).unwrap();
    //         }
    //     }
    // });

    // let re_render_task = tokio::spawn({
    //     let updated_component_ids = updated_component_ids.clone();
    //     async move {
    //         while state_updated_watch_rx.changed().await.is_ok() {
    //             while let Some(component_id) = { updated_component_ids.lock().unwrap().pop_first() }
    //             {
    //                 let mut stored_elements = stored_elements.lock().unwrap();
    //                 re_render(component_id, &mut stored_elements);

    //                 for (i, element) in stored_elements.iter().enumerate() {
    //                     let debug_text = match element {
    //                         Some(element) => match element {
    //                             Element::Native(_) => "Native",
    //                             Element::Component { props, render } => "Component",
    //                         },
    //                         None => "None",
    //                     };
    //                     println!("{}: {:?}", i, debug_text);
    //                 }

    //                 match stored_elements.last().unwrap() {
    //                     Some(element) => match element {
    //                         Element::Native(native) => {
    //                             let button = native.as_any().downcast_ref::<Button>().unwrap();
    //                             button.on_click.invoke(())
    //                         }
    //                         Element::Component { props, render } => todo!(),
    //                     },
    //                     None => todo!(),
    //                 };
    //             }
    //         }
    //     }
    // });

    // tokio::try_join!(state_update_task, re_render_task).unwrap();
}

fn re_render(head: usize, stored_elements: &mut Vec<Option<Element>>) {
    let stored = stored_elements.get(head).unwrap().as_ref().unwrap();
    match stored {
        Element::Native(_) => {}
        Element::Component { props, render } => {
            STORED_STATE_INDEX.with(|stored_state_index| {
                stored_state_index.replace(0);
            });
            let next = (render)(props);
            mount(next, head + 1, stored_elements)
        }
    }
}

fn mount(element: Element, head: usize, stored_elements: &mut Vec<Option<Element>>) {
    // 여기서 직접 재렌더링 하지 말고, updated_component_ids에 넣거나 해서 여러 update에 대해 한번만 렌더링 되게 해야해.
    let stored = {
        while stored_elements.len() <= head {
            stored_elements.push(None);
        }
        stored_elements.get_mut(head).unwrap()
    };

    let rerender_type = get_rerender_type(stored.as_ref(), &element);
    match rerender_type {
        RerenderType::Full => {
            *stored = Some(element);
        }
        RerenderType::Props => {
            let mut prev = stored.as_mut().unwrap();
            match (&mut prev, element) {
                (Element::Native(_), Element::Native(_)) => todo!(),
                (Element::Native(_), Element::Component { props, render }) => todo!(),
                (
                    Element::Component {
                        props: prev_props,
                        render: prev_render,
                    },
                    Element::Native(_),
                ) => todo!(),
                (
                    Element::Component {
                        props: prev_props,
                        render: _,
                    },
                    Element::Component {
                        props: next_props,
                        render: _,
                    },
                ) => {
                    *prev_props = next_props;
                }
            }
        }
        RerenderType::None => {}
    }

    if rerender_type != RerenderType::None {
        match stored.as_ref().unwrap() {
            Element::Native(_) => {}
            Element::Component { props, render } => {
                STORED_STATE_INDEX.with(|stored_state_index| {
                    stored_state_index.replace(0);
                });
                let next = (render)(props);
                mount(next, head + 1, stored_elements)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum RerenderType {
    Full,
    Props,
    None,
}

fn get_rerender_type(prev: Option<&Element>, next: &Element) -> RerenderType {
    let Some(prev) = prev else {
        return RerenderType::Full;
    };

    match (prev, next) {
        (Element::Native(_), Element::Native(_)) => {
            return RerenderType::Full;
        }
        (Element::Native(_), Element::Component { props, render }) => todo!(),
        (
            Element::Component {
                props: prev_props,
                render: prev_render,
            },
            Element::Native(_),
        ) => todo!(),
        (
            Element::Component {
                props: prev_props,
                render: prev_render,
            },
            Element::Component {
                props: next_props,
                render: next_render,
            },
        ) => {
            if prev_render as *const _ != next_render as *const _ {
                return RerenderType::Full;
            }

            if prev_props.equals(next_props.as_ref()) {
                return RerenderType::Props;
            }
        }
    }

    RerenderType::None
}

pub enum Element {
    Native(Box<dyn AnyClonePartialEq>),
    Component {
        props: Box<dyn AnyClonePartialEq>,
        render: fn(&Box<dyn AnyClonePartialEq>) -> Element,
    },
}

unsafe impl Sync for Element {}
unsafe impl Send for Element {}

struct ComponentContext {
    effect_index: usize,
    effect_deps: Vec<AnyClonePartialEqBox>,
}
impl ComponentContext {
    fn get_last_deps(&self) -> Option<&AnyClonePartialEqBox> {
        self.effect_deps.get(self.effect_index)
    }
    fn save_deps(&mut self, deps: AnyClonePartialEqBox) {
        if self.effect_deps.len() <= self.effect_index {
            self.effect_deps.push(deps);
        } else {
            self.effect_deps[self.effect_index] = deps;
        }
    }
    fn increase_effect_index(&mut self) {
        self.effect_index += 1;
    }
}

static mut COMPONENT_CONTEXT: Option<ComponentContext> = None;

fn effect(name: &str, callback: impl FnOnce() + 'static, deps: impl AnyClonePartialEq + 'static) {
    let context = unsafe { COMPONENT_CONTEXT.as_mut().unwrap() };
    let last_deps = context.get_last_deps();
    if let Some(last_deps) = last_deps {
        if last_deps.as_ref().equals(&deps) {
            return;
        }
    }
    println!("effect: {}", name);
    callback();
    context.save_deps(deps.boxing());
    context.increase_effect_index();
}

#[derive(Debug, Clone, PartialEq)]
struct Button {
    pub on_click: Closure<()>,
}

impl Component for Button {
    fn to_element(self) -> Element {
        Element::Native(Box::new(self))
    }

    fn render(&self, render: Render) -> Render {
        println!("render button");
        render
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct SetState<T> {
    _marker: std::marker::PhantomData<T>,
    component_id: usize,
    state_index: usize,
}

struct SetStateInvoked {
    component_id: usize,
    state_index: usize,
    state: Arc<dyn AnyClonePartialEq>,
}
impl Debug for SetStateInvoked {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetStateInvoked")
            .field("component_id", &self.component_id)
            .field("state_index", &self.state_index)
            .finish()
    }
}
impl SetStateInvoked {
    fn update_state(self) {
        STORED_STATES.with(move |state| {
            let mut state = state.borrow_mut();
            println!("self.state_index: {}", self.state_index);
            let state = state.get_mut(self.state_index).unwrap();
            *state = self.state;
        });
    }
}
unsafe impl Send for SetStateInvoked {}
unsafe impl Sync for SetStateInvoked {}

impl<T: 'static + Any + Clone + PartialEq + Debug> SetState<T> {
    fn i(&self, new_state: T) {
        println!(
            "set state invoked, component_id: {}, state_index: {}",
            self.component_id, self.state_index
        );

        SET_STATE_TX
            .get()
            .unwrap()
            .send(SetStateInvoked {
                component_id: self.component_id,
                state_index: self.state_index,
                state: Arc::new(new_state),
            })
            .unwrap();
    }

    fn new(component_id: usize, state_index: usize) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            component_id,
            state_index,
        }
    }
}

thread_local! {
    static COMPONENT_ID: RefCell<usize> = RefCell::new(0);
    static STORED_STATES: RefCell<Vec<Arc<dyn AnyClonePartialEq>>> = RefCell::new(vec![]);
    static STORED_STATE_INDEX: RefCell<usize> = RefCell::new(0);
}

fn state<'a, T: 'static + Any + Clone + PartialEq + Debug>(initial: T) -> (&'a T, SetState<T>) {
    let component_id: usize = COMPONENT_ID.with(|id| id.borrow().clone());
    let state_index: usize = STORED_STATE_INDEX.with(|index| {
        let mut index = index.borrow_mut();
        let ret_index = *index;
        *index += 1;
        ret_index
    });
    let state = STORED_STATES.with(move |state| {
        let mut state = state.borrow_mut();
        if state.get(state_index).is_none() {
            state.insert(state_index, Arc::new(initial));
        }
        state[state_index].clone()
    });
    let state_ptr = Arc::as_ptr(&state);
    let set_state = SetState::new(component_id, state_index);

    let state_ref = unsafe { &*state_ptr };
    (state_ref.as_any().downcast_ref::<T>().unwrap(), set_state)
}

thread_local! {
    static STORED_SIGNALS: RefCell<Vec<Arc<dyn AnyClonePartialEq>>> = RefCell::new(vec![]);

    static STORED_SIGNAL_INDEX: RefCell<usize> = RefCell::new(0);
}
