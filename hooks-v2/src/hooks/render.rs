use super::*;

pub(crate) fn handle_render<'a, C: Component>(
    ctx: &'a Context,
    render: impl 'a + FnOnce() -> C,
) -> Option<OnceCell<Box<dyn Component>>> {
    handle_render_internal(ctx, render)
}

pub(crate) fn handle_render_with_event<'a, C: Component, Event: 'static + Send + Sync>(
    ctx: &'a Context,
    render: impl FnOnce(EventContext<Event>) -> C,
) -> Option<OnceCell<Box<dyn Component>>> {
    handle_render_internal(ctx, || {
        let event_context = EventContext::new(ctx.instance.component_id);
        render(event_context)
    })
}

fn handle_render_internal<'a, C: Component>(
    ctx: &'a Context,
    render: impl 'a + FnOnce() -> C,
) -> Option<OnceCell<Box<dyn Component>>> {
    unsafe {
        let is_first_run = || {
            ctx.instance
                .is_first_render
                .swap(false, std::sync::atomic::Ordering::SeqCst)
        };

        let used_signal_updated = || {
            let render_used_signals = ctx.instance.render_used_signals.as_ptr().as_ref().unwrap();
            ctx.is_used_signal_updated(render_used_signals)
        };

        if is_first_run() || ctx.is_set_state_phase() && used_signal_updated() {
            let child = render();
            let used_signal_ids = take_used_signals();
            let render_used_signals = ctx.instance.render_used_signals.as_ptr().as_mut().unwrap();
            *render_used_signals = used_signal_ids;

            Some(
                std::mem::transmute::<Box<dyn Component>, Box<dyn Component>>(Box::new(child))
                    .into(),
            )
        } else {
            None
        }
    }
}
