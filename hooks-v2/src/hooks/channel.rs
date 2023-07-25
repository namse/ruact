use super::*;
use std::sync::OnceLock;

pub(crate) static RX: OnceLock<crossbeam::channel::Receiver<Item>> = OnceLock::new();
pub(crate) static TX: OnceLock<crossbeam::channel::Sender<Item>> = OnceLock::new();

#[derive(Debug)]
pub(crate) enum Item {
    SetStateItem(SetStateItem),
    EventCallback(EventCallback),
}

pub(crate) fn init() {
    let (tx, rx) = crossbeam::channel::unbounded();
    RX.set(rx).unwrap();
    TX.set(tx).unwrap();
}

pub(crate) fn send(item: Item) {
    println!("Channel Send: {:#?}", item);
    TX.get().unwrap().send(item).unwrap();
}
