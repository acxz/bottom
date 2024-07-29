use std::{borrow::Cow, num::NonZeroU16, time::Instant};

use concat_string::concat_string;
use tui::{style::Style, widgets::Row};

use crate::{
    app::AppConfigFields,
    canvas::{
        components::data_table::{
            ColumnHeader, DataTableColumn, DataTableProps, DataTableStyling, DataToCell,
            SortColumn, SortDataTable, SortDataTableProps, SortOrder, SortsRow
        },
        styling::CanvasStyling,
        Painter,
    },
    data_collection::cpu::CpuDataType,
    data_conversion::CpuWidgetData,
    options::config::cpu::CpuDefault,
};

#[derive(Default)]
pub struct CpuWidgetStyling {
    pub all: Style,
    pub avg: Style,
    pub entries: Vec<Style>,
}

impl CpuWidgetStyling {
    fn from_colours(colours: &CanvasStyling) -> Self {
        let entries = if colours.cpu_colour_styles.is_empty() {
            vec![Style::default()]
        } else {
            colours.cpu_colour_styles.clone()
        };

        Self {
            all: colours.all_colour_style,
            avg: colours.avg_colour_style,
            entries,
        }
    }
}

pub enum CpuWidgetColumn {
    CPU,
    Use,
}

impl ColumnHeader for CpuWidgetColumn {
    fn text(&self) -> Cow<'static, str> {
        match self {
            CpuWidgetColumn::CPU => "CPU".into(),
            CpuWidgetColumn::Use => "Use".into(),
        }
    }
}

// TODO: Do I need this?
// #[derive(Clone)]
pub enum CpuWidgetTableData {
    All,
    Entry {
        data_type: CpuDataType,
        last_entry: f64,
    },
}

impl CpuWidgetTableData {
    pub fn from_cpu_widget_data(data: &CpuWidgetData) -> CpuWidgetTableData {
        match data {
            CpuWidgetData::All => CpuWidgetTableData::All,
            CpuWidgetData::Entry {
                data_type,
                data: _,
                last_entry,
            } => CpuWidgetTableData::Entry {
                data_type: *data_type,
                last_entry: *last_entry,
            },
        }
    }
}

impl DataToCell<CpuWidgetColumn> for CpuWidgetTableData {
    fn to_cell(
        &self, column: &CpuWidgetColumn, calculated_width: NonZeroU16,
    ) -> Option<Cow<'static, str>> {
        const CPU_TRUNCATE_BREAKPOINT: u16 = 5;

        let calculated_width = calculated_width.get();

        // This is a bit of a hack, but apparently we can avoid having to do any fancy
        // checks of showing the "All" on a specific column if the other is
        // hidden by just always showing it on the CPU (first) column - if there
        // isn't room for it, it will just collapse down.
        //
        // This is the same for the use percentages - we just *always* show them, and
        // *always* hide the CPU column if it is too small.
        match &self {
            CpuWidgetTableData::All => match column {
                CpuWidgetColumn::CPU => Some("All".into()),
                CpuWidgetColumn::Use => None,
            },
            CpuWidgetTableData::Entry {
                data_type,
                last_entry,
            } => {
                if calculated_width == 0 {
                    None
                } else {
                    match column {
                        CpuWidgetColumn::CPU => match data_type {
                            CpuDataType::Avg => Some("AVG".into()),
                            CpuDataType::Cpu(index) => {
                                let index_str = index.to_string();
                                let text = if calculated_width < CPU_TRUNCATE_BREAKPOINT {
                                    index_str.into()
                                } else {
                                    concat_string!("CPU", index_str).into()
                                };

                                Some(text)
                            }
                        },
                        CpuWidgetColumn::Use => Some(format!("{:.0}%", last_entry.round()).into()),
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn style_row<'a>(&self, row: Row<'a>, painter: &Painter) -> Row<'a> {
        let style = match self {
            CpuWidgetTableData::All => painter.colours.all_colour_style,
            CpuWidgetTableData::Entry {
                data_type,
                last_entry: _,
            } => match data_type {
                CpuDataType::Avg => painter.colours.avg_colour_style,
                CpuDataType::Cpu(index) => {
                    painter.colours.cpu_colour_styles
                        [index % painter.colours.cpu_colour_styles.len()]
                }
            },
        };

        row.style(style)
    }

    fn column_widths<C: DataTableColumn<CpuWidgetColumn>>(
        _data: &[Self], _columns: &[C],
    ) -> Vec<u16>
    where
        Self: Sized,
    {
        vec![1, 3]
    }
}

impl SortsRow for CpuWidgetColumn {
    type DataType = CpuWidgetTableData;

    fn sort_data(&self, data: &mut [Self::DataType], descending: bool) {
        // let mut table_data = data.iter()
        //                          .map(CpuWidgetTableData::from_cpu_widget_data)
        //                          .collect()
        match self {
            // TODO: don't sort ALL and AVG
            CpuWidgetColumn::CPU => {
                data.sort_by(|a, b| {
                    let mut order = match (a, b) {
                        (CpuWidgetTableData::All, _) => std::cmp::Ordering::Greater,
                        (_, CpuWidgetTableData::All) => std::cmp::Ordering::Less,
                        (CpuWidgetTableData::Entry {
                            data_type: a_data_type, ..},
                        CpuWidgetTableData::Entry {
                            data_type: b_data_type, ..}
                        ) => {
                            match (a_data_type, b_data_type) {
                                (CpuDataType::Avg, _) => std::cmp::Ordering::Greater,
                                (_, CpuDataType::Avg) => std::cmp::Ordering::Less,
                                // TODO: does this get the name field?
                                (CpuDataType::Cpu(a_cpu), CpuDataType::Cpu(b_cpu)) => a_cpu.cmp(b_cpu),
                            }
                        },
                    };
                    // TODO: factor in descending bool
                    if !descending {
                        // Flip order
                        order = match order {
                            std::cmp::Ordering::Less => std::cmp::Ordering::Greater,
                            std::cmp::Ordering::Greater => std::cmp::Ordering::Less,
                            _ => order,
                        }
                    }
                    return order;
                });
            }
            CpuWidgetColumn::Use => {
                data.sort_by(|a, b| {
                    let mut order = match (a, b) {
                        (CpuWidgetTableData::All, _) => std::cmp::Ordering::Greater,
                        (_, CpuWidgetTableData::All) => std::cmp::Ordering::Less,
                        (CpuWidgetTableData::Entry {
                            data_type: a_data_type,
                            last_entry: a_last_entry
                        },
                        CpuWidgetTableData::Entry {
                            data_type: b_data_type,
                            last_entry: b_last_entry
                        }) => {
                            match (a_data_type, b_data_type) {
                                (CpuDataType::Avg, _) => std::cmp::Ordering::Greater,
                                (_, CpuDataType::Avg) => std::cmp::Ordering::Less,
                                // TODO: does this get the usage field?
                                (CpuDataType::Cpu(_), CpuDataType::Cpu(_)) => a_last_entry.partial_cmp(b_last_entry).unwrap_or(std::cmp::Ordering::Equal),
                            }
                        },
                    };
                    // TODO: factor in descending bool
                    if !descending {
                        // Flip order
                        order = match order {
                            std::cmp::Ordering::Less => std::cmp::Ordering::Greater,
                            std::cmp::Ordering::Greater => std::cmp::Ordering::Less,
                            _ => order,
                        }
                    }
                    return order;

                    // let order = std::cmp::Ordering::Equal;
                    // if a == CpuWidgetData::All {
                    //     std::cmp::Ordering::Greater;
                    // } else if a == CpuWidgetData::Entry { // a == Avg
                    //     if b == CpuWidgetData::All {
                    //         std::cmp::Ordering::Less;
                    //     } else {
                    //         std::cmp::Ordering::Greater;
                    //     }
                    // } else {
                    //     if b == CpuWidgetData::All || b == CpuWidgetData::Entry { // b == Avg
                    //         std::cmp::Ordering::Less;
                    //     }
                    //     else {
                    //         // TODO: does this get the use field?
                    //         // TODO: run num compare on a and b cpuusage
                    //     }
                    // }
                });
            }
        }
    }
}

// TODO: required for sorting cpuwidgetdatas in the SortsRow impl
// impl PartialOrd for CpuWidgetData {
// 
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         match (self, other) {
//             (CpuWidgetData::All, CpuWidget::Entry(_, _, _)) => Some(Greater),
//             (CpuWidgetData::Entry(_, _, _), CpuWidgetData::All) => Some(Less),
//             (CpuWidgetData::Entry(data_type_a, data_a, last_entry_a), CpuWidgetData::Entry(data_type_b, data_b, last_entry_b)) => //TODO: Match this on cpu or use,
//             _ => unreachable!(),
//         }
//     }
// }

pub struct CpuWidgetState {
    pub current_display_time: u64,
    pub is_legend_hidden: bool,
    pub show_avg: bool,
    pub autohide_timer: Option<Instant>,
    pub table: SortDataTable<CpuWidgetTableData, CpuWidgetColumn>,
    pub styling: CpuWidgetStyling,
}

impl CpuWidgetState {
    pub fn new(
        config: &AppConfigFields, default_selection: CpuDefault, current_display_time: u64,
        autohide_timer: Option<Instant>, colours: &CanvasStyling,
    ) -> Self {
        let columns: [SortColumn<CpuWidgetColumn>; 2] = [
            SortColumn::soft(CpuWidgetColumn::CPU, None).default_descending(),
            SortColumn::soft(CpuWidgetColumn::Use, None),
        ];

        let props = SortDataTableProps {
            inner: DataTableProps {
                title: None,
                table_gap: config.table_gap,
                left_to_right: false,
                is_basic: false,
                show_table_scroll_position: config.show_table_scroll_position,
                show_current_entry_when_unfocused: true,
            },
            sort_index: 0,
            order: SortOrder::Ascending
        };

        let styling = DataTableStyling::from_colours(colours);

        let mut table = SortDataTable::new_sortable(columns, props, styling);
        match default_selection {
            CpuDefault::All => {}
            CpuDefault::Average if !config.show_average_cpu => {}
            CpuDefault::Average => {
                table = table.first_draw_index(1);
            }
        }

        CpuWidgetState {
            current_display_time,
            is_legend_hidden: false,
            show_avg: config.show_average_cpu,
            autohide_timer,
            table,
            styling: CpuWidgetStyling::from_colours(colours),
        }
    }

    pub fn update_table(&mut self, data: const &[CpuWidgetData]) {
        // TODO: This is the funky stuff
        let mut table_data = [CpuWidgetTableData::All; data.len()];
        for (idx, cpu_widget_data) in data.iter().enumerate() {
            table_data[idx] = CpuWidgetTableData::from_cpu_widget_data(cpu_widget_data);
        }

        // let mut data = data.to_vec();
        if let Some(column) = self.table.columns.get(self.table.sort_index()) {
            column.sort_by(&mut table_data, self.table.order());
        }
        self.table.set_data(table_data);
        // self.table.set_data(
        //     data.iter()
        //         .map(CpuWidgetTableData::from_cpu_widget_data)
        //         .collect(),
        // );
    }
}
