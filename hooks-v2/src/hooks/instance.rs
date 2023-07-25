use super::*;
use std::{any::TypeId, fmt::Debug, sync::atomic::AtomicBool};

pub(crate) struct ComponentInstance {
    pub(crate) component_id: usize,
    pub(crate) component_type_id: TypeId,
    pub(crate) component_type_name: &'static str,
    pub(crate) state_list: AtomicCell<Vec<Arc<dyn Value>>>,
    pub(crate) effect_used_signals_list: AtomicCell<Vec<Vec<SignalId>>>,
    pub(crate) memo_value_list: AtomicCell<Vec<Arc<dyn Value>>>,
    pub(crate) memo_used_signals_list: AtomicCell<Vec<Vec<SignalId>>>,
    pub(crate) render_used_signals: AtomicCell<Vec<SignalId>>,
    pub(crate) is_first_render: AtomicBool,
}

impl Debug for ComponentInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("ComponentInstance")
                .field("component_id", &self.component_id)
                .field("component_type_id", &self.component_type_id)
                .field("component_type_name", &self.component_type_name)
                .field("state_list", &self.state_list.as_ptr().as_ref().unwrap())
                .field(
                    "effect_used_signals_list",
                    &self.effect_used_signals_list.as_ptr().as_ref().unwrap(),
                )
                .field(
                    "memo_value_list",
                    &self.memo_value_list.as_ptr().as_ref().unwrap(),
                )
                .field(
                    "memo_used_signals_list",
                    &self.memo_used_signals_list.as_ptr().as_ref().unwrap(),
                )
                .finish()
        }
    }
}

impl ComponentInstance {
    pub(crate) fn new(
        component_id: usize,
        component_type_id: TypeId,
        component_type_name: &'static str,
    ) -> Self {
        Self {
            component_id,
            component_type_id,
            component_type_name,
            state_list: AtomicCell::new(Vec::new()),
            effect_used_signals_list: AtomicCell::new(Vec::new()),
            memo_value_list: AtomicCell::new(Vec::new()),
            memo_used_signals_list: AtomicCell::new(Vec::new()),
            render_used_signals: AtomicCell::new(Vec::new()),
            is_first_render: AtomicBool::new(true),
        }
    }
}
