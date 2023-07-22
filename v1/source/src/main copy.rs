use std::{
    any::Any,
    sync::{atomic::AtomicBool, Arc},
};

fn main() {
    println!("Hello, world!");
}

pub struct Component {
    state: Box<dyn Any>,
    props: Box<dyn Any>,
    on_event: fn(Box<dyn Any>, Box<dyn Any>, Box<dyn Any>) -> (Box<dyn Any>, bool),
    render: fn(Box<dyn Any>, Box<dyn Any>),
}

trait ComponentProps {
    fn component(self) -> Component;
}

mod foo {
    use super::*;
    struct State {}
    pub struct Props {}
    enum Event {}
    fn on_event(state: State, props: &Props, event: Event) -> Option<State> {
        None
    }
    fn render(state: &State, props: &Props) {
        bar::Props {};
        bar::component(bar::Props {});
    }
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

fn set_interval(event: impl Any, interval: usize) {}
fn fetch<Event: Any>(
    url: impl ToString,
    on_response_event: impl FnOnce(String) -> Event + 'static,
    deps: impl Any,
) {
}

struct Element {
    on_mount: Box<dyn FnOnce()>,
    on_unmount: Box<dyn FnOnce()>,
}

fn zz(tx: std::sync::mpsc::Sender<String>) {
    let str = format!("123");
    let a = || tx.send(str.clone());
    let b = || tx.send(str.clone());
}

mod bar {
    use super::*;
    struct State {
        a: i32,
    }
    pub struct Props {}
    enum Event {
        OnTick,
        OnResponse(String),
    }
    fn on_event(state: &mut DirtyCheck<State>, props: &Props, event: Event) {
        match event {
            Event::OnTick => todo!(),
            Event::OnResponse(response) => todo!(),
        }
    }
    fn render(state: &State, props: &Props) {
        set_interval(Event::OnTick, 1000);
        fetch("http://localhost:8080", |res| Event::OnResponse(res), ());
    }
    fn new_state(props: &Props) -> State {
        State { a: 0 }
    }

    pub fn component(props: Props) -> Component {
        props.component()
    }

    impl ComponentProps for Props {
        fn component(self) -> Component {
            Component {
                state: Box::new(new_state(&self)),
                props: Box::new(self),
                on_event: |state, props, event| {
                    let state = state.downcast::<State>().unwrap();
                    let mut dirty_check_state = DirtyCheck {
                        item: *state,
                        dirty: false,
                    };
                    on_event(
                        &mut dirty_check_state,
                        props.downcast::<Props>().unwrap().as_ref(),
                        *event.downcast::<Event>().unwrap(),
                    );

                    (Box::new(dirty_check_state.item), dirty_check_state.dirty)
                },
                render: |state, props| {
                    render(
                        state.downcast::<State>().unwrap().as_ref(),
                        props.downcast::<Props>().unwrap().as_ref(),
                    )
                },
            }
        }
    }
}
