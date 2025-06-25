use std::sync::OnceLock;

use impl_helper::ImplHelper;
use wrap_context::arg_context;

use crate::render::Render;

pub static DEBUG_PRINT_LIMIT: OnceLock<usize> = OnceLock::new();

pub type Step = usize; // TODO maybe remove
pub type Path = Vec<Step>;

#[derive(Debug, PartialEq, Eq)]
pub enum CursorDirection {
    Up,
    Down,
    Right,
    Left,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, ImplHelper)]
pub struct Entry {
    #[helper(all)]
    name: String,

    #[helper(all)]
    path: Path,
}

impl Entry {
    pub fn new(name: String, path: Path) -> Self {
        Self { name, path }
    }
}

#[derive(Debug, Default, ImplHelper)]
pub struct ViewModel<V, M> {
    view: V,

    #[helper(get)]
    model: M,
}

impl<V, M> ViewModel<V, M>
where
    M: std::fmt::Debug,
    V: for<'a> TryFrom<&'a M, Error = anyhow::Error>,
{
    pub fn try_with_model_mut<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut M) -> anyhow::Result<()>,
    {
        arg_context!(f(&mut self.model))?;
        self.view = arg_context!(V::try_from(&self.model))?;
        anyhow::Ok(())
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn try_model_set(mut self, model: M) -> anyhow::Result<Self> {
        self.model = model;
        self.view = arg_context!(V::try_from(&self.model))?;
        anyhow::Ok(self)
    }
}

impl<V, M> ViewModel<V, M>
where
    M: std::fmt::Debug,
    V: for<'a> From<&'a M>,
{
    pub fn with_model_mut<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut M) -> anyhow::Result<()>,
    {
        arg_context!(f(&mut self.model))?;
        self.view = V::from(&self.model);
        anyhow::Ok(())
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn model_set(mut self, model: M) -> Self {
        self.model = model;
        self.view = V::from(&self.model);
        self
    }
}

impl<V, M> Render for ViewModel<V, M>
where
    V: Render,
{
    fn render(&mut self, frame: &mut ratatui::Frame) {
        self.view.render(frame);
    }
}
