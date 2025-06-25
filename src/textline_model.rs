use impl_helper::ImplHelper;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
};

use crate::{directional_constraint::DirectionalConstraint, utils::clip_string};

type Span = (String, Style);

#[derive(Default, Debug, ImplHelper)]
pub struct TextLineModel {
    #[helper(get, get_mut /* set and upd are custom */)]
    area: Rect,

    #[helper(/* set and upd are custom */)]
    hide_spans: Vec<Span>,

    #[helper(get)]
    show_spans: Vec<Span>,

    #[helper(all)]
    style: Style,

    #[helper(all)]
    alignment: Alignment,

    #[helper(all)]
    transparent: bool,

    #[helper(all)]
    constraint: DirectionalConstraint,
}

impl TextLineModel {
    fn make_show_spans(&mut self) {
        self.show_spans = self.hide_spans.clone();
        for (content, _) in self.show_spans.iter_mut() {
            *content = clip_string(content.clone(), "…", self.area.width.into());
        }
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn area_set(mut self, value: Rect) -> Self {
        self.area = value;
        self.make_show_spans();
        self
    }

    pub fn area_update(&mut self, value: Rect) -> &mut Self {
        self.area = value;
        self.make_show_spans();
        self
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn hide_spans_set(mut self, value: Vec<Span>) -> Self {
        self.hide_spans = value;
        self.make_show_spans();
        self
    }

    pub fn hide_spans_update(&mut self, value: Vec<Span>) -> &mut Self {
        self.hide_spans = value;
        self.make_show_spans();
        self
    }
}
