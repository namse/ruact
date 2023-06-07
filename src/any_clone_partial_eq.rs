use std::fmt::Debug;

pub(crate) trait AnyClonePartialEq {
    fn boxing(self) -> AnyClonePartialEqBox
    where
        Self: Sized + 'static,
    {
        AnyClonePartialEqBox {
            inner: Box::new(self),
        }
    }
    fn dyning<'a>(&'a self) -> DynAnyClonePartialEq<'a>
    where
        Self: Sized + 'static,
    {
        DynAnyClonePartialEq { inner: self }
    }
    fn clone_box(&self) -> Box<dyn AnyClonePartialEq>;
    fn equals(&self, other: &dyn AnyClonePartialEq) -> bool;
    fn as_any(&self) -> &dyn std::any::Any;
    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl<T: 'static + std::any::Any + Clone + PartialEq + Debug> AnyClonePartialEq for T {
    fn clone_box(&self) -> Box<dyn AnyClonePartialEq> {
        Box::new(Clone::clone(self))
    }
    fn equals(&self, other: &dyn AnyClonePartialEq) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .map_or(false, |a| self == a)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

pub(crate) struct DynAnyClonePartialEq<'a> {
    inner: &'a dyn AnyClonePartialEq,
}
impl<'a> PartialEq for DynAnyClonePartialEq<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.equals(other.inner)
    }
}
impl<'a> Debug for DynAnyClonePartialEq<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.debug(f)
    }
}

pub(crate) struct AnyClonePartialEqBox {
    inner: Box<dyn AnyClonePartialEq>,
}

impl Clone for AnyClonePartialEqBox {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}
impl PartialEq for AnyClonePartialEqBox {
    fn eq(&self, other: &Self) -> bool {
        self.inner.equals(other.inner.as_ref())
    }
}
impl Debug for AnyClonePartialEqBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.debug(f)
    }
}

impl AnyClonePartialEqBox {
    pub fn new(event: impl std::any::Any + Clone + PartialEq + Debug) -> Self {
        Self {
            inner: Box::new(event),
        }
    }

    pub fn downcast_ref<T: 'static + std::any::Any + Clone + PartialEq + Debug>(
        &self,
    ) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }

    pub fn as_ref(&self) -> &dyn AnyClonePartialEq {
        self.inner.as_ref()
    }
}
