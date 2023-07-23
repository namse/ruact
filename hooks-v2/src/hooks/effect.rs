use super::*;

pub(crate) fn handle_effect<'a, Event>(context: &'a Context<Event>, effect: impl FnOnce()) {
    unsafe {
        let effect_used_signals_list = context
            .instance
            .effect_used_signals_list
            .as_ptr()
            .as_mut()
            .unwrap();
        let effect_index = context
            .effect_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let is_first_run = || effect_used_signals_list.len() <= effect_index;

        let used_signal_updated = || {
            let used_signals = effect_used_signals_list.get(effect_index).unwrap();

            used_signals
                .into_iter()
                .any(|signal_id| context.is_signal_updated(*signal_id))
        };

        if is_first_run() || used_signal_updated() {
            effect();
            let used_signal_ids = take_used_signals();
            effect_used_signals_list.insert(effect_index, used_signal_ids);
        }
    }
}
