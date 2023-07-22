use super::*;

pub struct SetState<State> {
    _state: std::marker::PhantomData<State>,
}

impl<State> SetState<State> {
    pub fn set(self, state: State) {}
    pub fn mutate(self, mutate: impl FnOnce(&mut State)) {}
}

pub(crate) fn handle_use_state<'a, Event, State: Send + Sync + 'static>(
    context: &'a Context<Event>,
    init: impl FnOnce() -> State,
) -> (&'a State, SetState<State>) {
    unsafe {
        let state_list = context.state_list.as_ptr().as_mut().unwrap();

        let no_state = state_list.len()
            <= context
                .state_index
                .load(std::sync::atomic::Ordering::SeqCst);

        if no_state {
            let state = init();

            state_list.push(Arc::new(state));
        }

        let state = state_list
            .get(
                context
                    .state_index
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            )
            .unwrap();

        let state = &*(Arc::into_raw(state.clone()) as *const State);
        let set_state = SetState {
            _state: std::marker::PhantomData,
        };

        (state, set_state)
    }
}
