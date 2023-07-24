use super::*;

pub struct SetState<State> {
    _state: std::marker::PhantomData<State>,
}

impl<State> SetState<State> {
    pub fn set(self, state: State) {}
    pub fn mutate(self, mutate: impl FnOnce(&mut State)) {}
}

pub(crate) fn handle_state<'a, State: Send + Sync + 'static>(
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

        let state = &*(Arc::into_raw(state_list.get(state_index).unwrap().clone()) as *const State);

        let signal_id = SignalId {
            component_id: context.instance.component_id,
            signal_index: context
                .signal_index
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        };

        let set_state = SetState {
            _state: std::marker::PhantomData,
        };

        let signal = Signal::new(state, signal_id);

        (signal, set_state)
    }
}
