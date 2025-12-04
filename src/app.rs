use crate::data::{Data, fetch_data};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Row, Table, TableState},
};
use std::error::Error;
use tokio::runtime::Runtime;

pub struct App {
    state: TableState,
    items: Vec<Data>,
}

impl App {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let rt = Runtime::new()?;
        let items = rt.block_on(fetch_data())?;
        Ok(Self {
            state: TableState::default().with_selected(0),
            items,
        })
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

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
            Row::new(vec![
                item.name.as_str(),
                item.status.as_str(),
                item.age.as_str(),
            ])
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
