mod hooks;

use hooks::*;
use std::{any::TypeId, fmt::Debug};

#[derive(Debug)]
struct MyComponent {}

enum Event {
    OnClick,
}

impl Component for MyComponent {
    fn component<'a>(&'a self, ctx: &'a Context) -> ContextDone {
        let (count, set_count) = ctx.state(|| 0);
        let fibo = ctx.memo(|| get_fibo(*count));
        let text = ctx.memo(|| format!("Count: {}, Fibo: {}", *count, *fibo));

        ctx.render_with_event(
            |event| match event {
                Event::OnClick => {
                    println!("Clicked");
                    set_count.mutate(|count| *count += 1)
                }
            },
            |ctx| Button {
                text,
                on_click: ctx.event(Event::OnClick),
            },
        )
    }
}

impl StaticType for MyComponent {
    fn static_type_id(&self) -> TypeId {
        TypeId::of::<MyComponent>()
    }
}

mod without_event {
    use super::*;

    #[derive(Debug)]
    struct MyComponent {
        on_something: EventCallback,
    }

    enum Event {
        OnClick,
    }

    impl Component for MyComponent {
        fn component<'a>(&'a self, ctx: &'a Context) -> ContextDone {
            let (count, set_count) = ctx.state(|| 0);

            let fibo = ctx.memo(|| get_fibo(*count));

            let text = ctx.memo(|| format!("Count: {}, Fibo: {}", *count, *fibo));

            ctx.render(|| Button {
                text,
                on_click: self.on_something.clone(),
            })
        }
    }

    impl StaticType for MyComponent {
        fn static_type_id(&self) -> TypeId {
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

#[derive(Debug)]
struct Button<'a> {
    text: Signal<'a, String>,
    on_click: EventCallback,
}

impl StaticType for Button<'_> {
    fn static_type_id(&self) -> TypeId {
        TypeId::of::<Button<'static>>()
    }
}

impl Component for Button<'_> {
    fn component<'a>(&'a self, ctx: &'a Context) -> ContextDone {
        ctx.effect("Print text on text effect", || {
            if self.text.on_effect() {
                println!("Count changed");
            }
        });

        ctx.effect("On button render", || {
            println!("Button rendered");
        });

        ctx.render(|| Native::Button {
            on_click: self.on_click.clone(),
        })
    }
}

fn main() {
    let root = MyComponent {};
    start(root);
}
