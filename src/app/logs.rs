use super::App;
use super::Mode;
use crate::k8s::stream_logs;
use humantime::format_duration;
use k8s_openapi::chrono::{DateTime, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};
use std::time::Duration;
impl App {
    /// Log view
    pub fn draw_logs(&mut self, frame: &mut Frame, pod: &str, start: &DateTime<Utc>) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(area);
        let text = if self.logs.is_empty() {
            "(no data yet)".to_string()
        } else {
            format!("Start Logs for {}\n{}", pod, self.logs.join("\n"))
        };
        let block = Block::default()
            .title(format!("Logs for {}", pod))
            .borders(Borders::ALL);
        let para = Paragraph::new(text.clone())
            .block(block)
            .wrap(Wrap { trim: false });
        let total_lines = para.line_count(chunks[0].width) as u16;
        let mut scroll_y = total_lines.saturating_sub(chunks[0].height);
        self.max_log_lines = scroll_y;
        scroll_y = scroll_y.saturating_sub(self.scroll_offset);
        frame.render_widget(para.scroll((scroll_y, 0)), chunks[0]);
        if let Some(pct) = latest_alf_progress(&self.logs) {
            let elapsed = Utc::now().signed_duration_since(*start).num_seconds();
            let seconds_left = if pct >= 100 || pct < 1 {
                0.0
            } else {
                (elapsed as f64 / ((pct as f64) / 100.0)) - elapsed as f64
            };
            let eta = format_duration(Duration::from_secs_f64(seconds_left.round()));
            let gague = Gauge::default()
                .block(Block::default().title(format!("ETA: {}", eta)))
                .gauge_style(Style::new().blue().on_black())
                .percent(pct);
            frame.render_widget(gague, chunks[1]);
        }
    }
    /// Clamping scroll for logs
    pub fn scroll_logs(&mut self, down: bool) {
        if down {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
        } else {
            self.scroll_offset = self
                .scroll_offset
                .saturating_add(1)
                .clamp(0, self.max_log_lines);
        }
    }
    /// Spawn async log stream
    pub fn start_log_mode(&mut self) {
        if let Some(idx) = self.state.selected().and_then(|i| self.items.get(i)) {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let pod = idx.name.clone();
            self.log_rx = Some(rx);
            // let rt_handle = &self.rt;
            let client = self.client.clone();
            self.log_task = Some(self.rt.spawn(async move {
                match stream_logs(client, &pod).await {
                    Ok(reader) => {
                        use futures::AsyncBufReadExt;
                        use futures::StreamExt;
                        use futures::io::BufReader;
                        let mut lines = BufReader::new(reader).lines();
                        while let Some(line) = lines.next().await {
                            match line {
                                Ok(line) => {
                                    if tx.send(line).is_err() {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(format!("Log error: {e}"));
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!("Log error: {e}"));
                    }
                }
            }));
            self.logs.clear();
            self.mode = Mode::Logs {
                pod: idx.name.clone(),
                start: idx.started_at.clone().unwrap_or_else(Utc::now),
            };
        }
    }
    /// Get logs from async task
    pub fn drain_logs(&mut self) {
        if let Some(rx) = self.log_rx.as_mut() {
            while let Ok(line) = rx.try_recv() {
                self.logs.push(line);
            }
        }
    }
    pub fn exit_log_mode(&mut self) {
        self.logs.clear();
        self.scroll_offset = 0;
        self.log_rx = None;
        if let Some(handle) = self.log_task.take() {
            handle.abort();
        }
        self.mode = Mode::Table;
    }
}
/// Parse ALF_PROGRESS line from logs
fn parse_alf_progress(line: &str) -> Option<u16> {
    if !line.starts_with("ALF_PROGRESS") {
        return None;
    }
    let pct_token = line.split_whitespace().nth(1)?;
    let pct_str = pct_token.trim_end_matches('%');
    let pct = pct_str.parse::<u16>().ok()?;
    Some(pct.clamp(0, 100))
}
/// Get latest ALF_PROGRESS from log lines
pub fn latest_alf_progress(lines: &[String]) -> Option<u16> {
    lines.iter().rev().find_map(|line| parse_alf_progress(line))
}
