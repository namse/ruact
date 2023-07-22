use super::*;

pub trait AnyEqual {
    fn as_any(&self) -> &dyn Any;
    fn equals(&self, _: &dyn Component) -> bool;
}

impl<S: 'static + PartialEq> AnyEqual for S {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn equals(&self, other: &dyn Component) -> bool {
        other
            .as_any()
            .downcast_ref::<S>()
            .map_or(false, |a| self == a)
    }
}

pub trait Component: AnyEqual {
    fn render(&self, render: Render) -> Render;
    fn to_element(self) -> Element
    where
        Self: Clone + Any + Debug + PartialEq,
    {
        Element::Component {
            props: Box::new(self),
            render: |props| {
                todo!()
                // let render = Render::new();

                // props
                //     .as_any()
                //     .downcast_ref::<Self>()
                //     .unwrap()
                //     .render(render)
                //     .to_element()
            },
        }
    }
    fn add_to_render(self, render: &mut Render)
    where
        Self: Sized + 'static,
    {
        render.add_component(self);
    }
}

impl<T0, T1> Component for (T0, T1)
where
    T0: Component + Clone + Debug + Any + PartialEq,
    T1: Component + Clone + Debug + Any + PartialEq,
{
    fn render(&self, _render: Render) -> Render {
        unreachable!()
    }
    fn add_to_render(self, render: &mut Render)
    where
        Self: Sized + 'static,
    {
        self.0.add_to_render(render);
        self.1.add_to_render(render);
    }
}

impl<T0, T1, T2> Component for (T0, T1, T2)
where
    T0: Component + Clone + Debug + Any + PartialEq,
    T1: Component + Clone + Debug + Any + PartialEq,
    T2: Component + Clone + Debug + Any + PartialEq,
{
    fn render(&self, _render: Render) -> Render {
        unreachable!()
    }
    fn add_to_render(self, render: &mut Render)
    where
        Self: Sized + 'static,
    {
        self.0.add_to_render(render);
        self.1.add_to_render(render);
        self.2.add_to_render(render);
    }
}
