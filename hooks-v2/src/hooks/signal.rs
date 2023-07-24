use std::{cell::RefCell, collections::HashSet};

thread_local! {
    pub(crate) static USED_SIGNAL_IDS: RefCell<HashSet<SignalId>> = RefCell::new(HashSet::new());
}

pub(crate) fn take_used_signals() -> Vec<SignalId> {
    USED_SIGNAL_IDS.with(|ids| {
        let mut ids = ids.borrow_mut();
        ids.drain().collect()
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct SignalId {
    pub component_id: usize,
    pub signal_index: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Signal<'a, T> {
    id: SignalId,
    value: &'a T,
}

impl<'a, T> Signal<'a, T> {
    pub(crate) fn new(value: &'a T, id: SignalId) -> Self {
        Self { value, id }
    }
    fn subscribe(&self) {
        USED_SIGNAL_IDS.with(|ids| {
            let mut ids = ids.borrow_mut();
            ids.insert(self.id);
        });
    }
    pub fn on_effect(&'a self) -> bool {
        self.subscribe();
        true
    }
}

impl<T> std::ops::Deref for Signal<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.subscribe();
        self.value
    }
}
