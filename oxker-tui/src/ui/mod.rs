use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use parking_lot::Mutex;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Position},
};
use std::{
    collections::HashSet,
    io::{self, Stdout, Write},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use std::{sync::atomic::AtomicBool, time::Instant};
use tokio::sync::mpsc::Sender;
use tracing::error;

mod draw_blocks;
mod gui_state;
mod redraw;
mod view_models;
pub use redraw::Rerender;
pub use view_models::{FrameViewModel, ContainerView, ChartData, PortView, LogView, CommandsView};

pub use self::gui_state::{DeleteButton, GuiState, SelectablePanel, Status};
use crate::input_handler::InputMessages;
use crate::handlers::UIContainerState;
use oxker_core::{
    AppData, AppError,
    AppColors, Keymap,
    ContainerId, State, Header,
    Columns, ContainerPorts, CpuTuple, FilterBy, MemTuple, SortedOrder,
    TerminalSize, Config,
};

const POLL_RATE: Duration = std::time::Duration::from_millis(50);

// could have a render struct, which takes in poll rate, and docker

pub struct Ui {
    container_state: Arc<Mutex<UIContainerState>>,
    config: Config,
    cursor_position: Position,
    gui_state: Arc<Mutex<GuiState>>,
    input_tx: Sender<InputMessages>,
    is_running: Arc<AtomicBool>,
    now: Instant,
    rerender: Arc<Rerender>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Ui {
    /// Enable mouse capture, but don't enable capture of all the mouse movements, doing so will improve performance, and is part of the fix for the weird mouse event output bug
    pub fn enable_mouse_capture() -> Result<()> {
        Ok(io::stdout().write_all(
            concat!(
                crossterm::csi!("?1000h"),
                crossterm::csi!("?1015h"),
                crossterm::csi!("?1006h"),
            )
            .as_bytes(),
        )?)
    }

    /// Create a new Ui struct, and execute the drawing loop
    pub async fn start(
        container_state: Arc<Mutex<UIContainerState>>,
        config: Config,
        gui_state: Arc<Mutex<GuiState>>,
        input_tx: Sender<InputMessages>,
        is_running: Arc<AtomicBool>,
        rerender: Arc<Rerender>,
    ) {
        match Self::setup_terminal() {
            Ok(mut terminal) => {
                let cursor_position = terminal.get_cursor_position().unwrap_or_default();
                let mut ui = Self {
                    container_state,
                    config,
                    cursor_position,
                    gui_state,
                    input_tx,
                    is_running,
                    now: Instant::now(),
                    rerender,
                    terminal,
                };
                if let Err(e) = ui.draw_ui().await {
                    error!("{e}");
                }
                if let Err(e) = ui.reset_terminal() {
                    error!("{e}");
                }
            }
            _ => {
                error!("Terminal Error");
            }
        }
    }

    /// Setup the terminal for full-screen drawing mode, with mouse capture
    fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
        let stdout = Self::init_terminal()?;
        let backend = CrosstermBackend::new(stdout);
        Ok(Terminal::new(backend)?)
    }

    fn init_terminal() -> Result<Stdout> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        Self::enable_mouse_capture()?;
        Ok(stdout)
    }

    /// reset the terminal back to default settings
    pub fn reset_terminal(&mut self) -> Result<()> {
        self.terminal.clear()?;

        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        disable_raw_mode()?;
        self.terminal.clear().ok();
        self.terminal.set_cursor_position(self.cursor_position)?;
        Ok(self.terminal.show_cursor()?)
    }

    /// Draw the the error message ui, for 5 seconds, with a countdown
    fn err_loop(&mut self) -> Result<(), AppError> {
        let mut seconds = 5;
        let colors = self.config.app_colors;
        let keymap = self.config.keymap.clone();
        let mut redraw = true;
        loop {
            if self.now.elapsed() >= std::time::Duration::from_secs(1) {
                seconds -= 1;
                self.now = Instant::now();
                redraw = true;
                if seconds < 1 {
                    break;
                }
            }

            if redraw
                && self
                    .terminal
                    .draw(|f| {
                        draw_blocks::error::draw(
                            colors,
                            &AppError::DockerConnect,
                            f,
                            &keymap,
                            Some(seconds),
                        );
                    })
                    .is_err()
            {
                return Err(AppError::Terminal);
            }
            redraw = false;
            std::thread::sleep(POLL_RATE);
        }
        Ok(())
    }

    /// Check if the user has attempt to clear the screen, and if so clear and redraw
    fn check_clear(&mut self) {
        if self.rerender.get_clear() {
            self.terminal.clear().ok();
            self.rerender.update_draw();
        }
    }
    /// Use external docker cli to exec into a container
    async fn exec(&mut self) {
        let exec_mode = self.gui_state.lock().get_exec_mode();

        if let Some(mode) = exec_mode {
            self.reset_terminal().ok();
            self.terminal.clear().ok();
            if let Err(e) = mode.run(TerminalSize::new(&self.terminal)).await {
                // TODO: Need to handle errors differently now that we don't have AppData
                // For now, just update the gui_state
                self.gui_state.lock().status_push(Status::Error);
            }
        }
        self.terminal.clear().ok();
        self.reset_terminal().ok();
        Self::init_terminal().ok();
        self.gui_state.lock().status_del(Status::Exec);
    }

    /// Use the previously redrawn time, the current time, the docker_interval, and the redraw struct, to calculate
    /// if the screen should be redrawn or not
    fn should_redraw(&self, previous: &mut Instant, docker_interval_ms: u128) -> bool {
        let result =
            self.rerender.swap_draw() || previous.elapsed().as_millis() >= docker_interval_ms;
        if result {
            *previous = std::time::Instant::now();
        }
        result
    }

    /// The loop for drawing the main UI to the terminal
    async fn gui_loop(&mut self) -> Result<(), AppError> {
        let colors = self.config.app_colors;
        let keymap = self.config.keymap.clone();
        let docker_interval_ms = u128::from(self.config.docker_interval_ms);
        let mut drawn_at = std::time::Instant::now();

        if let Ok(size) = self.terminal.size() {
            self.gui_state.lock().set_screen_width(size.width);
        }

        while self.is_running.load(Ordering::SeqCst) {
            // if self.redraw.get_clear() {
            //     self.terminal.clear().ok();
            //     continue;
            // }
            if self.should_redraw(&mut drawn_at, docker_interval_ms) {
                let screen_width = self.gui_state.lock().get_screen_width();
                let fd = FrameViewModel::from_state(
                    &*self.container_state.lock(),
                    &*self.gui_state.lock(),
                    colors,
                    screen_width,
                );

                let exec = fd.status.contains(&Status::Exec);
                if exec {
                    self.exec().await;
                }

                if self
                    .terminal
                    .draw(|frame| {
                        draw_frame(&self.container_state, &self.config, colors, &keymap, frame, &fd, &self.gui_state);
                    })
                    .is_err()
                {
                    return Err(AppError::Terminal);
                }
            }

            if crossterm::event::poll(POLL_RATE).unwrap_or(false) {
                if let Ok(event) = event::read() {
                    if let Event::Key(key) = event {
                        if key.kind == event::KeyEventKind::Press {
                            self.input_tx
                                .send(InputMessages::ButtonPress((key.code, key.modifiers)))
                                .await
                                .ok();
                        }
                    } else if let Event::Mouse(m) = event {
                        match m.kind {
                            event::MouseEventKind::Down(_)
                            | event::MouseEventKind::ScrollDown
                            | event::MouseEventKind::ScrollUp => {
                                self.input_tx
                                    .send(InputMessages::MouseEvent((m, m.modifiers)))
                                    .await
                                    .ok();
                            }
                            _ => (),
                        }
                    } else if let Event::Resize(width, _) = event {
                        self.gui_state.lock().clear_area_map();
                        self.terminal.autoresize().ok();
                        self.gui_state.lock().set_screen_width(width);
                    }
                }
            }
            self.check_clear();
        }
        Ok(())
    }

    /// Draw either the Error, or main oxker ui, to the terminal
    async fn draw_ui(&mut self) -> Result<(), AppError> {
        let status = self.gui_state.lock().get_status();
        if status.contains(&Status::DockerConnect) {
            self.err_loop()?;
        } else {
            self.gui_loop().await?;
        }
        Ok(())
    }
}


/// Draw the main ui to a frame of the terminal
fn draw_frame(
    container_state: &Arc<Mutex<UIContainerState>>,
    config: &Config,
    colors: AppColors,
    keymap: &Keymap,
    f: &mut Frame,
    fd: &FrameViewModel,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    let whole_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if fd.status.contains(&Status::Filter) {
            vec![Constraint::Max(1), Constraint::Min(1), Constraint::Max(1)]
        } else {
            vec![Constraint::Max(1), Constraint::Min(1)]
        })
        .split(f.area());

    draw_blocks::headers::draw(whole_layout[0], colors, f, fd, gui_state, keymap);

    // If required, draw filter bar
    if let Some(rect) = whole_layout.get(2) {
        draw_blocks::filter::draw(*rect, colors, f, fd);
    }

    let upper_main = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if fd.has_containers {
            vec![Constraint::Percentage(75), Constraint::Percentage(25)]
        } else {
            vec![Constraint::Percentage(100), Constraint::Percentage(0)]
        })
        .split(whole_layout[1]);

    let containers_logs_section = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if fd.show_logs {
            vec![Constraint::Min(6), Constraint::Percentage(fd.log_height)]
        } else {
            vec![Constraint::Percentage(100)]
        })
        .split(upper_main[0]);

    // Containers + docker commands
    let containers_commands = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if fd.has_containers {
            vec![Constraint::Percentage(90), Constraint::Percentage(10)]
        } else {
            vec![Constraint::Percentage(100)]
        })
        .split(containers_logs_section[0]);

    draw_blocks::containers::draw(container_state, containers_commands[0], colors, f, fd, gui_state);

    if fd.show_logs {
        draw_blocks::logs::draw(
            container_state,
            containers_logs_section[1],
            colors,
            f,
            fd,
            gui_state,
        );
    }

    if let Some(id) = fd.delete_confirm.as_ref() {
        // Find container name from UIContainerState
        let container_name = container_state.lock()
            .get_container_items()
            .iter()
            .find(|c| &c.id == id)
            .map(|c| c.name.clone());
            
        if let Some(name) = container_name {
            draw_blocks::delete_confirm::draw(colors, f, gui_state, keymap, &name);
        } else {
            // If a container is deleted outside of oxker but whilst the Delete Confirm dialog is open, it can get caught in kind of a dead lock situation
            // so if in that unique situation, just clear the delete_container id
            gui_state.lock().set_delete_container(None);
        }
    }

    // only draw commands + charts if there are containers
    if let Some(rect) = containers_commands.get(1) {
        draw_blocks::commands::draw(container_state, *rect, colors, f, fd, gui_state);

        // Can calculate the max string length here, and then use that to keep the ports section as small as possible (+4 for some padding + border)
        let ports_len = if let Some(port_view) = &fd.port_view {
            u16::try_from(port_view.max_lens.0 + port_view.max_lens.1 + port_view.max_lens.2 + 2)
                .unwrap_or(26)
        } else {
            26
        };

        let lower = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Max(ports_len)])
            .split(upper_main[1]);

        draw_blocks::charts::draw(lower[0], colors, f, fd);
        draw_blocks::ports::draw(lower[1], colors, f, fd);
    }

    if let Some((text, instant)) = fd.info_text.as_ref() {
        draw_blocks::info::draw(colors, f, gui_state, instant, text.to_owned());
    }

    // Check if error, and show popup if so
    if fd.status.contains(&Status::Help) {
        let tz = config.timezone.clone();
        draw_blocks::help::draw(
            colors,
            f,
            keymap,
            config.show_timestamp,
            tz.as_ref(),
        );
    }

    if let Some(error) = fd.has_error.as_ref() {
        draw_blocks::error::draw(colors, error, f, keymap, None);
    }
}
