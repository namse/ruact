use crate::AnyClonePartialEqBox;
use std::sync::Arc;

pub fn closure<Param: 'static, Capture>(
    capture: Capture,
    func: fn(Param, &Capture),
) -> Closure<Param>
where
    Capture: std::any::Any + Clone + PartialEq + std::fmt::Debug,
{
    Closure {
        func_ptr: func as *const (),
        func: Arc::new(move |param, capture| {
            let capture: &Capture = capture.downcast_ref().unwrap();
            (func)(param, capture);
        }),
        capture: AnyClonePartialEqBox::new(capture),
    }
}

// PartialEq, Debug
#[derive(Clone)]
pub struct Closure<Param> {
    func_ptr: *const (),
    func: Arc<dyn Fn(Param, &AnyClonePartialEqBox)>,
    capture: AnyClonePartialEqBox,
}

impl<Param> PartialEq for Closure<Param> {
    fn eq(&self, other: &Self) -> bool {
        self.func_ptr == other.func_ptr && self.capture == other.capture
    }
}

impl<Param> std::fmt::Debug for Closure<Param> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Closure")
            .field("func_ptr", &self.func_ptr)
            .field("capture", &self.capture)
            .finish()
    }
}

impl<Param> Closure<Param> {
    pub fn invoke(&self, param: Param) {
        (self.func)(param, &self.capture);
    }
}
