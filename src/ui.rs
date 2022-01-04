use crate::complex::Complex;
use std::error::Error;
use std::io::{stdout, Bytes, Read, Stdout, Write};
use termion::{
    async_stdin,
    raw::{IntoRawMode, RawTerminal},
    AsyncReader,
};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Terminal,
};

pub struct Ui {
    samples: Vec<(f64, f64)>,
    sample_window: usize,
    total_samples: usize,
    sample_rate: usize,
    terminal: Terminal<TermionBackend<RawTerminal<Stdout>>>,
    stdin: Bytes<AsyncReader>,
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

    // Pad a frame to the nearest power of 2 of entries for the fast-fourier transform
    fn fft_round_to_nearest_pow2(mut frame: Vec<Complex<f64>>) -> Vec<Complex<f64>> {
        let current_len = frame.len();
        let new_len = current_len.next_power_of_two();
        let new_entries = new_len - current_len;
        for _ in 0..new_entries {
            frame.push(Complex::real(0.));
        }
        frame
    }

    fn frequency_in_hz_of_sample(
        sample_index: usize,
        num_samples: usize,
        sample_rate: usize,
    ) -> f64 {
        let sample_index = sample_index as f64;
        let num_samples = num_samples as f64;
        let sample_rate = sample_rate as f64;
        sample_rate * (sample_index / num_samples)
    }

    fn fft_frame(&self, sample_window: usize) -> (f64, f64, Vec<(f64, f64)>) {
        // TODO: Pre-allocate memory in self on sample size changes and modify fast-fourier
        // transform to be in place. Performance should stop sucking afterwards.
        // (Maybe subsample larger windows)
        use crate::fft::do_fft;
        let (_first_time, _last_time, frame) = self.frame(sample_window);
        let frame: Vec<Complex<f64>> = frame.iter().map(|(_, x)| Complex::real(*x)).collect();
        let mut frame = Self::fft_round_to_nearest_pow2(frame);
        let frequency_samples = frame.len();
        do_fft(&mut frame, false).expect("do_fft failed. probably not a power of two");

        let frequency_samples = frame
            .iter()
            .enumerate()
            .map(|(sample_index, frequency_state)| {
                (
                    Self::frequency_in_hz_of_sample(
                        sample_index,
                        frequency_samples,
                        self.sample_rate,
                    ),
                    frequency_state.real,
                )
            });

        let zero_zero = [(0., 0.)].into_iter();

        let frame: Vec<(f64, f64)> = zero_zero.chain(frequency_samples).collect();
        (0., self.sample_rate as f64, frame)
    }

    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
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
                Ok(b'q') => std::process::exit(0),
                _ => {}
            };
        }

        Ok(())
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
                            format!("{}{}", first_time, x_unit),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{}{}", last_time, x_unit),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .title("amplitude")
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
        let (first_freq, last_freq, fft_frame) = self.fft_frame(self.sample_window);

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
                .constraints([Constraint::Length(15), Constraint::Length(15)].as_ref())
                .margin(1)
                .split(f.size());

            let mut render_exists = |widget: Option<Chart>, chunk: usize| match widget {
                Some(widget) => f.render_widget(widget, chunks[chunk]),
                None => (),
            };

            render_exists(freq_widget, 0);
            render_exists(fft_widget, 1);
        })?;
        Ok(())
    }
}
