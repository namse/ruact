use super::*;

pub trait AnyPartialEqClone {
    fn eq(&self, other: &dyn Any) -> bool;
    fn clone_arc(&self) -> Arc<dyn AnyPartialEqClone>;
}

impl<T: PartialEq + Clone + 'static> AnyPartialEqClone for T {
    fn eq(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .map(|other| self == other)
            .unwrap_or(false)
    }

    fn clone_arc(&self) -> Arc<dyn AnyPartialEqClone> {
        Arc::new(self.clone())
    }
}

pub(crate) fn handle_use_effect<'a, Event, Deps: AnyPartialEqClone + 'static>(
    context: &'a Context<Event>,
    deps: &'a Deps,
    effect: impl FnOnce(),
) {
    unsafe {
        let effect_deps_list = context.effect_deps_list.as_ptr().as_mut().unwrap();

        let prev_deps = effect_deps_list.get(
            context
                .effect_index
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        );

        if match prev_deps {
            Some(prev_deps) => !prev_deps.eq(deps),
            None => true,
        } {
            effect_deps_list.push(deps.clone_arc());

            effect();
        }
    }
}
