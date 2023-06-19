use super::*;
use crate::closure::closure;

struct Atom<T: Send + Sync> {
    value: Mutex<T>,
}
impl<T: Send + Sync> Atom<T> {
    const fn new(init: T) -> Self {
        Self {
            value: Mutex::new(init),
        }
    }
    fn select<R>(&self, selector: impl Fn(&T) -> R + 'static) -> R {
        let value = self.value.lock().unwrap();
        selector(&value)
    }

    fn update(&self, updator: impl FnOnce(&mut T)) {
        let mut value = self.value.lock().unwrap();
        updator(&mut value);
    }
}

static ATOM_B: Atom<i32> = Atom::new(0);
static mut ATOM_B_VALUE: OnceLock<i32> = OnceLock::new();

#[derive(Debug, Clone, PartialEq)]
pub struct Foo {
    pub b: i32,
}

struct Test {
    a: i32,
    b: f32,
}

static ATOM_TEST: Atom<Test> = Atom::new(Test { a: 0, b: 0.0 });

impl Component for Foo {
    fn render(&self, render: Render) -> Render {
        let (a, set_a) = state::<i32>(15);
        let b = ATOM_B.select(|b| *b);
        let c = ATOM_TEST.select(|test| test.b);

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

        render.add(Button {
            on_click: closure((*a, set_a), |_, (a, set_a)| {
                ATOM_TEST.update(|test| {
                    test.a += 1;
                });
                set_a.i(a + 1);
            }),
        })
        //     .add(Button {
        //         on_click: closure((*a, set_a), |_, (a, set_a)| {
        //             ATOM_TEST.update(|test| {
        //                 test.a += 1;
        //             });
        //             set_a.i(a + 1);
        //         }),
        //     })
        //     .add((
        //         Button {
        //             on_click: closure((*a, set_a), |_, (a, set_a)| {
        //                 ATOM_TEST.update(|test| {
        //                     test.a += 1;
        //                 });
        //                 set_a.i(a + 1);
        //             }),
        //         },
        //         Button {
        //             on_click: closure((*a, set_a), |_, (a, set_a)| {
        //                 ATOM_TEST.update(|test| {
        //                     test.a += 1;
        //                 });
        //                 set_a.i(a + 1);
        //             }),
        //         },
        //         Button {
        //             on_click: closure((*a, set_a), |_, (a, set_a)| {
        //                 ATOM_TEST.update(|test| {
        //                     test.a += 1;
        //                 });
        //                 set_a.i(a + 1);
        //             }),
        //         },
        //     ))
    }
}
