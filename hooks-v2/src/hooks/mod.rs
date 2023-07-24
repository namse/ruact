mod effect;
mod event;
mod instance;
mod memo;
mod signal;
mod state;

use crossbeam::atomic::AtomicCell;
use effect::*;
pub use event::*;
pub use instance::*;
pub use memo::*;
pub use signal::*;
pub use state::*;
use std::{
    any::{Any, TypeId},
    cell::OnceCell,
    collections::HashSet,
    fmt::Debug,
    sync::{atomic::AtomicUsize, Arc},
};

#[derive(Debug)]
pub(crate) enum ContextFor {
    Mount,
    Event { event_callback: EventCallback },
    Consumed,
}

impl Default for ContextFor {
    fn default() -> Self {
        Self::Consumed
    }
}

pub struct Context {
    context_for: AtomicCell<ContextFor>,
    instance: Arc<ComponentInstance>,
    signal_index: AtomicUsize,
    state_index: AtomicUsize,
    effect_index: AtomicUsize,
    memo_index: AtomicUsize,
    updated_signals: Arc<HashSet<SignalId>>,
}

fn handle_on_event<Event>(context: &Context, on_event: impl FnOnce(Event)) {
    todo!()
}

impl Context {
    pub(crate) fn new(
        context_for: ContextFor,
        instance: Arc<ComponentInstance>,
        updated_signals: Arc<HashSet<SignalId>>,
    ) -> Self {
        Self {
            context_for: AtomicCell::new(context_for),
            instance,
            signal_index: AtomicUsize::new(0),
            state_index: AtomicUsize::new(0),
            effect_index: AtomicUsize::new(0),
            memo_index: AtomicUsize::new(0),
            updated_signals,
        }
    }

    pub fn state<State: Send + Sync + 'static>(
        &self,
        init: impl FnOnce() -> State,
    ) -> (Signal<State>, SetState<State>) {
        handle_state(self, init)
    }

    pub fn effect(&self, name: &'static str, effect: impl FnOnce()) {
        let _ = name;
        handle_effect(self, effect);
    }

    pub fn spec<'a, 'b, C: Component + 'b>(&self, render: impl 'a + FnOnce() -> C) -> ContextDone {
        match self.context_for.take() {
            ContextFor::Mount => {
                let child = render();
                ContextDone::Mount {
                    child: unsafe {
                        std::mem::transmute::<Box<dyn Component>, Box<dyn Component>>(Box::new(
                            child,
                        ))
                        .into()
                    },
                }
            }
            ContextFor::Event { .. } => {
                unreachable!()
            }
            ContextFor::Consumed => unreachable!(),
        }
    }

    pub fn spec_with_event<'me, 'a, 'b, C: Component + 'b, Event: 'static + Send + Sync>(
        &'me self,
        on_event: impl 'a + FnOnce(&Event),
        render: impl 'a + FnOnce(EventContext<Event>) -> C,
    ) -> ContextDone {
        match self.context_for.take() {
            ContextFor::Mount => {
                let event_context = EventContext::new(self.instance.component_id);
                let child = render(event_context);
                ContextDone::Mount {
                    child: unsafe {
                        std::mem::transmute::<Box<dyn Component>, Box<dyn Component>>(Box::new(
                            child,
                        ))
                        .into()
                    },
                }
            }
            ContextFor::Event { event_callback } => {
                assert_eq!(event_callback.component_id, self.instance.component_id);
                let event = Arc::downcast::<Event>(event_callback.event).unwrap();
                on_event(event.as_ref());
                ContextDone::Event
            }
            ContextFor::Consumed => unreachable!(),
        }
    }

    pub fn memo<T: 'static>(&self, memo: impl FnOnce() -> T) -> Signal<'_, T> {
        handle_memo(self, memo)
    }

    fn is_signal_updated(&self, signal_id: SignalId) -> bool {
        self.updated_signals.contains(&signal_id)
    }
}

pub struct EventContext<Event: 'static> {
    component_id: usize,
    _event: std::marker::PhantomData<Event>,
}

impl<Event: 'static + Send + Sync> EventContext<Event> {
    fn new(component_id: usize) -> Self {
        Self {
            component_id,
            _event: std::marker::PhantomData,
        }
    }
    pub fn event(&self, event: Event) -> EventCallback {
        EventCallback {
            component_id: self.component_id,
            event: Arc::new(event),
        }
    }
}

pub enum ContextDone {
    Mount { child: OnceCell<Box<dyn Component>> },
    Event,
    Native { native: Native },
}

#[derive(Debug)]
pub enum Native {
    Button { on_click: EventCallback },
}

impl Debug for ContextDone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextDone::Mount { .. } => write!(f, "ContextDone::Mount"),
            ContextDone::Event => write!(f, "ContextDone::Event"),
            ContextDone::Native { native } => write!(f, "ContextDone::Native({:?})", native),
        }
    }
}

pub trait Component: StaticTypeId + Debug {
    fn component<'a>(&'a self, ctx: &'a Context) -> ContextDone;
}

pub trait StaticTypeId {
    fn type_id(&self) -> TypeId;
}
