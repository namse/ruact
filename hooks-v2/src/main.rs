mod hooks;

use hooks::*;

struct MyComponent {
    on_something: EventCallback,
}

enum Event {
    OnClick,
}

impl Component for MyComponent {
    type Event = Event;
    fn component(&self, ctx: &Context<Event>) -> ContextDone {
        let (count, set_count) = ctx.use_state(|| 0);

        ctx.spec(
            |effect| {
                effect.on("Print count on change", count, || {
                    self.on_something.call();
                });
            },
            |event| match event {
                Event::OnClick => set_count.set(count + 1),
            },
            |renderer| {
                renderer.add(Button {
                    // on_click: ctx.event(Event::OnClick),
                    on_click: self.on_something.clone(),
                })
            },
        )
    }
}

struct Button {
    on_click: EventCallback,
}

impl Component for Button {
    type Event = Event;

    fn component(&self, ctx: &Context<Self::Event>) -> ContextDone {
        ctx.spec(
            |effect| {
                effect.on("On button render", &(), || {
                    println!("Button rendered");
                });
            },
            |event| match event {
                Event::OnClick => {}
            },
            |renderer| {
                renderer.add(Button {
                    on_click: ctx.event(Event::OnClick),
                })
            },
        )
    }
}

fn main() {}
