use crate::data::{Data, fetch_data};
use crate::k8s::set_host_schedulable;
use k8s_openapi::chrono::{DateTime, Utc};
use kube::Client;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    widgets::TableState,
};
use std::error::Error;
use std::time::Duration;
use tokio::runtime::Runtime;
pub mod logs;
pub mod table;
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
    client: Client,
}
enum Mode {
    Table,
    Logs { pod: String, start: DateTime<Utc> },
}
impl App {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let rt = Runtime::new()?;
        let client = rt.block_on(Client::try_default())?;
        let items = rt.block_on(fetch_data(client.clone()))?;
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
            client,
        })
    }
    /// Main app loop
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<(), Box<dyn Error>> {
        let tick = Duration::from_millis(500);
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
                    if let Ok(items) = self.rt.block_on(fetch_data(self.client.clone())) {
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
    /// Keybinds
    fn handle_key(&mut self, key: event::KeyEvent) -> Result<bool, Box<dyn Error>> {
        match &self.mode {
            // Keybinds while in default pod table
            Mode::Table => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
                KeyCode::Char('j') | KeyCode::Down => self.next(),
                KeyCode::Char('k') | KeyCode::Up => self.previous(),
                KeyCode::Char('D') => self.delete_key(),
                KeyCode::Char('p') => {
                    if let Err(e) =
                        self.rt
                            .block_on(set_host_schedulable(self.client.clone(), None, true))
                    {
                        eprintln!("Failed to mark host schedulable: {}", e);
                    }
                }
                KeyCode::Char('o') => {
                    if let Err(e) =
                        self.rt
                            .block_on(set_host_schedulable(self.client.clone(), None, false))
                    {
                        eprintln!("Failed to mark host unschedulable: {}", e);
                    }
                }
                KeyCode::Enter => self.start_log_mode(),
                _ => {}
            },
            // Keybinds while in log mode
            Mode::Logs { pod: _, start: _ } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.exit_log_mode(),
                    KeyCode::Char('k') | KeyCode::Up => self.scroll_logs(false),
                    KeyCode::Char('j') | KeyCode::Down => self.scroll_logs(true),
                    _ => {}
                }
                return Ok(false);
            }
        }
        Ok(false)
    }
}
