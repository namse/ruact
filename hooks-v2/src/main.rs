mod hooks;

use hooks::*;
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

struct MyComponent {}

enum Event {
    OnClick,
}

impl Component for MyComponent {
    fn component<'a>(&self, ctx: &'a Context) -> ContextDone<'a> {
        let (count, set_count) = ctx.state(|| 0);
        let fibo = ctx.memo(|| get_fibo(*count));
        let text = ctx.memo(|| format!("Count: {}, Fibo: {}", *count, *fibo));

        ctx.spec_with_event(
            |event| match event {
                Event::OnClick => set_count.set(*count + 1),
            },
            |ctx| Button {
                text,
                on_click: ctx.event(Event::OnClick),
            },
        )
    }
}

impl StaticTypeId for MyComponent {
    fn type_id(&self) -> TypeId {
        TypeId::of::<MyComponent>()
    }
}

mod without_event {
    use super::*;
    struct MyComponent {
        on_something: EventCallback,
    }

    enum Event {
        OnClick,
    }

    impl Component for MyComponent {
        fn component<'a>(&self, ctx: &'a Context) -> ContextDone<'a> {
            let (count, set_count) = ctx.state(|| 0);

            let fibo = ctx.memo(|| get_fibo(*count));

            let text = ctx.memo(|| format!("Count: {}, Fibo: {}", *count, *fibo));

            ctx.spec(|| Button {
                text,
                on_click: self.on_something.clone(),
            })
        }
    }

    impl StaticTypeId for MyComponent {
        fn type_id(&self) -> TypeId {
            TypeId::of::<MyComponent>()
        }
    }
}

fn get_fibo(x: u32) -> u32 {
    if x == 0 {
        return 0;
    }
    if x == 1 {
        return 1;
    }
    get_fibo(x - 1) + get_fibo(x - 2)
}

struct Button<'a> {
    text: Signal<'a, String>,
    on_click: EventCallback,
}

impl StaticTypeId for Button<'_> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<Button<'static>>()
    }
}

impl Component for Button<'_> {
    fn component<'a>(&self, ctx: &'a Context) -> ContextDone<'a> {
        ctx.effect("Print text on text effect", || {
            if self.text.on_effect() {
                println!("Count changed");
            }
        });

        ctx.effect("On button render", || {
            println!("Button rendered");
        });

        ContextDone::Native {
            native: Native::Button {
                on_click: self.on_click.clone(),
            },
        }
    }
}

trait Object: Debug {
    fn get_next<'a>(&'a self) -> Option<Arc<dyn 'a + Object>>;
}

impl Object for () {
    fn get_next<'a>(&'a self) -> Option<Arc<dyn 'a + Object>> {
        todo!()
    }
}

#[derive(Debug)]
struct Holder<'lifetime> {
    objects: Vec<Arc<dyn Object + 'lifetime>>,
}

struct ComponentHolder<'a> {
    id: usize,
    component: Arc<dyn 'a + Object>,
    children_ids: Vec<usize>,
}

struct ComponentHolderMap<'a> {
    inner: HashMap<usize, ComponentHolder<'a>>,
}

impl<'a> ComponentHolderMap<'a> {
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
    fn insert(&mut self, component_holder: ComponentHolder<'a>) {
        self.inner.insert(component_holder.id, component_holder);
    }

    fn get_mut(&mut self, id: &usize) -> Option<&mut ComponentHolder<'a>> {
        self.inner.get_mut(id)
    }
}

fn run() {
    let mut component_holder_map = ComponentHolderMap::new();
    let root = ();
    expand_component_tree(root, &mut component_holder_map);

    let (tx, rx) = std::sync::mpsc::channel::<EventCallback>();
    while let Ok(event_callback) = rx.recv() {
        handle_event(event_callback, &mut component_holder_map)
    }
}

fn handle_event(event_callback: EventCallback, component_holder_map: &mut ComponentHolderMap<'_>) {
    let Some(component) = component_holder_map.get_mut(&event_callback.component_id) else {
        println!("Component not found");
        return;
    };

    // component.
}

fn expand_component_tree<'a>(
    root: impl Object + 'a,
    component_holder_map: &mut ComponentHolderMap<'a>,
) {
    let id = 0;
    let children = vec![()];
    let children_ids = vec![1];
    let component_holder = ComponentHolder {
        id,
        component: Arc::new(root),
        children_ids,
    };
    component_holder_map.insert(component_holder);
    for child in children {
        expand_component_tree(child, component_holder_map);
    }
}

impl<'lifetime> Holder<'lifetime> {
    fn new() -> Self {
        Self { objects: vec![] }
    }
    fn run(self: &'_ mut Self, object: Arc<dyn 'lifetime + Object>) {
        self.objects.push(object.clone());

        let me_ref = unsafe { &*Arc::as_ptr(&object) };

        let next = me_ref.get_next();
        if let Some(next) = next {
            self.run(next);
        }
    }
}

fn main() {
    let root = MyComponent {};
    start(root);
}

fn start<T: Component + 'static>(component: T) {
    init_event_channel();
    let holder = next(Box::new(component));

    // match native {
    //     Native::Button { on_click } => {
    //         println!("Button clicked");
    //         on_click.call();
    //     }
    // }

    // while let Ok(event) = EVENT_RX.get().unwrap().recv() {
    //     println!("Event Recv: {:#?}", event);
    // }

    struct ComponentHolder<'a> {
        component: Box<dyn Component + 'a>,
        component_instance: ComponentInstance,
        children: Vec<Child<'a>>,
    }

    enum Child<'a> {
        Component { component: ComponentHolder<'a> },
        Native { native: Native },
    }

    fn next<'a>(component: Box<dyn Component + 'a>) -> ComponentHolder<'a> {
        let type_id = component.as_ref().type_id();
        let component_instance = ComponentInstance::new(0, type_id);

        let mut holder = ComponentHolder {
            component,
            component_instance,
            children: vec![],
        };

        {
            let updated_signals = HashSet::new();
            let context = Context::new(
                ContextFor::Mount,
                &holder.component_instance,
                &updated_signals,
            );

            let done = holder.component.component(&context);

            println!("instance: {:#?}", holder.component_instance);
            println!("done: {:#?}", done);

            match done {
                ContextDone::Mount { child } => {
                    holder.children.push(Child::Component {
                        component: next(child),
                    });
                }
                ContextDone::Event => unreachable!(),
                ContextDone::Native { native } => {
                    println!("Native. Done!");
                    holder.children.push(Child::Native { native });
                }
            }
        }

        holder
        // match done {
        //     ContextDone::Mount { child } =>
        //     // next(child)
        //     {
        //         todo!()
        //     }
        //     ContextDone::Event => unreachable!(),
        //     ContextDone::Native { native } => {
        //         println!("Native. Done!");
        //         native
        //     }
        // }
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
