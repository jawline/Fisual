use crate::fft::RealFft;
use std::error::Error;
use std::io::{stdout, Bytes, Read, Stdout, Write};
use termion::{
    async_stdin,
    raw::{IntoRawMode, RawTerminal},
    AsyncReader,
};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Widget, Wrap},
    Frame, Terminal,
};

pub enum LoopState {
    Continue,
    Exit,
}

pub struct Ui {
    samples: Vec<(f64, f64)>,
    sample_window: usize,
    total_samples: usize,
    sample_rate: usize,
    terminal: Terminal<TermionBackend<RawTerminal<Stdout>>>,
    stdin: Bytes<AsyncReader>,
    fft_buffer: RealFft<f64>,
}

impl Ui {
    pub fn new(
        sample_window: usize,
        seconds_to_record: usize,
        sample_rate: usize,
    ) -> Result<Self, Box<dyn Error>> {
        let mut stdout = stdout().into_raw_mode()?;
        write!(stdout, "{}", termion::clear::All).unwrap();

        let backend = TermionBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let stdin = async_stdin().bytes();

        Ok(Ui {
            sample_window,
            samples: vec![(0., 0.); sample_rate * seconds_to_record],
            total_samples: 0,
            sample_rate,
            terminal,
            stdin,
            fft_buffer: RealFft::new(65536)?,
        })
    }

    pub fn add_sample(&mut self, sample: f32) {
        let capacity = self.samples.capacity();
        self.samples[self.total_samples % capacity] = (
            self.total_samples as f64 / self.sample_rate as f64,
            sample as f64,
        );
        //println!("{:?}", self.samples[self.total_samples % capacity]);
        self.total_samples += 1;
    }

    fn frame(&self, sample_window: usize) -> (f64, f64, Vec<(f64, f64)>) {
        if self.total_samples < sample_window {
            return (0., 0., Vec::new());
        }

        let first = (self.total_samples - sample_window) % self.sample_rate;

        let mut frame = Vec::new();

        for i in 0..sample_window {
            frame.push(self.samples[(first + i) % self.sample_rate]);
        }

        let (first_time, _) = frame[0];
        let (last_time, _) = frame[frame.len() - 1];

        (first_time, last_time, frame)
    }

    fn fft_frame(
        &mut self,
        sample_window: usize,
    ) -> Result<(f64, f64, Vec<(f64, f64)>), Box<dyn Error>> {
        // TODO: fft could be modified to take an inter of amplitudes to avoid
        // the overhead of cloning twice
        let (_first_time, _last_time, frame) = self.frame(sample_window);
        let frame_amplitudes: Vec<f64> = frame.iter().map(|(x, y)| *y).collect();

        // Add a zero point so tui prints a flat line before the first data point
        // rather than empty space.
        let result_frequencies: Vec<(f64, f64)> = self
            .fft_buffer
            .run(&frame_amplitudes)?
            .iter()
            .cloned()
            .collect();
        Ok((0., result_frequencies.last().unwrap().0, result_frequencies))
    }

    pub fn update(&mut self) -> Result<LoopState, Box<dyn Error>> {
        while let Some(item) = self.stdin.next() {
            match item {
                Ok(b'+') => {
                    self.sample_window += 50;
                }
                Ok(b'-') => {
                    if self.sample_window > 50 {
                        self.sample_window -= 50;
                    }
                }
                Ok(b'q') => return Ok(LoopState::Exit),
                _ => {}
            };
        }

        Ok(LoopState::Continue)
    }

    fn draw_widget<W: Widget + Sized, T: Backend>(
        f: &mut Frame<'_, T>,
        widget: Option<W>,
        chunk: Rect,
    ) {
        match widget {
            Some(widget) => f.render_widget(widget, chunk),
            None => (),
        }
    }

    pub fn chart<'a>(
        title: &'a str,
        x_title: &'a str,
        x_unit: &'a str,
        (first_time, last_time): (f64, f64),
        frame: &'a [(f64, f64)],
    ) -> Chart<'a> {
        let datasets = vec![Dataset::default()
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Green))
            .graph_type(GraphType::Line)
            .data(frame)];
        Chart::new(datasets)
            .block(
                Block::default()
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .title(x_title)
                    .style(Style::default().fg(Color::Gray))
                    .bounds([first_time, last_time])
                    .labels(vec![
                        Span::styled(
                            format!("{:.2}{}", first_time, x_unit),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{:.2}{}", last_time, x_unit),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .bounds([-1., 1.])
                    .labels(vec![
                        Span::styled("-1.0", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw("0."),
                        Span::styled("1.0", Style::default().add_modifier(Modifier::BOLD)),
                    ]),
            )
    }

    pub fn draw(&mut self) -> Result<(), Box<dyn Error>> {
        let (first_time, last_time, frame) = self.frame(self.sample_window);
        let (first_freq, last_freq, fft_frame) = self.fft_frame(self.sample_window)?;

        self.terminal.draw(|f| {
            let freq_widget = {
                if frame.len() == 0 {
                    None
                } else {
                    Some(Self::chart(
                        "waveform",
                        "time (s)",
                        "s",
                        (first_time, last_time),
                        &frame[..],
                    ))
                }
            };

            let fft_widget = {
                if frame.len() == 0 {
                    None
                } else {
                    Some(Self::chart(
                        "frequency spectrum",
                        "frequency (hz)",
                        "hz",
                        (first_freq, last_freq),
                        &fft_frame[..],
                    ))
                }
            };

            let chunks = Layout::default()
                .constraints(
                    [
                        Constraint::Length(4),
                        Constraint::Length(15),
                        Constraint::Length(15),
                    ]
                    .as_ref(),
                )
                .margin(1)
                .split(f.size());

            let intro_text = Some(
                Paragraph::new(format!("{} samples visualized", self.sample_window))
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::White).bg(Color::Black))
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: true }),
            );

            Self::draw_widget(f, intro_text, chunks[0]);
            Self::draw_widget(f, fft_widget, chunks[1]);
            Self::draw_widget(f, freq_widget, chunks[2]);
        })?;
        Ok(())
    }
}
