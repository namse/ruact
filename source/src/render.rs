use super::*;

pub struct Render {
    vec: Vec<Box<dyn Component>>,
}
impl Render {
    pub(crate) fn new() -> Render {
        Render { vec: Vec::new() }
    }
    pub fn add(mut self, component: impl Component + 'static) -> Self {
        component.add_to_render(&mut self);
        self
    }

    pub(crate) fn to_element(self) -> Vec<Element> {
        // self.vec
        //     .into_iter()
        //     .map(|component| component.to_element())
        //     .flatten()
        //     .collect()

        todo!()
    }

    pub(crate) fn add_component(&mut self, component: impl Component + 'static) {
        self.vec.push(Box::new(component));
    }

    pub(crate) fn into_children(self) -> impl Iterator<Item = Box<dyn Component>> {
        self.vec.into_iter()
    }
}
