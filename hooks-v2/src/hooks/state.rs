use super::*;

pub(crate) enum SetStateItem {
    Set {
        signal_id: SignalId,
        value: Box<dyn Value>,
    },
    Mutate {
        signal_id: SignalId,
        mutate: Box<dyn FnOnce(&mut (dyn Value)) + Send + Sync>,
    },
}

impl Debug for SetStateItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetStateItem::Set { signal_id, value } => {
                write!(
                    f,
                    "SetStateItem::Set {{ signal_id: {:?}, value: {:?} }}",
                    signal_id, value
                )
            }
            SetStateItem::Mutate {
                signal_id,
                mutate: _,
            } => {
                write!(f, "SetStateItem::Mutate {{ signal_id: {:?} }}", signal_id,)
            }
        }
    }
}

pub struct SetState<State: 'static + Debug + Send + Sync> {
    signal_id: SignalId,
    _state: std::marker::PhantomData<State>,
}

impl<State: 'static + Debug + Send + Sync> SetState<State> {
    pub fn set(self, state: State) {
        channel::send(channel::Item::SetStateItem(SetStateItem::Set {
            signal_id: self.signal_id,
            value: Box::new(state),
        }));
    }
    pub fn mutate(self, mutate: impl FnOnce(&mut State) + Send + Sync + 'static) {
        channel::send(channel::Item::SetStateItem(SetStateItem::Mutate {
            signal_id: self.signal_id,
            mutate: Box::new(move |state| {
                let state = state.as_any_mut().downcast_mut::<State>().unwrap();
                println!("mutate before: {:?}", state);
                mutate(state);
                println!("mutate after: {:?}", state);
            }),
        }));
    }
}

pub(crate) fn handle_state<'a, State: Send + Sync + Debug + 'static>(
    context: &'a Context,
    init: impl FnOnce() -> State,
) -> (Signal<'a, State>, SetState<State>) {
    unsafe {
        let state_list = context.instance.state_list.as_ptr().as_mut().unwrap();
        let state_index = context
            .state_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let no_state = state_list.len() <= state_index;

        if no_state {
            let state = init();

            state_list.push(Arc::new(state));
        }

        let state = &*(Arc::as_ptr(state_list.get(state_index).unwrap()) as *const State);

        let signal_id = SignalId {
            component_id: context.instance.component_id,
            signal_index: context
                .signal_index
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        };

        let set_state = SetState {
            signal_id,
            _state: std::marker::PhantomData,
        };

        let signal = Signal::new(state, signal_id);

        (signal, set_state)
    }
}
