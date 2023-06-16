mod any_clone_partial_eq;
mod closure;
mod foo;

use any_clone_partial_eq::{AnyClonePartialEq, AnyClonePartialEqBox};
use closure::Closure;
use crossbeam_channel::{Receiver, Sender};
use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    ops::Deref,
    sync::{Arc, Mutex, OnceLock},
};

static SET_STATE_TX: OnceLock<Sender<SetStateInvoked>> = OnceLock::new();

fn main() {
    let (tx, rx) = crossbeam_channel::unbounded();
    SET_STATE_TX.get_or_init(|| tx);

    let root = foo::Foo { b: 1 }.component();

    let head = 0;
    let mut stored_elements: Vec<Option<Element>> = vec![];

    mount(root, head, &mut stored_elements);

    for (i, element) in stored_elements.iter().enumerate() {
        let debug_text = match element {
            Some(element) => match element {
                Element::Native(_) => "Native",
                Element::Component { props, render } => "Component",
            },
            None => "None",
        };
        println!("{}: {:?}", i, debug_text);
    }

    match stored_elements.last().unwrap() {
        Some(element) => match element {
            Element::Native(native) => {
                let button = native.as_any().downcast_ref::<Button>().unwrap();
                button.on_click.invoke(())
            }
            Element::Component { props, render } => todo!(),
        },
        None => todo!(),
    };

    while let Ok(set_state) = rx.recv() {
        set_state.update_state();
        re_render(set_state.component_id, &mut stored_elements);

        match stored_elements.last().unwrap() {
            Some(element) => match element {
                Element::Native(native) => {
                    let button = native.as_any().downcast_ref::<Button>().unwrap();
                    button.on_click.invoke(())
                }
                Element::Component { props, render } => todo!(),
            },
            None => todo!(),
        };
    }
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

trait Component {
    fn render(&self) -> Element;
    fn component(self) -> Element;
}

struct DirtyCheck<'a, Item> {
    item: &'a mut Item,
    dirty: bool,
}

impl<Item> std::ops::Deref for DirtyCheck<'_, Item> {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<Item> std::ops::DerefMut for DirtyCheck<'_, Item> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.item
    }
}

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
    fn render(&self) -> Element {
        unreachable!()
    }
    fn component(self) -> Element {
        Element::Native(Box::new(self))
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
impl SetStateInvoked {
    fn update_state(&self) {
        STORED_STATES.with(move |state| {
            let mut state = state.borrow_mut();
            let state = state.get_mut(self.state_index).unwrap();
            *state = self.state.clone();
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
