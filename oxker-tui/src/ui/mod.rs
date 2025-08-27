use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use parking_lot::Mutex;
use ratatui::{Frame, Terminal, backend::CrosstermBackend, layout::Position};
use std::{
    io::{self, Stdout, Write},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use std::{sync::atomic::AtomicBool, time::Instant};
use tokio::sync::mpsc::Sender;
use tracing::error;

// draw_blocks module removed - functionality migrated to components
mod exec_integration;
mod gui_state;
mod redraw;
mod view_models;

// Component-based architecture
pub mod components;
pub mod views;

pub use redraw::Rerender;
pub use view_models::{ChartData, CommandsView, ContainerView, FrameViewModel, LogView, PortView};

pub use self::gui_state::{DeleteButton, GuiState, SelectablePanel, Status};
use crate::handlers::UIContainerState;
use crate::input_handler::InputMessages;
use oxker_core::{AppColors, AppError, Config, Keymap};

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
    cached_view_model: Option<FrameViewModel>,
}

impl Ui {
    /// Enable mouse capture, but don't enable capture of all the mouse movements, doing so will improve performance, and is part of the fix for the weird mouse event output bug
    ///
    /// # Errors
    ///
    /// Returns an error if writing to stdout fails
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
                    cached_view_model: None,
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
    ///
    /// # Errors
    ///
    /// Returns an error if terminal operations fail
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
                        use components::{
                            Component,
                            panels::error::{ErrorPanel, ErrorPanelProps},
                        };
                        let error_panel = ErrorPanel::new();
                        let props = ErrorPanelProps {
                            error: &AppError::DockerConnect,
                            theme: &colors,
                            keymap: &keymap,
                            auto_close_seconds: Some(seconds),
                        };
                        error_panel.render(&props, f.area(), f);
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
            if let Err(_e) = exec_integration::run_exec_mode(mode, &self.terminal).await {
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
        // Check if immediate redraw is requested
        if self.rerender.swap_draw() {
            *previous = std::time::Instant::now();
            return true;
        }

        // Otherwise check if enough time has passed for docker update
        if previous.elapsed().as_millis() >= docker_interval_ms {
            *previous = std::time::Instant::now();
            return true;
        }

        false
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

                // Check if we need to recreate the view model
                let needs_update = {
                    let ui_state = self.container_state.lock();
                    self.cached_view_model.is_none() || ui_state.has_significant_changes()
                };

                if needs_update {
                    let mut ui_state = self.container_state.lock();
                    self.cached_view_model = Some(FrameViewModel::from_state(
                        &ui_state,
                        &self.gui_state.lock(),
                        colors,
                        screen_width,
                    ));
                    ui_state.mark_changes_rendered();
                }

                if let Some(fd) = &self.cached_view_model {
                    let exec = fd.status.contains(&Status::Exec);

                    if exec {
                        self.exec().await;
                    } else if self
                        .terminal
                        .draw(|frame| {
                            draw_frame(
                                &self.container_state,
                                &self.config,
                                colors,
                                &keymap,
                                frame,
                                fd,
                                &self.gui_state,
                            );
                        })
                        .is_err()
                    {
                        return Err(AppError::Terminal);
                    }
                }
            }

            if crossterm::event::poll(POLL_RATE).unwrap_or(false)
                && let Ok(event) = event::read()
            {
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
                    // Invalidate cached view model on resize
                    self.cached_view_model = None;
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
    _colors: AppColors,
    keymap: &Keymap,
    f: &mut Frame,
    fd: &FrameViewModel,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    // Use the component-based MainView
    use views::{View, main_view::MainView};
    // TODO: Consider caching MainView instance to avoid recreation
    let main_view = MainView::new(config, keymap, gui_state, container_state);
    main_view.render(fd, f);
}
