use super::*;

pub(crate) fn handle_memo<'a, Event, T: 'static>(
    context: &'a Context<Event>,
    memo: impl FnOnce() -> T,
) -> Signal<'a, T> {
    unsafe {
        let memo_value_list = context.instance.memo_value_list.as_ptr().as_mut().unwrap();
        let memo_used_signals_list = context
            .instance
            .memo_used_signals_list
            .as_ptr()
            .as_mut()
            .unwrap();
        let memo_index = context
            .memo_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let is_first_run = || memo_value_list.len() <= memo_index;

        let used_signal_updated = || {
            let used_signals = memo_used_signals_list.get(memo_index).unwrap();

            used_signals
                .into_iter()
                .any(|signal_id| context.is_signal_updated(*signal_id))
        };

        if is_first_run() || used_signal_updated() {
            let value = memo();
            memo_value_list.insert(memo_index, Arc::new(value));

            let used_signal_ids = take_used_signals();
            memo_used_signals_list.insert(memo_index, used_signal_ids);
        }

        let value = &*(Arc::into_raw(memo_value_list.get(memo_index).unwrap().clone()) as *const T);
        let signal_id = SignalId {
            component_id: context.instance.component_id,
            signal_index: context
                .signal_index
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        };

        Signal::new(value, signal_id)
    }
}
