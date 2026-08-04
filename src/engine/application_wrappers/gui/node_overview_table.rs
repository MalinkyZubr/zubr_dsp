use crate::engine::control_plane::pipeline_analytics::NodeAnalytics;
use crate::engine::application_wrappers::gui::node_overview_table::table_advanced::Column;
use iced::widget::{container, responsive, row as wrow};
use iced::widget::{scrollable, text};
use iced::{Fill, Shrink, Size, Task, Length};
use iced::widget;
use iced::{Element, Theme};
use iced_table2::table as table_advanced;
use itertools::Itertools;
use std::time;
use std::time::UNIX_EPOCH;
use iced::widget::text::Wrapping;
use crate::engine::control_plane::pipeline_hl::Pipeline;
use crate::engine::application_wrappers::gui::log_monitor::LogMonitor;
use crate::engine::application_wrappers::gui::style::container_style;

#[derive(Clone)]
#[derive(Debug)]
pub enum PipelineTableMessage {
    SyncHeader(scrollable::AbsoluteOffset),
    HeaderPressed(usize),
    RowPressed(usize),
    ColumnDragged(usize, f32),
    ColumnReleased,
    StatusUpdate(Vec<NodeAnalytics>)
}
pub enum StopRequestEnum {
    Requested,
    NotRequested,
    InvalidNode
}


// pub enum SortBy {
//     ID,
//     Name,
//     RunModel,
//     NumExecutions,
//     ExecTime,
//     TimeSinceExec,
//     CurrentState,
// }
pub struct PipelineTableColumn {
    title: &'static str,
    width: f32,
    sort: Option<bool>,
    resize_offset: Option<f32>,
}
impl<'a> table_advanced::Column<'a, PipelineTableMessage, Theme, iced::Renderer> for PipelineTableColumn {
    type Row = NodeAnalytics;
    
    fn header(&'a self, _col_index: usize) -> Element<'a, PipelineTableMessage> {
        let label = text(self.title);
        match self.sort {
            Some(true) => wrow![label, text(" ▲")].into(),
            Some(false) => wrow![label, text(" ▼")].into(),
            None => label.into(),
        }
    }
    
    fn footer(&'a self, col_index: usize, rows: &'a [NodeAnalytics]) -> Option<Element<'a, PipelineTableMessage>> {
        let content = match col_index {
            0 => format!("Num Nodes: {}", rows.len().to_string()),
            _ => "".to_string(),
        };
        Some(text(content).into())
    }

    fn cell(
        &'a self,
        col_index: usize,
        _row_index: usize,
        row: &'a NodeAnalytics,
    ) -> Element<'a, PipelineTableMessage> {
        let content: String = match col_index {
            0 => row.name.clone(),
            1 => row.id.to_string(),
            2 => row.run_model.to_string(),
            3 => row.current_state.to_string(),
            4 => row.standard_deviation_execution_time.to_string(),
            5 => row.average_execution_time.to_string(),
            _ => "".to_string(),
        };
        text(content).into()
    }
    
    fn width(&self) -> f32 {
        self.width
    }

    fn resize_offset(&self) -> Option<f32> {
        None
    }
}


pub struct PipelineTable {
    columns: Vec<PipelineTableColumn>,
    rows: Vec<NodeAnalytics>,
    selected_row: Option<usize>,
    sort_column: Option<usize>,
    sort_ascending: bool,
    header_id: widget::Id,
    footer_id: widget::Id,
    body_id: widget::Id,
    
}
impl PipelineTable {
    pub fn new(rows: Vec<NodeAnalytics>) -> Self {
        Self {
            columns: vec![
                PipelineTableColumn { title: "Name", width: 100.0, sort: Some(true), resize_offset: None },
                PipelineTableColumn { title: "ID", width: 100.0, sort: Some(true), resize_offset: None },
                PipelineTableColumn { title: "Run Model", width: 100.0, sort: Some(true), resize_offset: None },
                PipelineTableColumn { title: "Current State", width: 100.0, sort: Some(true), resize_offset: None },
                PipelineTableColumn { title: "Exec Stdev Time (ns)", width: 100.0, sort: Some(true), resize_offset: None },
                PipelineTableColumn { title: "Exec Average Time (ns)", width: 100.0, sort: Some(true), resize_offset: None },
            ],
            rows,
            header_id: widget::Id::unique(),
            body_id: widget::Id::unique(),
            footer_id: widget::Id::unique(),
            selected_row: None,
            sort_column: None,
            sort_ascending: true,
        }
    }
    pub fn view(&self) -> Element<'_, PipelineTableMessage> {
        container(responsive(|size: Size| {
            let mut tbl = table_advanced(
                self.header_id.clone(),
                self.body_id.clone(),
                &self.columns,
                &self.rows,
                PipelineTableMessage::SyncHeader,
            )
                .footer(self.footer_id.clone())
                .on_column_resize(PipelineTableMessage::ColumnDragged, PipelineTableMessage::ColumnReleased)
                .on_row_press(PipelineTableMessage::RowPressed)
                .on_header_press(PipelineTableMessage::HeaderPressed)
                .cell_padding(16)
                .min_column_width(size.width / self.columns.len() as f32)
                .min_width(size.width)
                .divider_width(5.0);
            if let Some(index) = self.selected_row {
                tbl = tbl.selected_row(index);
            }
            tbl.into()
        }))
            .padding(20)
            .into()
    }

    pub fn update(&mut self, message: PipelineTableMessage) -> Task<PipelineTableMessage> {
        match message {
            PipelineTableMessage::SyncHeader(offset) => {
                return Task::batch([
                    widget::operation::scroll_to(self.header_id.clone(), offset),
                    widget::operation::scroll_to(self.footer_id.clone(), offset),
                ]);
            }
            PipelineTableMessage::ColumnDragged(index, offset) => {
                if let Some(col) = self.columns.get_mut(index) {
                    col.resize_offset = Some(offset);
                }
            }
            PipelineTableMessage::ColumnReleased => {
                for col in &mut self.columns {
                    if let Some(offset) = col.resize_offset.take() {
                        col.width = (col.width + offset).max(4.0);
                    }
                }
            }
            PipelineTableMessage::RowPressed(index) => {
                self.selected_row = if self.selected_row == Some(index) {
                    None
                } else {
                    Some(index)
                };
            }
            PipelineTableMessage::HeaderPressed(index) => {
                if self.sort_column == Some(index) {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(index);
                    self.sort_ascending = true;
                }
                for (i, col) in self.columns.iter_mut().enumerate() {
                    col.sort = if i == index { Some(self.sort_ascending) } else { None };
                }
                let ascending = self.sort_ascending;
                self.rows.sort_by(|a, b| {
                    let ord = match index {
                        0 => a.name.cmp(&b.name),
                        1 => a.id.cmp(&b.id),
                        2 => a.run_model.cmp(&b.run_model),
                        3 => a.current_state.cmp(&b.current_state),
                        4 => a.standard_deviation_execution_time.cmp(&b.standard_deviation_execution_time),
                        5 => a.average_execution_time.cmp(&b.average_execution_time),
                        _ => std::cmp::Ordering::Equal,
                    };
                    if ascending { ord } else { ord.reverse() }
                });
                self.selected_row = None;
            }
            PipelineTableMessage::StatusUpdate(update) => {
                for new_status in update {
                    let new_id = &new_status.id;
                    let mut idx = 0;
                    let mut completed = false;

                    while idx < self.rows.len() && !completed {
                        if self.rows[idx].id == *new_id {
                            completed = true;
                        } else {
                            idx += 1;
                        }
                    }
                    if !completed {
                        self.rows.push(new_status);
                    } else {
                        self.rows[idx] = new_status;
                    }
                }
            }
        }
        Task::none()
    }
}