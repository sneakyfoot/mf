use color_eyre::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Row, Table, TableState},
};

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}

struct Data {
    name: String,
    status: String,
    age: String,
}

struct App {
    state: TableState,
    items: Vec<Data>,
}

impl App {
    fn new() -> Self {
        Self {
            state: TableState::default().with_selected(0),
            items: sample_data(),
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
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

    fn draw(&mut self, frame: &mut Frame) {
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

fn sample_data() -> Vec<Data> {
    vec![
        Data {
            name: "render-beauty-fkas45".into(),
            status: "Running".into(),
            age: "24m".into(),
        },
        Data {
            name: "sim-pyro-3daf4".into(),
            status: "Pending".into(),
            age: "95m".into(),
        },
    ]
}
