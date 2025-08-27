use ratatui::style::Color;

/// The macro accepts a list of struct names with key names
/// Returns a struct where every key name is an Option<String>, with the correct derived attributes
macro_rules! optional_config_struct {
    ($($struct_name:ident, $($key_name:ident),*);*) => {
        $(
            #[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
            struct $struct_name {
                $(
                    $key_name: Option<String>,
                )*
            }
        )*
    };
}

/// The macro accepts a list of struct names with key names
macro_rules! config_struct {
    ($($struct_name:ident, $($key_name:ident),*);*) => {
        $(
            #[derive(Debug, Clone, PartialEq, Eq, Copy)]
            pub struct $struct_name {
                $(
                    pub $key_name: Color,
                )*
            }
        )*
    };
}

impl AppColors {
    fn parse_color(s: &str) -> Result<Color, String> {
        match s.to_lowercase().as_str() {
            "reset" => Ok(Color::Reset),
            "black" => Ok(Color::Black),
            "red" => Ok(Color::Red),
            "green" => Ok(Color::Green),
            "yellow" => Ok(Color::Yellow),
            "blue" => Ok(Color::Blue),
            "magenta" => Ok(Color::Magenta),
            "cyan" => Ok(Color::Cyan),
            "gray" | "grey" => Ok(Color::Gray),
            "darkgray" | "darkgrey" => Ok(Color::DarkGray),
            "lightred" => Ok(Color::LightRed),
            "lightgreen" => Ok(Color::LightGreen),
            "lightyellow" => Ok(Color::LightYellow),
            "lightblue" => Ok(Color::LightBlue),
            "lightmagenta" => Ok(Color::LightMagenta),
            "lightcyan" => Ok(Color::LightCyan),
            "white" => Ok(Color::White),
            _ => {
                // Try to parse RGB format: "rgb(r,g,b)" or "#RRGGBB"
                if s.starts_with("rgb(") && s.ends_with(')') {
                    let inner = &s[4..s.len() - 1];
                    let parts: Vec<&str> = inner.split(',').collect();
                    if parts.len() == 3 {
                        let r = parts[0]
                            .trim()
                            .parse::<u8>()
                            .map_err(|_| format!("Invalid RGB color: {s}"))?;
                        let g = parts[1]
                            .trim()
                            .parse::<u8>()
                            .map_err(|_| format!("Invalid RGB color: {s}"))?;
                        let b = parts[2]
                            .trim()
                            .parse::<u8>()
                            .map_err(|_| format!("Invalid RGB color: {s}"))?;
                        return Ok(Color::Rgb(r, g, b));
                    }
                } else if s.starts_with('#') && s.len() == 7 {
                    let r = u8::from_str_radix(&s[1..3], 16)
                        .map_err(|_| format!("Invalid hex color: {s}"))?;
                    let g = u8::from_str_radix(&s[3..5], 16)
                        .map_err(|_| format!("Invalid hex color: {s}"))?;
                    let b = u8::from_str_radix(&s[5..7], 16)
                        .map_err(|_| format!("Invalid hex color: {s}"))?;
                    return Ok(Color::Rgb(r, g, b));
                } else if let Ok(index) = s.parse::<u8>() {
                    return Ok(Color::Indexed(index));
                }
                Err(format!("Unknown color: {s}"))
            }
        }
    }

    fn map_color(color_str: Option<&str>, setter: &mut Color) {
        color_str.map(|i| Self::parse_color(i).map(|i| *setter = i).ok());
    }

    fn apply_headers_colors(config_colors: &ConfigColors, app_colors: &mut Self) {
        // Heading bar
        if let Some(hb) = &config_colors.headers_bar {
            Self::map_color(
                hb.background.as_deref(),
                &mut app_colors.headers_bar.background,
            );
            Self::map_color(
                hb.loading_spinner.as_deref(),
                &mut app_colors.headers_bar.loading_spinner,
            );
            Self::map_color(hb.text.as_deref(), &mut app_colors.headers_bar.text);
            Self::map_color(
                hb.text_selected.as_deref(),
                &mut app_colors.headers_bar.text_selected,
            );
        }
    }

    fn apply_ui_colors(config_colors: &ConfigColors, app_colors: &mut Self) {
        // Selectable panel borders
        if let Some(b) = &config_colors.borders {
            Self::map_color(b.selected.as_deref(), &mut app_colors.borders.selected);
            Self::map_color(b.unselected.as_deref(), &mut app_colors.borders.unselected);
        }

        // Filter panel
        if let Some(fc) = &config_colors.filter {
            Self::map_color(fc.background.as_deref(), &mut app_colors.filter.background);
            Self::map_color(fc.highlight.as_deref(), &mut app_colors.filter.highlight);

            Self::map_color(
                fc.selected_filter_background.as_deref(),
                &mut app_colors.filter.selected_filter_background,
            );
            Self::map_color(
                fc.selected_filter_text.as_deref(),
                &mut app_colors.filter.selected_filter_text,
            );
            Self::map_color(fc.text.as_deref(), &mut app_colors.filter.text);
        }
    }

    fn apply_popup_colors(config_colors: &ConfigColors, app_colors: &mut Self) {
        // Error Popup
        if let Some(ep) = &config_colors.popup_error {
            Self::map_color(
                ep.background.as_deref(),
                &mut app_colors.popup_error.background,
            );
            Self::map_color(ep.text.as_deref(), &mut app_colors.popup_error.text);
        }

        // Help Popup
        if let Some(hp) = &config_colors.popup_help {
            Self::map_color(
                hp.background.as_deref(),
                &mut app_colors.popup_help.background,
            );
            Self::map_color(hp.text.as_deref(), &mut app_colors.popup_help.text);
            Self::map_color(
                hp.text_highlight.as_deref(),
                &mut app_colors.popup_help.text_highlight,
            );
        }

        // Info Popup
        if let Some(ip) = &config_colors.popup_info {
            Self::map_color(
                ip.background.as_deref(),
                &mut app_colors.popup_info.background,
            );
            Self::map_color(ip.text.as_deref(), &mut app_colors.popup_info.text);
        }

        // Delete Popup
        if let Some(dp) = &config_colors.popup_delete {
            Self::map_color(
                dp.background.as_deref(),
                &mut app_colors.popup_delete.background,
            );
            Self::map_color(dp.text.as_deref(), &mut app_colors.popup_delete.text);
            Self::map_color(
                dp.text_highlight.as_deref(),
                &mut app_colors.popup_delete.text_highlight,
            );
        }
    }

    fn apply_chart_colors(config_colors: &ConfigColors, app_colors: &mut Self) {
        // Chart Cpu
        if let Some(cc) = &config_colors.chart_cpu {
            Self::map_color(
                cc.background.as_deref(),
                &mut app_colors.chart_cpu.background,
            );
            Self::map_color(cc.border.as_deref(), &mut app_colors.chart_cpu.border);
            Self::map_color(cc.max.as_deref(), &mut app_colors.chart_cpu.max);
            Self::map_color(cc.points.as_deref(), &mut app_colors.chart_cpu.points);
            Self::map_color(cc.title.as_deref(), &mut app_colors.chart_cpu.title);
            Self::map_color(cc.y_axis.as_deref(), &mut app_colors.chart_cpu.y_axis);
        }

        // Chart Memory
        if let Some(cm) = &config_colors.chart_memory {
            Self::map_color(
                cm.background.as_deref(),
                &mut app_colors.chart_memory.background,
            );
            Self::map_color(cm.border.as_deref(), &mut app_colors.chart_memory.border);
            Self::map_color(cm.max.as_deref(), &mut app_colors.chart_memory.max);
            Self::map_color(cm.points.as_deref(), &mut app_colors.chart_memory.points);
            Self::map_color(cm.title.as_deref(), &mut app_colors.chart_memory.title);
            Self::map_color(cm.y_axis.as_deref(), &mut app_colors.chart_memory.y_axis);
        }

        // Chart ports
        if let Some(cp) = &config_colors.chart_ports {
            Self::map_color(
                cp.background.as_deref(),
                &mut app_colors.chart_ports.background,
            );
            Self::map_color(cp.border.as_deref(), &mut app_colors.chart_ports.border);
            Self::map_color(cp.headings.as_deref(), &mut app_colors.chart_ports.headings);
            Self::map_color(cp.text.as_deref(), &mut app_colors.chart_ports.text);
            Self::map_color(cp.title.as_deref(), &mut app_colors.chart_ports.title);
        }
    }

    fn apply_panel_colors(config_colors: &ConfigColors, app_colors: &mut Self) {
        // Containers
        if let Some(c) = &config_colors.containers {
            Self::map_color(
                c.background.as_deref(),
                &mut app_colors.containers.background,
            );
            Self::map_color(c.icon.as_deref(), &mut app_colors.containers.icon);
            Self::map_color(c.text.as_deref(), &mut app_colors.containers.text);
            Self::map_color(c.text_rx.as_deref(), &mut app_colors.containers.text_rx);
            Self::map_color(c.text_tx.as_deref(), &mut app_colors.containers.text_tx);
        }

        // Commands
        if let Some(cc) = &config_colors.commands {
            Self::map_color(
                cc.background.as_deref(),
                &mut app_colors.commands.background,
            );
            Self::map_color(cc.pause.as_deref(), &mut app_colors.commands.pause);
            Self::map_color(cc.restart.as_deref(), &mut app_colors.commands.restart);
            Self::map_color(cc.stop.as_deref(), &mut app_colors.commands.stop);
            Self::map_color(cc.delete.as_deref(), &mut app_colors.commands.start);
            Self::map_color(cc.resume.as_deref(), &mut app_colors.commands.resume);
            Self::map_color(cc.start.as_deref(), &mut app_colors.commands.start);
        }

        // Logs panel
        if let Some(cl) = &config_colors.logs {
            Self::map_color(cl.background.as_deref(), &mut app_colors.logs.background);
            Self::map_color(cl.text.as_deref(), &mut app_colors.logs.text);
        }
    }

    fn apply_container_state_colors(config_colors: &ConfigColors, app_colors: &mut Self) {
        // Container State
        if let Some(cs) = &config_colors.container_state {
            Self::map_color(cs.dead.as_deref(), &mut app_colors.container_state.dead);
            Self::map_color(cs.exited.as_deref(), &mut app_colors.container_state.exited);
            Self::map_color(cs.paused.as_deref(), &mut app_colors.container_state.paused);
            Self::map_color(
                cs.removing.as_deref(),
                &mut app_colors.container_state.removing,
            );
            Self::map_color(
                cs.restarting.as_deref(),
                &mut app_colors.container_state.restarting,
            );
            Self::map_color(
                cs.running_healthy.as_deref(),
                &mut app_colors.container_state.running_healthy,
            );
            Self::map_color(
                cs.running_unhealthy.as_deref(),
                &mut app_colors.container_state.running_unhealthy,
            );
            Self::map_color(
                cs.unknown.as_deref(),
                &mut app_colors.container_state.unknown,
            );
        }
    }
}

impl From<Option<ConfigColors>> for AppColors {
    fn from(value: Option<ConfigColors>) -> Self {
        let mut app_colors = Self::new();

        if let Some(config_colors) = value {
            Self::apply_headers_colors(&config_colors, &mut app_colors);
            Self::apply_ui_colors(&config_colors, &mut app_colors);
            Self::apply_popup_colors(&config_colors, &mut app_colors);
            Self::apply_chart_colors(&config_colors, &mut app_colors);
            Self::apply_panel_colors(&config_colors, &mut app_colors);
            Self::apply_container_state_colors(&config_colors, &mut app_colors);
        }
        app_colors
    }
}

const ORANGE: Color = Color::Rgb(255, 178, 36);

optional_config_struct!(
    ConfigBackgroundText, background, text;
    ConfigBackgroundTextHighlight, background, text, text_highlight;
    ConfigBorders, selected, unselected;
    ConfigChartCpu, background, border, order, title, max, points,y_axis;
    ConfigChartMemory, background, border, title, max, points, y_axis;
    ConfigChartPorts, background, border, title, headings, text;
    ConfigCommands, background, pause, restart, stop, delete, resume, start;
    ConfigContainers, background, icon, text, text_rx, text_tx;
    ConfigContainerState, background, dead, exited, paused, removing, restarting, running_healthy, running_unhealthy, unknown;
    ConfigFilter, background, text, selected_filter_background, selected_filter_text, highlight;
    ConfigHeadersBar, background, loading_spinner, text, text_selected;
    ConfigLogs, background, text
);

config_struct!(
    Borders, selected, unselected;
    ChartCpu, background, border, title, max, points, y_axis;
    ChartMemory, background, border, title, max, points, y_axis;
    ChartPorts, background, border, title, headings, text;
    Commands, background, pause, restart, stop, delete, resume, start;
    Containers, background, icon, text, text_rx, text_tx;
    ContainerState, dead, exited, paused, removing, restarting, running_healthy, running_unhealthy, unknown;
    Filter, background, text, selected_filter_background, selected_filter_text, highlight;
    HeadersBar, background, text_selected, loading_spinner, text;
    Logs, background, text;
    PopupDelete, background, text, text_highlight;
    PopupError, background, text;
    PopupHelp, background, text, text_highlight;
    PopupInfo, background, text
);

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ConfigColors {
    borders: Option<ConfigBorders>,
    chart_cpu: Option<ConfigChartCpu>,
    chart_memory: Option<ConfigChartMemory>,
    chart_ports: Option<ConfigChartPorts>,
    commands: Option<ConfigCommands>,
    container_state: Option<ConfigContainerState>,
    containers: Option<ConfigContainers>,
    filter: Option<ConfigFilter>,
    headers_bar: Option<ConfigHeadersBar>,
    logs: Option<ConfigLogs>,
    popup_delete: Option<ConfigBackgroundTextHighlight>,
    popup_error: Option<ConfigBackgroundText>,
    popup_help: Option<ConfigBackgroundTextHighlight>,
    popup_info: Option<ConfigBackgroundText>,
}

/// Default colours for the header bar
impl HeadersBar {
    const fn new() -> Self {
        Self {
            background: Color::Magenta,
            loading_spinner: Color::White,
            text: Color::Black,
            text_selected: Color::Gray,
        }
    }
}

/// Default colours for the borders
impl Borders {
    const fn new() -> Self {
        Self {
            selected: Color::LightCyan,
            unselected: Color::Gray,
        }
    }
}

/// Default colours for the delete popup
impl Commands {
    const fn new() -> Self {
        Self {
            background: Color::Reset,
            pause: Color::Yellow,
            restart: Color::Magenta,
            stop: Color::Red,
            delete: Color::Gray,
            resume: Color::Blue,
            start: Color::Green,
        }
    }
}

/// Default colours for the help popup
impl ChartCpu {
    const fn new() -> Self {
        Self {
            background: Color::Reset,
            border: Color::White,
            title: Color::Green,
            max: ORANGE,
            points: Color::Magenta,
            y_axis: Color::White,
        }
    }
}

/// Default colours for the help popup
impl ChartMemory {
    const fn new() -> Self {
        Self {
            background: Color::Reset,
            border: Color::White,
            title: Color::Green,
            max: ORANGE,
            points: Color::Cyan,
            y_axis: Color::White,
        }
    }
}

/// Default colours for the help popup
impl ChartPorts {
    const fn new() -> Self {
        Self {
            background: Color::Reset,
            border: Color::White,
            title: Color::Green,
            headings: Color::Yellow,
            text: Color::White,
        }
    }
}

/// Default colours for the help popup
impl Containers {
    const fn new() -> Self {
        Self {
            background: Color::Reset,
            icon: Color::White,
            text: Color::Blue,
            text_rx: Color::Rgb(255, 233, 193),
            text_tx: Color::Rgb(205, 140, 140),
        }
    }
}

/// Default colours for the help popup
impl ContainerState {
    const fn new() -> Self {
        Self {
            paused: Color::Yellow,
            removing: Color::LightRed,
            restarting: Color::LightGreen,
            running_healthy: Color::Green,
            running_unhealthy: ORANGE,
            dead: Color::Red,
            exited: Color::Red,
            unknown: Color::Red,
        }
    }
}

/// Default colours for the filter panel
impl Filter {
    const fn new() -> Self {
        Self {
            background: Color::Reset,
            highlight: Color::Magenta,
            selected_filter_background: Color::Gray,
            selected_filter_text: Color::Black,
            text: Color::Gray,
        }
    }
}

/// Default colours for the logs panel, only applied if color_logs is false
impl Logs {
    const fn new() -> Self {
        Self {
            background: Color::Reset,
            text: Color::Reset,
        }
    }
}

/// Default colours for the Error popup
impl PopupError {
    const fn new() -> Self {
        Self {
            background: Color::Red,
            text: Color::White,
        }
    }
}

/// Default colours for the info popup
impl PopupInfo {
    const fn new() -> Self {
        Self {
            background: Color::Blue,
            text: Color::White,
        }
    }
}

/// Default colours for the help popup
impl PopupHelp {
    const fn new() -> Self {
        Self {
            background: Color::Magenta,
            text: Color::Black,
            text_highlight: Color::White,
        }
    }
}

/// Default colours for the delete popup
impl PopupDelete {
    const fn new() -> Self {
        Self {
            background: Color::White,
            text: Color::Black,
            text_highlight: Color::Red,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct AppColors {
    pub borders: Borders,
    pub chart_cpu: ChartCpu,
    pub chart_memory: ChartMemory,
    pub chart_ports: ChartPorts,
    pub commands: Commands,
    pub container_state: ContainerState,
    pub containers: Containers,
    pub filter: Filter,
    pub headers_bar: HeadersBar,
    pub logs: Logs,
    pub popup_delete: PopupDelete,
    pub popup_error: PopupError,
    pub popup_help: PopupHelp,
    pub popup_info: PopupInfo,
}

impl Default for AppColors {
    fn default() -> Self {
        Self::new()
    }
}

impl AppColors {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            borders: Borders::new(),
            chart_cpu: ChartCpu::new(),
            chart_memory: ChartMemory::new(),
            chart_ports: ChartPorts::new(),
            commands: Commands::new(),
            container_state: ContainerState::new(),
            containers: Containers::new(),
            filter: Filter::new(),
            headers_bar: HeadersBar::new(),
            logs: Logs::new(),
            popup_delete: PopupDelete::new(),
            popup_error: PopupError::new(),
            popup_help: PopupHelp::new(),
            popup_info: PopupInfo::new(),
        }
    }
}
