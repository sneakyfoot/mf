use super::App;
use crate::k8s::{cancel_jobs, is_host_schedulable, set_host_schedulable};

use humantime::format_duration;
use k8s_openapi::chrono::{DateTime, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Cell, Paragraph, Row, Table},
};
use std::time::Duration;

impl App {
    /// Main table view
    pub fn draw_table(&mut self, frame: &mut Frame) {
        // Define Regions
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);
        // Main job table
        let rows = self.items.iter().map(|item| {
            let age = item
                .created_at
                .as_ref()
                .map(format_age)
                .unwrap_or_else(|| "n/a".into());
            let run_time = item
                .started_at
                .as_ref()
                .map(|s| format_run_time(&s, &item.finished_at.unwrap_or_else(Utc::now)))
                .unwrap_or_else(|| "n/a".into());
            let style = status_colors(&item.status);
            Row::new(vec![
                item.name.clone(),
                item.status.clone(),
                item.artist.clone(),
                item.node.clone(),
                run_time,
                age,
            ])
            .style(style)
        });
        let columns = [
            ("Name", Constraint::Percentage(50)),
            ("Status", Constraint::Percentage(10)),
            ("Artist", Constraint::Percentage(10)),
            ("Node", Constraint::Percentage(10)),
            ("Run Time", Constraint::Percentage(10)),
            ("Age", Constraint::Percentage(10)),
        ];
        let table = Table::new(rows, columns.iter().map(|(_, c)| *c))
            .header(Row::new(
                columns.iter().map(|(title, _)| Cell::from(*title)),
            ))
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("⇝")
            .block(Block::bordered());
        frame.render_stateful_widget(table, chunks[1], &mut self.state);
        let host_status = self
            .rt
            .block_on(is_host_schedulable(self.client.clone(), None));
        let host_status = match host_status {
            Ok(false) => "not on the farm. Press (p) to return it to the farm.".to_string(),
            Ok(true) => "on the farm. Press (o) to check out your node.".to_string(),
            Err(_) => "not part of the cluster.".to_string(),
        };
        let info =
            Paragraph::new("MF - (q) to quit, (Enter) to view logs. (Shift + D) to cancel a job.")
                .block(Block::bordered());
        let checkout_status =
            Paragraph::new(format!("Your node is {}", &host_status)).block(Block::bordered());
        frame.render_widget(info, chunks[0]);
        frame.render_widget(checkout_status, chunks[2]);
        self.show_confirmation(frame);
    }

    /// Spawns the confirmation for job deletion, to kill all jobs that share the same "controller"
    /// id
    pub fn delete_key(&mut self) {
        if let Some(controller) = self
            .state
            .selected()
            .and_then(|i| self.items.get(i))
            .and_then(|idx| idx.controller.as_ref())
        {
            self.pending_confirmation = Some(crate::app::ConfirmAction::CancelJob {
                controller: controller.clone(),
            });
            self.confirmation_popup = true;
        }
    }

    pub fn run_cancel_jobs(&mut self, controller: String) {
        let client = self.client.clone();
        self.rt.spawn(async move {
            if let Err(e) = cancel_jobs(client, &controller).await {
                eprintln!("Failed to cancel job {}", e);
            }
        });
    }

    pub fn checkout_key(&mut self, checkout: bool) {
        self.pending_confirmation = Some(crate::app::ConfirmAction::CheckoutNode {
            schedulable: (checkout),
        });
        self.confirmation_popup = true;
    }

    pub fn run_checkout(&mut self, checkout: bool) {
        if let Err(e) = self
            .rt
            .block_on(set_host_schedulable(self.client.clone(), None, checkout))
        {
            eprintln!("Failed to mark host schedulable: {}", e);
        }
    }

    /// Next line in table keymap
    pub fn next(&mut self) {
        if let Some(i) = self.state.selected() {
            if i + 1 < self.items.len() {
                self.state.select(Some(i + 1));
            }
        } else if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }

    /// Prev line in table keymap
    pub fn previous(&mut self) {
        if let Some(i) = self.state.selected() {
            if i > 0 {
                self.state.select(Some(i - 1));
            }
        } else if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }
}

/// Status to colors for table view
pub fn status_colors(status: &str) -> Style {
    match status {
        "Running" => Style::default().fg(ratatui::style::Color::Green),
        "Pending" => Style::default().fg(ratatui::style::Color::Blue),
        "Succeeded" => Style::default().fg(ratatui::style::Color::DarkGray),
        "Failed" | "CrashLoopBackoff" => Style::default().fg(ratatui::style::Color::Red),
        _ => Style::default(),
    }
}

/// Turn pod birth time into human readble age string
pub fn format_age(created: &DateTime<Utc>) -> String {
    let secs = Utc::now().signed_duration_since(*created).num_seconds();
    if secs < 0 {
        return "Unknown".to_string();
    }
    format_duration(Duration::from_secs(secs as u64)).to_string()
}

/// Turn pod run time into human readble duration string
pub fn format_run_time(started: &DateTime<Utc>, finished: &DateTime<Utc>) -> String {
    let secs = finished.signed_duration_since(*started).num_seconds();
    if secs < 0 {
        return "Unknown".to_string();
    }
    format_duration(Duration::from_secs(secs as u64)).to_string()
}
