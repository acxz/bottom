use crate::tuine::{Shortcut, TextTable, TmpComponent, ViewContext};

/// A [`TempTable`] is a text table that is meant to display temperature data.
pub struct TempTable<'a, Message> {
    inner: Shortcut<Message, TextTable<'a, Message>>,
}

impl<'a, Message> TempTable<'a, Message> {
    #[track_caller]
    pub fn new(ctx: &mut ViewContext<'_>) -> Self {
        Self {
            inner: Shortcut::with_child(TextTable::new(ctx, vec!["Sensor", "Temp"])),
        }
    }
}

impl<'a, Message> TmpComponent<Message> for TempTable<'a, Message> {
    fn draw<Backend>(
        &mut self, state_ctx: &mut crate::tuine::StateContext<'_>,
        draw_ctx: &crate::tuine::DrawContext<'_>, frame: &mut tui::Frame<'_, Backend>,
    ) where
        Backend: tui::backend::Backend,
    {
        self.inner.draw(state_ctx, draw_ctx, frame);
    }

    fn on_event(
        &mut self, state_ctx: &mut crate::tuine::StateContext<'_>,
        draw_ctx: &crate::tuine::DrawContext<'_>, event: crate::tuine::Event,
        messages: &mut Vec<Message>,
    ) -> crate::tuine::Status {
        self.inner.on_event(state_ctx, draw_ctx, event, messages)
    }

    fn layout(
        &self, bounds: crate::tuine::Bounds, node: &mut crate::tuine::LayoutNode,
    ) -> crate::tuine::Size {
        self.inner.layout(bounds, node)
    }
}