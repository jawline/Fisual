use crate::complex::Complex;
use crate::fft::do_fft;
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

    fn fft_round_to(mut frame: Vec<Complex<f64>>, new_len: usize) -> Vec<Complex<f64>> {
        let current_len = frame.len();

        if frame.len() >= new_len {
            panic!("too large");
        }

        let new_entries = new_len - current_len;
        for _ in 0..new_entries {
            frame.push(Complex::real(0.));
        }
        frame
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

        // We run our fft on the samples returned by frame using a specific number of sound
        // samples.
        let (_first_time, _last_time, frame) = self.frame(sample_window);
        let frame: Vec<Complex<f64>> = frame.iter().map(|(_, x)| Complex::real(*x)).collect();

        // We pad the fft frame to 2^16 elements which has the effect of interpolating values in
        // the fft.
        let mut frame = Self::fft_round_to(frame, 65536);
        do_fft(&mut frame, false).expect("do_fft failed. probably not a power of two");

        // For real numbers, the fft is symmetric and we get the amplitude by summing the
        // magnitudes of X[k] and X[-k] for 0 <= k < (len(X) / 2)
        let datapoints = frame.len();
        let half_datapoints = frame.len() / 2;

        let first_half = frame.iter().take(half_datapoints);
        let second_half = frame
            .iter()
            .skip(half_datapoints)
            .take(half_datapoints)
            .rev();

        let frequency_samples = first_half.zip(second_half).enumerate().map(
            |(sample_index, (first_half_freq, second_half_freq))| {
                (
                    Self::frequency_in_hz_of_sample(sample_index, datapoints, self.sample_rate),
                    (first_half_freq.magnitude() + second_half_freq.magnitude())
                        / self.sample_window as f64,
                )
            },
        );

        // Add a zero point so tui prints a flat line before the first data point
        // rather than empty space.
        let zero_zero = [(0., 0.)].into_iter();
        let frame: Vec<(f64, f64)> = zero_zero.chain(frequency_samples).collect();

        (0., frame.last().unwrap().0, frame)
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
