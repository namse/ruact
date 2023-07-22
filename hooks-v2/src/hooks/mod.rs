mod effect;
mod state;

use crossbeam::atomic::AtomicCell;
use effect::*;
pub use state::*;
use std::{
    any::Any,
    sync::{atomic::AtomicUsize, Arc},
};

pub struct Context<Event: 'static> {
    id: usize,
    _event: std::marker::PhantomData<Event>,

    state_index: AtomicUsize,
    state_list: AtomicCell<Vec<Arc<dyn Any>>>,

    effect_index: AtomicUsize,
    effect_deps_list: AtomicCell<Vec<Arc<dyn AnyPartialEqClone>>>,
}

fn handle_on_event<Event>(context: &Context<Event>, on_event: impl FnOnce(Event)) {
    todo!()
}

impl<Event: 'static> Context<Event> {
    pub fn use_state<'a, State: Send + Sync + 'static>(
        &'a self,
        init: impl FnOnce() -> State,
    ) -> (&'a State, SetState<State>) {
        handle_use_state(self, init)
    }

    pub fn spec<
        'a,
        Effects: 'a + FnOnce(&EffectContext<'a, Event>),
        OnEvent: 'a + FnOnce(Event),
        Render: 'a + FnOnce(&mut Renderer),
    >(
        &'a self,
        effects: Effects,
        on_event: OnEvent,
        render: Render,
    ) -> ContextDone {
        let effect_context = EffectContext { context: self };
        effects(&effect_context);

        ContextDone {}
    }

    pub fn event(&self, event: Event) -> EventCallback {
        EventCallback {
            component_id: self.id,
            event: Arc::new(event),
        }
    }
}

pub struct EffectContext<'a, Event: 'static> {
    context: &'a Context<Event>,
}

impl<'a, Event> EffectContext<'a, Event> {
    pub fn on<Deps: AnyPartialEqClone + 'static>(
        &'a self,
        name: &'static str,
        deps: &'a Deps,
        effect: impl FnOnce() + 'a,
    ) {
        let _ = name;
        handle_use_effect(self.context, deps, effect);
        todo!()
    }
}

pub struct Renderer {}
impl Renderer {
    pub fn add(&mut self, _component: impl Component) {}
}

pub struct ContextDone {}

pub trait Component {
    type Event;
    fn component(&self, ctx: &Context<Self::Event>) -> ContextDone;
}

#[derive(Clone)]
pub struct EventCallback {
    component_id: usize,
    event: Arc<dyn Any>,
}
impl EventCallback {
    pub(crate) fn call(&self) {
        todo!()
    }
}
