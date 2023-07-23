mod effect;
mod instance;
mod memo;
mod signal;
mod state;

use crossbeam::atomic::AtomicCell;
use effect::*;
pub use instance::*;
pub use memo::*;
pub use signal::*;
pub use state::*;
use std::{
    any::Any,
    collections::HashSet,
    fmt::Debug,
    sync::{atomic::AtomicUsize, Arc},
};

#[derive(Debug)]
pub(crate) enum ContextFor {
    Mount,
    Event { event: Box<dyn Any> },
    Consumed,
}

impl Default for ContextFor {
    fn default() -> Self {
        Self::Consumed
    }
}

pub struct Context<'ctx, Event: 'static> {
    context_for: AtomicCell<ContextFor>,
    instance: &'ctx ComponentInstance,
    _event: std::marker::PhantomData<Event>,
    signal_index: AtomicUsize,
    state_index: AtomicUsize,
    effect_index: AtomicUsize,
    memo_index: AtomicUsize,
    updated_signals: &'ctx HashSet<SignalId>,
}

fn handle_on_event<Event>(context: &Context<Event>, on_event: impl FnOnce(Event)) {
    todo!()
}

impl<'ctx, Event: 'static> Context<'ctx, Event> {
    pub(crate) fn new(
        context_for: ContextFor,
        instance: &'ctx ComponentInstance,
        updated_signals: &'ctx HashSet<SignalId>,
    ) -> Self {
        Self {
            context_for: AtomicCell::new(context_for),
            instance,
            _event: std::marker::PhantomData,
            signal_index: AtomicUsize::new(0),
            state_index: AtomicUsize::new(0),
            effect_index: AtomicUsize::new(0),
            memo_index: AtomicUsize::new(0),
            updated_signals,
        }
    }

    pub fn state<State: Send + Sync + 'static>(
        &'ctx self,
        init: impl FnOnce() -> State,
    ) -> (Signal<State>, SetState<State>) {
        handle_state(self, init)
    }

    pub fn effect(&'ctx self, name: &'static str, effect: impl FnOnce() + 'ctx) {
        let _ = name;
        handle_effect(self, effect);
    }

    pub fn spec<'a, C: AnyComponent + 'ctx>(
        &'ctx self,
        on_event: impl 'a + FnOnce(Event),
        render: impl 'a + FnOnce() -> C,
    ) -> ContextDone {
        match self.context_for.take() {
            ContextFor::Mount => {
                let child = render();
                ContextDone::Mount {
                    child: Box::new(child),
                }
            }
            ContextFor::Event { event } => {
                on_event(*event.downcast().unwrap());
                ContextDone::Event
            }
            ContextFor::Consumed => unreachable!(),
        }
    }

    pub fn event(&'ctx self, event: Event) -> EventCallback {
        EventCallback {
            component_id: self.instance.component_id,
            event: Arc::new(event),
        }
    }

    pub fn memo<T: 'static>(&'ctx self, memo: impl FnOnce() -> T) -> Signal<'ctx, T> {
        handle_memo(self, memo)
    }

    fn is_signal_updated(&self, signal_id: SignalId) -> bool {
        self.updated_signals.contains(&signal_id)
    }
}

pub struct Renderer {}
impl Renderer {
    pub fn add(&mut self, _component: impl Component) {}
}

pub enum ContextDone<'a> {
    Mount { child: Box<dyn AnyComponent + 'a> },
    Event,
    Native,
}

impl Debug for ContextDone<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextDone::Mount { .. } => write!(f, "ContextDone::Mount"),
            ContextDone::Event => write!(f, "ContextDone::Event"),
            ContextDone::Native => write!(f, "ContextDone::Native"),
        }
    }
}

pub trait AnyComponent {
    fn mount(&self);
}

pub trait Component {
    type Event;
    fn component<'a>(&self, ctx: &'a Context<Self::Event>) -> ContextDone<'a>;
}

#[derive(Clone)]
pub struct EventCallback {
    pub(crate) component_id: usize,
    pub(crate) event: Arc<dyn Any>,
}
impl EventCallback {
    pub(crate) fn call(&self) {
        todo!()
    }
}
