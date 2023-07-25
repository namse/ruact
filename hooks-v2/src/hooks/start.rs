use super::*;

pub fn start<T: Component + 'static>(component: T) {
    channel::init();

    let mut root_holder: ComponentHolder =
        mount_visit(OnceCell::from(Box::new(component) as Box<dyn Component>));

    println!("--- visit ---");
    visit(&root_holder, &|holder| {
        let component = holder.component.get().unwrap().as_ref();
        if component.static_type_id() == TypeId::of::<Native>() {
            let native = component.native();
            match native {
                Native::Button { on_click } => {
                    on_click.call();
                    on_click.call();
                    on_click.call();
                    on_click.call();
                    on_click.call();
                }
            }
        }
    });

    println!("--- visit done ---");
    println!("root_holder: {:#?}", root_holder);

    while let Ok(item) = channel::RX.get().unwrap().recv() {
        println!("Channel Recv: {:#?}", item);
        match item {
            Item::SetStateItem(set_state_item) => {
                let signal_id = match set_state_item {
                    SetStateItem::Set { signal_id, .. } => signal_id,
                    SetStateItem::Mutate { signal_id, .. } => signal_id,
                };

                let component = find_component_by_id(&root_holder, signal_id.component_id);
                if let Some(component) = component {
                    unsafe {
                        let state_list = component
                            .component_instance
                            .state_list
                            .as_ptr()
                            .as_mut()
                            .unwrap();

                        let state = state_list.get_mut(signal_id.signal_index).unwrap();
                        match set_state_item {
                            SetStateItem::Set { value, .. } => {
                                *state = value.into();
                            }
                            SetStateItem::Mutate { mutate, .. } => {
                                // panic!
                                let mut inner_state = std::mem::replace(state, Arc::new(()));
                                let count = Arc::strong_count(&inner_state);
                                println!("count: {}", count);
                                let mut_state = Arc::get_mut(&mut inner_state).unwrap();
                                mutate(mut_state);
                                let _ = std::mem::replace(state, inner_state);
                            }
                        }
                    };

                    let updated_signals =
                        Arc::new(AtomicCell::new(vec![signal_id].into_iter().collect()));

                    set_state_propagation(&mut root_holder, updated_signals);
                }
            }
            Item::EventCallback(event_callback) => {
                let holder = find_component_by_id(&root_holder, event_callback.component_id);
                if let Some(holder) = holder {
                    let ctx = Context::new(
                        ContextFor::Event { event_callback },
                        holder.component_instance.clone(),
                    );

                    let ContextDone::NoRender = holder.component.get().unwrap().component(&ctx) else {
                        unreachable!()
                    };
                }
            }
        }
        println!("root_holder: {:#?}", root_holder);
    }

    fn set_state_propagation(
        holder: &mut ComponentHolder,
        updated_signals: Arc<AtomicCell<HashSet<SignalId>>>,
    ) {
        let ctx = Context::new(
            ContextFor::SetState {
                updated_signals: updated_signals.clone(),
            },
            holder.component_instance.clone(),
        );
        let done = holder.component.get().unwrap().component(&ctx);
        match done {
            ContextDone::Rendered { child } => {
                let child_object = child.get().unwrap().as_ref();
                let component_type_id = StaticType::static_type_id(child_object);
                let prev_type_id = holder.component.get().unwrap().as_ref().static_type_id();

                if prev_type_id != component_type_id {
                    let component_instance = Arc::new(ComponentInstance::new(
                        new_component_id(),
                        component_type_id,
                        child_object.static_type_name(),
                    ));

                    let child_holder = ComponentHolder {
                        component: child,
                        component_instance,
                        children: AtomicCell::new(vec![]),
                    };
                    holder.children.store(vec![child_holder]);
                }
            }
            _ => {}
        }

        for child in unsafe { holder.children.as_ptr().as_mut().unwrap() } {
            set_state_propagation(child, updated_signals.clone())
        }
    }

    fn find_component_by_id<'a>(
        root: &'a ComponentHolder,
        component_id: usize,
    ) -> Option<&'a ComponentHolder> {
        find_component(root, &|holder| {
            holder.component_instance.component_id == component_id
        })
    }

    fn find_component<'a>(
        holder: &'a ComponentHolder,
        find: &impl Fn(&ComponentHolder) -> bool,
    ) -> Option<&'a ComponentHolder> {
        if find(holder) {
            Some(holder)
        } else {
            match holder {
                ComponentHolder {
                    component: _,
                    component_instance: _,
                    children,
                } => {
                    for child in unsafe { children.as_ptr().as_ref().unwrap() } {
                        if let Some(component) = find_component(child, find) {
                            return Some(component);
                        }
                    }
                    None
                }
            }
        }
    }

    fn visit(holder: &ComponentHolder, on_component: &impl Fn(&ComponentHolder)) {
        on_component(holder);
        match holder {
            ComponentHolder {
                component: _,
                component_instance: _,
                children,
            } => {
                for child in unsafe { children.as_ptr().as_ref().unwrap() } {
                    visit(child, on_component);
                }
            }
        }
    }

    fn mount_visit(component: OnceCell<Box<dyn Component>>) -> ComponentHolder {
        let component_id = new_component_id();
        let component_object = component.get().unwrap().as_ref();
        let component_type_id = component_object.static_type_id();
        let component_type_name = component_object.static_type_name();
        let component_instance = Arc::new(ComponentInstance::new(
            component_id,
            component_type_id,
            component_type_name,
        ));

        let context = Context::new(ContextFor::Mount, component_instance.clone());

        let done = component.get().unwrap().component(&context);

        ComponentHolder {
            component,
            component_instance,
            children: AtomicCell::new(match done {
                ContextDone::Rendered { child } => vec![mount_visit(child.into())],
                ContextDone::NoRender => vec![],
            }),
        }
    }
}

fn new_component_id() -> usize {
    static COMPONENT_ID: AtomicUsize = AtomicUsize::new(0);
    COMPONENT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

struct ComponentHolder {
    component: OnceCell<Box<dyn Component>>,
    component_instance: Arc<ComponentInstance>,
    children: AtomicCell<Vec<ComponentHolder>>,
}

impl Debug for ComponentHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentHolder")
            .field("component_instance", &self.component_instance)
            .field("children", unsafe { &self.children.as_ptr().as_ref() })
            .finish()
    }
}

/*
- Start
root부터 최말단까지 component instance를 만들어서 저장하고, native component를 시스템에 연결하는 것.
시스템은 native component를 바탕으로 렌더링, I/O 등을 진행.

- OnEvent
시스템이 마우스 클릭과 같은 event를 받으면, 그 이벤트를 처리할 Component를 찾는다.
Component가 없다면 로그를 찍고 넘어간다.
Component가 있다면 그 컴포넌트에 Event를 건네줘서, 이벤트 처리를 할 수 있게 해준다.

- OnSignal
모종의 이유로 signal이 변경되었을 때 발동한다.
Root에서부터 signal을 subscribe한 컴포넌트를 찾아나간다.
컴포넌트 내 signal subscriber를 찾아서 재실행해준다.
참고로, set_state는 곧장 실행되지 않는다. 다음 OnSignal tick때 진행한다.
*/
