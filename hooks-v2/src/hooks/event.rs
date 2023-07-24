use super::*;
use std::sync::OnceLock;

pub(crate) static EVENT_RX: OnceLock<crossbeam::channel::Receiver<EventCallback>> = OnceLock::new();
pub(crate) static EVENT_TX: OnceLock<crossbeam::channel::Sender<EventCallback>> = OnceLock::new();

pub(crate) fn init_event_channel() {
    let (tx, rx) = crossbeam::channel::unbounded();
    EVENT_RX.set(rx).unwrap();
    EVENT_TX.set(tx).unwrap();
}

#[derive(Clone, Debug)]
pub struct EventCallback {
    pub(crate) component_id: usize,
    pub(crate) event: Arc<dyn Any>,
}
unsafe impl Send for EventCallback {}
unsafe impl Sync for EventCallback {}

impl EventCallback {
    pub(crate) fn call(&self) {
        EVENT_TX.get().unwrap().send(self.clone()).unwrap();
    }
}
