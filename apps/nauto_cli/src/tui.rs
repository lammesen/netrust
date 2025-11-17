use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nauto_model::Device;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io::stdout;
use std::time::Duration;

pub async fn launch(devices: Vec<Device>) -> Result<()> {
    tokio::task::spawn_blocking(move || run_ui(devices)).await??;
    Ok(())
}

fn run_ui(devices: Vec<Device>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = AppState::new(devices);

    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => app.next(),
                    KeyCode::Up => app.previous(),
                    _ => {}
                }
            }
        }
    }

    cleanup_terminal(&mut terminal)?;
    Ok(())
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

struct AppState {
    devices: Vec<Device>,
    list_state: ListState,
}

impl AppState {
    fn new(devices: Vec<Device>) -> Self {
        let mut list_state = ListState::default();
        if !devices.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            devices,
            list_state,
        }
    }

    fn next(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        let next = match self.list_state.selected() {
            Some(i) if i >= self.devices.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.list_state.select(Some(next));
    }

    fn previous(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        let prev = match self.list_state.selected() {
            Some(0) | None => self.devices.len() - 1,
            Some(i) => i - 1,
        };
        self.list_state.select(Some(prev));
    }

    fn selected_device(&self) -> Option<&Device> {
        self.list_state
            .selected()
            .and_then(|idx| self.devices.get(idx))
    }
}

fn draw(f: &mut ratatui::Frame, app: &mut AppState) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(f.size());

    let items: Vec<ListItem> = app
        .devices
        .iter()
        .map(|d| ListItem::new(format!("{} ({:?})", d.name, d.device_type)))
        .collect();

    let devices = List::new(items)
        .block(Block::default().title("Devices").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Cyan));

    f.render_stateful_widget(devices, layout[0], &mut app.list_state);

    let details = if let Some(device) = app.selected_device() {
        format!(
            "ID: {}\nAddress: {}\nTags: {}\nDriver: {:?}",
            device.id,
            device.mgmt_address,
            device.tags.join(", "),
            device.device_type
        )
    } else {
        "No device selected".into()
    };

    let detail_block =
        Paragraph::new(details).block(Block::default().title("Details").borders(Borders::ALL));
    f.render_widget(detail_block, layout[1]);
}
