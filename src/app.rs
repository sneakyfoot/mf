use crate::data::{Data, fetch_data};
use crate::k8s::stream_logs;
use humantime::format_duration;
use k8s_openapi::chrono::{DateTime, Utc};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table, TableState, Wrap},
};
use std::error::Error;
use std::time::Duration;
use tokio::runtime::Runtime;

////////////////////////
/* Main App Interface */
////////////////////////

pub struct App {
    state: TableState,
    items: Vec<Data>,
    rt: Runtime,
    mode: Mode,
    scroll_offset: u16,
    max_log_lines: u16,
    logs: Vec<String>,
    log_rx: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
    log_task: Option<tokio::task::JoinHandle<()>>,
}

enum Mode {
    Table,
    Logs { pod: String, start: DateTime<Utc> },
}

impl App {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let rt = Runtime::new()?;
        let items = rt.block_on(fetch_data())?;
        Ok(Self {
            state: TableState::default().with_selected(0),
            items,
            rt,
            mode: Mode::Table,
            scroll_offset: 0,
            max_log_lines: 0,
            logs: Vec::new(),
            log_rx: None,
            log_task: None,
        })
    }

    // Main loop
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<(), Box<dyn Error>> {
        let tick = Duration::from_millis(100);
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(tick)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if self.handle_key(key)? {
                            return Ok(());
                        }
                        if matches!(self.mode, Mode::Logs { .. }) {
                            self.drain_logs();
                        }
                    }
                }
            } else {
                if matches!(self.mode, Mode::Logs { .. }) {
                    self.drain_logs();
                } else {
                    if let Ok(items) = self.rt.block_on(fetch_data()) {
                        self.items = items;
                    }
                }
            }
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        match &self.mode {
            Mode::Table => self.draw_table(frame),
            Mode::Logs { pod, start } => {
                let pod = pod.clone();
                self.draw_logs(frame, &pod, &start.clone());
            }
        }
    }

    // Main table view
    fn draw_table(&mut self, frame: &mut Frame) {
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
                item.node.clone(),
                run_time,
                age,
            ])
            .style(style)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(50),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
            ],
        )
        .header(Row::new(vec!["Name", "Status", "Node", "Run Time", "Age"]))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("⇝");

        frame.render_stateful_widget(table, frame.area(), &mut self.state);
    }

    // Log view
    fn draw_logs(&mut self, frame: &mut Frame, pod: &str, start: &DateTime<Utc>) {
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

    // Keybinds
    fn handle_key(&mut self, key: event::KeyEvent) -> Result<bool, Box<dyn Error>> {
        match &self.mode {
            // Keybinds while in default pod table
            Mode::Table => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
                KeyCode::Char('j') | KeyCode::Down => self.next(),
                KeyCode::Char('k') | KeyCode::Up => self.previous(),
                KeyCode::Enter => self.start_log_mode(),
                _ => {}
            },
            // Keybinds while in log mode
            Mode::Logs { pod, start } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.mode = Mode::Table;
                        self.scroll_offset = 0;
                        self.logs.clear();
                    }
                    KeyCode::Char('k') | KeyCode::Up => self.scroll_logs(false),
                    KeyCode::Char('j') | KeyCode::Down => self.scroll_logs(true),
                    _ => {}
                }
                return Ok(false);
            }
        }
        Ok(false)
    }

    // Spawn async log stream
    fn start_log_mode(&mut self) {
        if let Some(idx) = self.state.selected().and_then(|i| self.items.get(i)) {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let pod = idx.name.clone();
            self.log_rx = Some(rx);
            let rt_handle = &self.rt;
            self.log_task = Some(self.rt.spawn(async move {
                match stream_logs(&pod).await {
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
    fn drain_logs(&mut self) {
        if let Some(rx) = self.log_rx.as_mut() {
            while let Ok(line) = rx.try_recv() {
                self.logs.push(line);
            }
        }
    }

    fn scroll_logs(&mut self, down: bool) {
        if down {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
        } else {
            self.scroll_offset = self
                .scroll_offset
                .saturating_add(1)
                .clamp(0, self.max_log_lines);
        }
    }

    // Next line in table keymap
    fn next(&mut self) {
        if let Some(i) = self.state.selected() {
            if i + 1 < self.items.len() {
                self.state.select(Some(i + 1));
            }
        } else if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }

    // Prev line in table keymap
    fn previous(&mut self) {
        if let Some(i) = self.state.selected() {
            if i > 0 {
                self.state.select(Some(i - 1));
            }
        } else if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }
}

// Status to colors for table view
fn status_colors(status: &str) -> Style {
    match status {
        "Running" => Style::default().fg(ratatui::style::Color::Green),
        "Pending" => Style::default().fg(ratatui::style::Color::Blue),
        "Succeeded" => Style::default().fg(ratatui::style::Color::DarkGray),
        "Failed" | "CrashLoopBackoff" => Style::default().fg(ratatui::style::Color::Red),
        _ => Style::default(),
    }
}

// Alf progress helpers
fn parse_alf_progress(line: &str) -> Option<u16> {
    if !line.starts_with("ALF_PROGRESS") {
        return None;
    }
    let pct_token = line.split_whitespace().nth(1)?;
    let pct_str = pct_token.trim_end_matches('%');
    let pct = pct_str.parse::<u16>().ok()?;
    Some(pct.clamp(0, 100))
}
fn latest_alf_progress(lines: &[String]) -> Option<u16> {
    lines.iter().rev().find_map(|line| parse_alf_progress(line))
}

// Turn pod birth time into human readble age string
fn format_age(created: &DateTime<Utc>) -> String {
    let secs = Utc::now().signed_duration_since(*created).num_seconds();
    if secs < 0 {
        return "Unknown".to_string();
    }
    format_duration(Duration::from_secs(secs as u64)).to_string()
}
fn format_run_time(started: &DateTime<Utc>, finished: &DateTime<Utc>) -> String {
    let secs = finished.signed_duration_since(*started).num_seconds();
    if secs < 0 {
        return "Unknown".to_string();
    }
    format_duration(Duration::from_secs(secs as u64)).to_string()
}
