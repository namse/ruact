use super::*;
use crate::closure::closure;

#[derive(Debug, Clone, PartialEq)]
pub struct Foo {
    pub b: i32,
}

fn render(foo: &Foo) -> Element {
    let (a, set_a) = state::<i32>(15);
    println!("state a: {}", a);
    /*
    rsx!(
        Button {
            value: 5,
        }(
            Button {
                value: 5,
            },
            Button {
                value: 5,
            },
        ),
    )
    */
    Button {
        on_click: closure((*a, set_a), |_, (a, set_a)| {
            set_a.i(a + 1);
        }),
    }
    .component()
}

impl Component for Foo {
    fn render(&self) -> Element {
        render(self)
    }
    fn component(self) -> Element {
        Element::Component {
            props: Box::new(self),
            render: |props| {
                unsafe {
                    match COMPONENT_CONTEXT.as_mut() {
                        Some(context) => {
                            println!("context.effect_index: {}", context.effect_index);
                            context.effect_index = 0;
                        }
                        None => {
                            COMPONENT_CONTEXT = Some(ComponentContext {
                                effect_index: 0,
                                effect_deps: vec![],
                            });
                        }
                    }
                }

                props.as_any().downcast_ref::<Foo>().unwrap().render()
            },
        }
    }
}
