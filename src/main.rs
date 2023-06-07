mod any_clone_partial_eq;

use any_clone_partial_eq::{AnyClonePartialEq, AnyClonePartialEqBox};
use crossbeam_channel::{Receiver, Sender};
use std::any::Any;

fn main() {
    init_tx_rx();

    let root = foo::Props { b: 1 };

    let component = root.component();

    let rendered = (component.render)(component.state, component.props, component.dispatch);

    unsafe {
        let event_message = RX.as_ref().unwrap().recv().unwrap();
        println!("event id: {:?}", event_message.id);
    }
}

pub struct Component {
    state: Box<dyn Any>,
    props: Box<dyn Any>,
    on_event: fn(Box<dyn Any>, Box<dyn Any>, Box<dyn Any>) -> (Box<dyn Any>, bool),
    render: fn(Box<dyn Any>, Box<dyn Any>, Dispatch) -> Rendered,
    dispatch: Dispatch,
}

trait ComponentProps {
    fn component(self) -> Component;
}

struct DirtyCheck<Item> {
    item: Item,
    dirty: bool,
}

impl<Item> std::ops::Deref for DirtyCheck<Item> {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<Item> std::ops::DerefMut for DirtyCheck<Item> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.item
    }
}

struct EventMessage {
    event: Box<dyn Any + Send + Sync>,
    id: usize,
}

static mut TX: Option<Sender<EventMessage>> = None;
static mut RX: Option<Receiver<EventMessage>> = None;

fn init_tx_rx() {
    unsafe {
        if TX.is_none() {
            let (tx, rx) = crossbeam_channel::unbounded();
            TX = Some(tx);
            RX = Some(rx);
        }
    }
}

#[derive(Clone, Copy)]
struct Dispatch {
    id: usize,
}
impl Dispatch {
    fn call(&self, event: impl Any + Send + Sync) {
        unsafe {
            TX.as_ref()
                .unwrap()
                .send(EventMessage {
                    event: Box::new(event),
                    id: self.id,
                })
                .unwrap();
        }
    }
}

struct ComponentContext {
    effect_index: usize,
    effect_deps: Vec<AnyClonePartialEqBox>,
}
impl ComponentContext {
    fn get_last_deps(&self) -> Option<&AnyClonePartialEqBox> {
        if self.effect_index == 0 {
            return None;
        }
        self.effect_deps.get(self.effect_index - 1)
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
    println!("effect: {}", name);
    let context = unsafe {
        if COMPONENT_CONTEXT.is_none() {
            COMPONENT_CONTEXT = Some(ComponentContext {
                effect_index: 0,
                effect_deps: vec![],
            });
        }
        COMPONENT_CONTEXT.as_mut().unwrap()
    };
    let last_deps = context.get_last_deps();
    if let Some(last_deps) = last_deps {
        if deps.equals(last_deps) {
            return;
        }
    }
    callback();
    context.save_deps(deps.boxing());
    context.increase_effect_index();
}

struct Rendered {}
fn render(props: impl ComponentProps) -> Rendered {
    Rendered {}
}

mod foo {
    use super::*;
    struct State {
        a: i32,
    }
    pub struct Props {
        pub b: i32,
    }
    enum Event {
        OnTick,
    }
    impl State {
        fn mount(props: &Props) -> State {
            State { a: props.b }
        }
        fn on_event(state: &mut DirtyCheck<State>, _props: &Props, event: Event) {
            match event {
                Event::OnTick => {
                    state.a += 1;
                }
            }
        }
        fn render(state: &State, props: &Props, dispatch: Dispatch) -> Rendered {
            println!("render foo. state.a: {}, props.b: {}", state.a, props.b);
            effect(
                "on mount",
                move || {
                    dispatch.call(Event::OnTick);
                },
                (),
            );

            render(Button {})
        }
    }

    impl ComponentProps for Props {
        fn component(self) -> Component {
            static mut ID: usize = 0;
            let dispatch = Dispatch {
                id: unsafe {
                    ID += 1;
                    ID
                },
            };
            Component {
                state: Box::new(State::mount(&self)),
                props: Box::new(self),
                on_event: |state, props, event| {
                    let state = state.downcast::<State>().unwrap();
                    let mut dirty_check_state = DirtyCheck {
                        item: *state,
                        dirty: false,
                    };
                    State::on_event(
                        &mut dirty_check_state,
                        props.downcast::<Props>().unwrap().as_ref(),
                        *event.downcast::<Event>().unwrap(),
                    );

                    println!("dirty_check_state.dirty: {}", dirty_check_state.dirty);

                    (Box::new(dirty_check_state.item), dirty_check_state.dirty)
                },
                render: |state, props, dispatch| {
                    State::render(
                        state.downcast::<State>().unwrap().as_ref(),
                        props.downcast::<Props>().unwrap().as_ref(),
                        dispatch,
                    )
                },
                dispatch,
            }
        }
    }
}

struct Button {}

impl ComponentProps for Button {
    fn component(self) -> Component {
        todo!()
    }
}