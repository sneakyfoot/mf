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
use std::time::Duration;

use std::error::Error;
use tokio::runtime::Runtime;

pub struct App {
    state: TableState,
    items: Vec<Data>,
    rt: Runtime,
}

impl App {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let rt = Runtime::new()?;
        let items = rt.block_on(fetch_data())?;
        Ok(Self {
            state: TableState::default().with_selected(0),
            items,
            rt,
        })
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<(), Box<dyn Error>> {
        let tick = Duration::from_secs(1);
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(tick)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('j') | KeyCode::Down => self.next(),
                            KeyCode::Char('k') | KeyCode::Up => self.previous(),
                            _ => {}
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

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) if i + 1 < self.items.len() => i + 1,
            _ => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(0) | None => self.items.len().saturating_sub(1),
            Some(i) => i - 1,
        };
        self.state.select(Some(i));
    }

    pub fn draw(&mut self, frame: &mut Frame) {
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
}

fn format_age(created: &DateTime<Utc>) -> String {
    let secs = Utc::now().signed_duration_since(*created).num_seconds();
    if secs < 0 {
        return "Unknown".to_string();
    }
    format_duration(Duration::from_secs(secs as u64)).to_string()
}
