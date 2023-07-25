use super::*;

pub(crate) fn handle_memo<'a, T: 'static + Debug + Send + Sync>(
    ctx: &'a Context,
    memo: impl FnOnce() -> T,
) -> Signal<'a, T> {
    unsafe {
        let memo_value_list = ctx.instance.memo_value_list.as_ptr().as_mut().unwrap();
        let memo_used_signals_list = ctx
            .instance
            .memo_used_signals_list
            .as_ptr()
            .as_mut()
            .unwrap();
        let memo_index = ctx
            .memo_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let is_first_run = || memo_value_list.len() <= memo_index;

        let used_signal_updated = || {
            let used_signals = memo_used_signals_list.get(memo_index).unwrap();
            ctx.is_used_signal_updated(used_signals)
        };

        if is_first_run() || ctx.is_set_state_phase() && used_signal_updated() {
            println!("memo index: {}", memo_index);
            let value = Arc::new(memo());
            update_or_push(memo_value_list, memo_index, value);

            let used_signal_ids = take_used_signals();
            update_or_push(memo_used_signals_list, memo_index, used_signal_ids);

            println!("memo_used_signals_list: {:?}", memo_used_signals_list);
        }

        let value = &*(Arc::into_raw(memo_value_list.get(memo_index).unwrap().clone()) as *const T);
        let signal_id = SignalId {
            component_id: ctx.instance.component_id,
            signal_index: ctx
                .signal_index
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        };

        Signal::new(value, signal_id)
    }
}
