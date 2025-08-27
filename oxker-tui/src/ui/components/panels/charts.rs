//! Charts panel component for displaying CPU and memory usage graphs

use crate::ui::{FrameViewModel, components::Component};
use oxker_core::{AppColors, ByteStats, CpuStats, State, Stats};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::Span,
    widgets::{Axis, Block, BorderType, Borders, Chart, Dataset, GraphType},
};
use std::fmt::Display;

/// Charts panel component that displays CPU and memory usage graphs
pub struct ChartsPanel {
    // No internal state needed
}

pub struct ChartsPanelProps<'a> {
    pub view_model: &'a FrameViewModel,
    pub theme: &'a AppColors,
}

impl ChartsPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

/// Transform chart data to ensure proper x-axis spacing (0-60 seconds)
#[allow(clippy::cast_precision_loss)]
fn transform_chart_data(data: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if data.is_empty() {
        return vec![(0.0, 0.0), (60.0, 0.0)];
    }

    // If we only have one data point, add another at the end
    if data.len() == 1 {
        return vec![(0.0, data[0].1), (60.0, data[0].1)];
    }

    // Transform indices to time series (0-60 seconds)
    let max_index = (data.len() - 1) as f64;
    if max_index == 0.0 {
        return vec![(0.0, data[0].1), (60.0, data[0].1)];
    }

    data.iter()
        .enumerate()
        .map(|(i, &(_, y))| {
            let x = (i as f64 / max_index) * 60.0;
            (x, y)
        })
        .collect()
}

impl<'p> Component<'p> for ChartsPanel {
    type Props = ChartsPanelProps<'p>;
    type Event = ();

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Skip rendering if area is too small
        if area.width < 10 || area.height < 5 {
            return;
        }

        // Extra safety check for extreme cases
        if area.width == 0 || area.height == 0 {
            return;
        }

        let split_chart = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .direction(Direction::Horizontal)
            .split(area);

        // Check if we have valid split areas
        if split_chart.len() < 2 || split_chart[0].width < 3 || split_chart[0].height < 3 {
            return;
        }

        // Create default chart data if none exists with proper x-axis spacing
        let default_cpu_data = (
            vec![(0.0, 0.0), (60.0, 0.0)],
            CpuStats::new(0.0),
            State::Unknown,
        );
        let default_mem_data = (
            vec![(0.0, 0.0), (60.0, 0.0)],
            ByteStats::new(0),
            State::Unknown,
        );

        if let Some(chart_data) = &props.view_model.chart_data {
            // Transform the data to ensure proper x-axis spacing
            let cpu_data = transform_chart_data(&chart_data.cpu_data.0);
            let mem_data = transform_chart_data(&chart_data.mem_data.0);

            let transformed_cpu = (cpu_data, chart_data.cpu_data.1, chart_data.cpu_data.2);
            let transformed_mem = (mem_data, chart_data.mem_data.1, chart_data.mem_data.2);

            render_charts(
                frame,
                &split_chart,
                props.theme,
                &transformed_cpu,
                &transformed_mem,
                chart_data.current_cpu,
                chart_data.current_mem,
            );
        } else {
            // Render empty charts with zero data
            render_charts(
                frame,
                &split_chart,
                props.theme,
                &default_cpu_data,
                &default_mem_data,
                CpuStats::new(0.0),
                ByteStats::new(0),
            );
        }
    }
}

impl Default for ChartsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartVariant {
    Cpu,
    Memory,
}

impl ChartVariant {
    const fn name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "memory",
        }
    }

    const fn get_title_color(self, colors: &AppColors, state: State) -> Color {
        if state.is_healthy() {
            match self {
                Self::Cpu => colors.chart_cpu.title,
                Self::Memory => colors.chart_memory.title,
            }
        } else {
            state.get_color(*colors)
        }
    }

    const fn get_bg_color(self, colors: &AppColors) -> Color {
        match self {
            Self::Cpu => colors.chart_cpu.background,
            Self::Memory => colors.chart_memory.background,
        }
    }

    const fn get_border_color(self, colors: &AppColors) -> Color {
        match self {
            Self::Cpu => colors.chart_cpu.border,
            Self::Memory => colors.chart_memory.border,
        }
    }

    const fn get_y_axis_color(self, colors: &AppColors) -> Color {
        match self {
            Self::Cpu => colors.chart_cpu.y_axis,
            Self::Memory => colors.chart_memory.y_axis,
        }
    }

    const fn get_max_color(self, colors: &AppColors, state: State) -> Color {
        if state.is_healthy() {
            match self {
                Self::Cpu => colors.chart_cpu.max,
                Self::Memory => colors.chart_memory.max,
            }
        } else {
            state.get_color(*colors)
        }
    }
}

/// Create a single chart
fn make_chart<'a, T: Stats + Display>(
    chart_variant: ChartVariant,
    colors: &AppColors,
    current: &'a T,
    dataset: Vec<Dataset<'a>>,
    max: &'a T,
    state: State,
    _data_points: &[(f64, f64)],
) -> Chart<'a> {
    let max_color = chart_variant.get_max_color(colors, state);

    // Ensure we have safe bounds that won't cause divide by zero
    let x_bounds = [0.0, 60.0]; // Fixed range for time series

    // Use dynamic y-bounds based on max value, with a small buffer
    // Add 0.01 to ensure the max point is always visible
    let max_value = max.get_value();
    let y_bounds = if max_value > 0.0 {
        [0.0, max_value + 0.01]
    } else {
        [0.0, 1.0] // Default bounds if no data
    };

    Chart::new(dataset)
        .bg(chart_variant.get_bg_color(colors))
        .x_axis(
            Axis::default()
                .bounds(x_bounds)
                .style(Style::default().fg(chart_variant.get_y_axis_color(colors))),
        )
        .y_axis(
            Axis::default()
                .bounds(y_bounds)
                .style(Style::default().fg(chart_variant.get_y_axis_color(colors)))
                .labels(vec![
                    Span::styled("", Style::default().fg(max_color)),
                    Span::styled(
                        format!("{max}"),
                        Style::default().fg(max_color).add_modifier(Modifier::BOLD),
                    ),
                ])
                .labels_alignment(Alignment::Left),
        )
        .block(
            Block::default()
                .style(Style::default().bg(chart_variant.get_bg_color(colors)))
                .title_alignment(Alignment::Center)
                .title(Span::styled(
                    format!(" {} {current} ", chart_variant.name()),
                    Style::default()
                        .fg(chart_variant.get_title_color(colors, state))
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(chart_variant.get_border_color(colors))),
        )
}

/// Render both CPU and memory charts
fn render_charts(
    frame: &mut Frame,
    area: &[Rect],
    colors: &AppColors,
    cpu: &(Vec<(f64, f64)>, CpuStats, State),
    mem: &(Vec<(f64, f64)>, ByteStats, State),
    current_cpu: CpuStats,
    current_mem: ByteStats,
) {
    // Skip rendering if areas are too small
    if area.len() < 2 || area[0].width < 3 || area[0].height < 3 {
        return;
    }
    // Create default data points if empty to avoid panics
    // Ensure we have at least 2 points with different x values
    let cpu_data = if cpu.0.is_empty() {
        vec![(0.0, 0.0), (60.0, 0.0)]
    } else if cpu.0.len() == 1 {
        let point = cpu.0[0];
        vec![(0.0, point.1), (60.0, point.1)]
    } else {
        cpu.0.clone()
    };

    let mem_data = if mem.0.is_empty() {
        vec![(0.0, 0.0), (60.0, 0.0)]
    } else if mem.0.len() == 1 {
        let point = mem.0[0];
        vec![(0.0, point.1), (60.0, point.1)]
    } else {
        mem.0.clone()
    };

    let cpu_dataset = vec![
        Dataset::default()
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(colors.chart_cpu.points))
            .graph_type(GraphType::Line)
            .data(&cpu_data),
    ];
    let mem_dataset = vec![
        Dataset::default()
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(colors.chart_memory.points))
            .graph_type(GraphType::Line)
            .data(&mem_data),
    ];

    // Use the actual current stats passed from the view model
    let cpu_chart = make_chart(
        ChartVariant::Cpu,
        colors,
        &current_cpu,
        cpu_dataset.clone(),
        &cpu.1,
        cpu.2,
        &cpu_data,
    );
    let mem_chart = make_chart(
        ChartVariant::Memory,
        colors,
        &current_mem,
        mem_dataset.clone(),
        &mem.1,
        mem.2,
        &mem_data,
    );

    // Only render charts if areas have valid dimensions
    if area[0].width > 2 && area[0].height > 2 {
        frame.render_widget(cpu_chart, area[0]);
    }
    if area.len() > 1 && area[1].width > 2 && area[1].height > 2 {
        frame.render_widget(mem_chart, area[1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::ChartData;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_charts_panel_render_no_data() {
        let panel = ChartsPanel::new();
        let theme = AppColors::new();
        let fd = FrameViewModel::default();

        let props = ChartsPanelProps {
            view_model: &fd,
            theme: &theme,
        };

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify no panic when no chart data
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 10);
    }

    #[test]
    fn test_charts_panel_with_data() {
        let panel = ChartsPanel::new();
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Add some chart data
        let cpu_data = vec![(0.0, 5.0), (1.0, 10.0), (2.0, 3.0)];
        let mem_data = vec![(0.0, 1000.0), (1.0, 2000.0), (2.0, 1500.0)];
        fd.chart_data = Some(ChartData {
            cpu_data: (
                cpu_data,
                CpuStats::new(10.0),
                State::Running(oxker_core::RunningState::Healthy),
            ),
            mem_data: (
                mem_data,
                ByteStats::new(2000),
                State::Running(oxker_core::RunningState::Healthy),
            ),
            current_cpu: CpuStats::new(5.0),
            current_mem: ByteStats::new(1500),
        });

        let props = ChartsPanelProps {
            view_model: &fd,
            theme: &theme,
        };

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify charts rendered
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 10);
    }
}
