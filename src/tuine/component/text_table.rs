pub mod table_column;
mod table_scroll_state;

use std::{borrow::Cow, cmp::min};

use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Row, Table},
    Frame,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    constants::TABLE_GAP_HEIGHT_LIMIT,
    tuine::{Event, Status},
};

pub use self::table_column::{TextColumn, TextColumnConstraint};
use self::table_scroll_state::ScrollState as TextTableState;

use super::Component;

#[derive(Clone, Debug, Default)]
pub struct StyleSheet {
    text: Style,
    selected_text: Style,
    table_header: Style,
}

struct TextTableMsg {}

/// A sortable, scrollable table for text data.
pub struct TextTable<'a> {
    state: TextTableState,
    column_widths: Vec<u16>,
    columns: Vec<TextColumn>,
    show_gap: bool,
    show_selected_entry: bool,
    data: Vec<Row<'a>>,
    style_sheet: StyleSheet,
    sortable: bool,
    table_gap: u16,
}

impl<'a> TextTable<'a> {
    pub fn new<S: Into<Cow<'static, str>>>(columns: Vec<S>) -> Self {
        Self {
            state: TextTableState::default(),
            column_widths: vec![0; columns.len()],
            columns: columns
                .into_iter()
                .map(|name| TextColumn::new(name))
                .collect(),
            show_gap: true,
            show_selected_entry: true,
            data: Vec::default(),
            style_sheet: StyleSheet::default(),
            sortable: false,
            table_gap: 0,
            on_select: None,
            on_select_click: None,
        }
    }

    /// Whether to try to show a gap between the table headers and data.
    /// Note that if there isn't enough room, the gap will still be hidden.
    ///
    /// Defaults to `true`.
    pub fn show_gap(mut self, show_gap: bool) -> Self {
        self.show_gap = show_gap;
        self
    }

    /// Whether to highlight the selected entry.
    ///
    /// Defaults to `true`.
    pub fn show_selected_entry(mut self, show_selected_entry: bool) -> Self {
        self.show_selected_entry = show_selected_entry;
        self
    }

    /// Whether the table should display as sortable.
    ///
    /// Defaults to `false`.
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// What [`Message`] to send when a row is selected.
    ///
    /// Defaults to `None` (doing nothing).
    pub fn on_select(mut self, on_select: Option<Message>) -> Self {
        self.on_select = on_select;
        self
    }

    /// What [`Message`] to send if a selected row is clicked on.
    ///
    /// Defaults to `None` (doing nothing).
    pub fn on_select_click(mut self, on_select_click: Option<Message>) -> Self {
        self.on_select_click = on_select_click;
        self
    }

    fn update_column_widths(&mut self, bounds: Rect) {
        let total_width = bounds.width;
        let mut width_remaining = bounds.width;

        let mut column_widths: Vec<u16> = self
            .columns
            .iter()
            .map(|column| {
                let width = match column.width_constraint {
                    TextColumnConstraint::Fill => {
                        let desired = column.name.graphemes(true).count().saturating_add(1) as u16;
                        min(desired, width_remaining)
                    }
                    TextColumnConstraint::Length(length) => min(length, width_remaining),
                    TextColumnConstraint::Percentage(percentage) => {
                        let length = total_width * percentage / 100;
                        min(length, width_remaining)
                    }
                    TextColumnConstraint::MaxLength(length) => {
                        let desired = column.name.graphemes(true).count().saturating_add(1) as u16;
                        min(min(length, desired), width_remaining)
                    }
                    TextColumnConstraint::MaxPercentage(percentage) => {
                        let desired = column.name.graphemes(true).count().saturating_add(1) as u16;
                        let length = total_width * percentage / 100;
                        min(min(desired, length), width_remaining)
                    }
                };
                width_remaining -= width;
                width
            })
            .collect();

        if !column_widths.is_empty() {
            let amount_per_slot = width_remaining / column_widths.len() as u16;
            width_remaining %= column_widths.len() as u16;
            for (index, width) in column_widths.iter_mut().enumerate() {
                if (index as u16) < width_remaining {
                    *width += amount_per_slot + 1;
                } else {
                    *width += amount_per_slot;
                }
            }
        }

        self.column_widths = column_widths;
    }
}

impl<'a> Component for TextTable<'a> {
    type Message = TextTableMsg;

    fn on_event(&mut self, bounds: Rect, event: Event, messages: &mut Vec<Message>) -> Status {
        use crate::tuine::MouseBoundIntersect;
        use crossterm::event::{MouseButton, MouseEventKind};

        match event {
            Event::Keyboard(_) => Status::Ignored,
            Event::Mouse(mouse_event) => {
                if mouse_event.does_mouse_intersect_bounds(bounds) {
                    match mouse_event.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            let y = mouse_event.row - bounds.top();

                            if self.sortable && y == 0 {
                                // TODO: Do this
                                Status::Captured
                            } else if y > self.table_gap {
                                let visual_index = usize::from(y - self.table_gap);
                                self.state.set_visual_index(visual_index)
                            } else {
                                Status::Ignored
                            }
                        }
                        MouseEventKind::ScrollDown => self.state.move_down(1),
                        MouseEventKind::ScrollUp => self.state.move_up(1),
                        _ => Status::Ignored,
                    }
                } else {
                    Status::Ignored
                }
            }
        }
    }

    fn draw<B: Backend>(&mut self, bounds: Rect, frame: &mut Frame<'_, B>) {
        self.table_gap = if !self.show_gap
            || (self.data.len() + 2 > bounds.height.into()
                && bounds.height < TABLE_GAP_HEIGHT_LIMIT)
        {
            0
        } else {
            1
        };

        let table_extras = 1 + self.table_gap;
        let scrollable_height = bounds.height.saturating_sub(table_extras);
        self.update_column_widths(bounds);

        // Calculate widths first, since we need them later.
        let widths = self
            .column_widths
            .iter()
            .map(|column| Constraint::Length(*column))
            .collect::<Vec<_>>();

        // Then calculate rows. We truncate the amount of data read based on height,
        // as well as truncating some entries based on available width.
        let data_slice = {
            // Note: `get_list_start` already ensures `start` is within the bounds of the number of items, so no need to check!
            let start = self
                .state
                .display_start_index(bounds, scrollable_height as usize);
            let end = min(self.state.num_items(), start + scrollable_height as usize);

            self.data[start..end].to_vec()
        };

        // Now build up our headers...
        let header = Row::new(self.columns.iter().map(|column| column.name.clone()))
            .style(self.style_sheet.table_header)
            .bottom_margin(self.table_gap);

        let mut table = Table::new(data_slice)
            .header(header)
            .style(self.style_sheet.text);

        if self.show_selected_entry {
            table = table.highlight_style(self.style_sheet.selected_text);
        }

        frame.render_stateful_widget(table.widths(&widths), bounds, self.state.tui_state());
    }
}

#[cfg(test)]
mod tests {}
