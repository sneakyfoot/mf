use crate::data::{Data, fetch_data};
use humantime::format_duration;
use k8s_openapi::chrono::{DateTime, Utc};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Row, Table, TableState},
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
    logs: Vec<String>,
}

enum Mode {
    Table,
    Logs { pod: String },
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
            logs: Vec::new(),
        })
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<(), Box<dyn Error>> {
        let tick = Duration::from_secs(1);
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(tick)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if self.handle_key(key)? {
                            return Ok(());
                        }
                    }
                }
            } else {
                if let Ok(items) = self.rt.block_on(fetch_data()) {
                    self.items = items;
                }
            }
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        match &self.mode {
            Mode::Table => self.draw_table(frame),
            Mode::Logs { pod } => {
                let pod = pod.clone();
                self.draw_logs(frame, &pod);
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
            Row::new(vec![item.name.clone(), item.status.clone(), age])
        });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(25),
                Constraint::Percentage(45),
                Constraint::Percentage(30),
            ],
        )
        .header(Row::new(vec!["Name", "Status", "Age"]))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

        frame.render_stateful_widget(table, frame.area(), &mut self.state);
    }

    // Log view
    fn draw_logs(&mut self, frame: &mut Frame, pod: &str) {
        let block = ratatui::widgets::Block::default()
            .title(format!("Logs for {}", pod))
            .borders(ratatui::widgets::Borders::ALL);
        frame.render_widget(block, frame.area());
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
            Mode::Logs { pod } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.mode = Mode::Table;
                        self.logs.clear();
                    }
                    _ => {}
                }
                return Ok(false);
            }
        }
        Ok(false)
    }

    fn start_log_mode(&mut self) {
        if let Some(idx) = self.state.selected().and_then(|i| self.items.get(i)) {
            self.mode = Mode::Logs {
                pod: idx.name.clone(),
            };
            self.logs.clear();
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) if i + 1 < self.items.len() => i + 1,
            _ => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(0) | None => self.items.len().saturating_sub(1),
            Some(i) => i - 1,
        };
        self.state.select(Some(i));
    }
}

fn format_age(created: &DateTime<Utc>) -> String {
    let secs = Utc::now().signed_duration_since(*created).num_seconds();
    if secs < 0 {
        return "Unknown".to_string();
    }
    format_duration(Duration::from_secs(secs as u64)).to_string()
}
