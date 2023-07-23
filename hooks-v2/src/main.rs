mod hooks;

use hooks::*;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

struct MyComponent {}

enum Event {
    OnClick,
}

impl Component for MyComponent {
    type Event = Event;
    fn component<'a>(&self, ctx: &'a Context<Event>) -> ContextDone<'a> {
        let (count, set_count) = ctx.state(|| 0);

        let fibo = ctx.memo(|| get_fibo(*count));

        let text = ctx.memo(|| format!("Count: {}, Fibo: {}", *count, *fibo));

        ctx.spec(
            |event| match event {
                Event::OnClick => set_count.set(*count + 1),
            },
            || {
                Button {
                    text,
                    on_click: ctx.event(Event::OnClick),
                    // on_click: self.on_something.clone(),
                }
            },
        )
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

impl Component for Button<'_> {
    type Event = Event;

    fn component<'a>(&self, ctx: &'a Context<Self::Event>) -> ContextDone<'a> {
        ctx.effect("Print text on text effect", || {
            if self.text.on_effect() {
                println!("Count changed");
            }
        });

        ctx.effect("On button render", || {
            println!("Button rendered");
        });

        ContextDone::Native
    }
}

impl AnyComponent for Button<'_> {
    fn mount(&self) {
        let component_instance = ComponentInstance::new(1);
        let updated_signals = HashSet::new();
        let context =
            Context::<'_, Event>::new(ContextFor::Mount, &component_instance, &updated_signals);

        let done: ContextDone<'_> = self.component(&context);

        println!("instance: {:#?}", component_instance);
        println!("done: {:#?}", done);
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
    let component_instance = ComponentInstance::new(0);
    let updated_signals = HashSet::new();
    let context =
        Context::<'_, Event>::new(ContextFor::Mount, &component_instance, &updated_signals);

    let done = root.component(&context);

    println!("instance: {:#?}", component_instance);
    println!("done: {:#?}", done);

    println!("--child--");

    let ContextDone::Mount { child } = done else {
        unreachable!()
    };
    let child = child.mount();
}
