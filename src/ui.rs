use std::error::Error;
use std::io::{Read, Bytes, Stdout, stdout, Write};
use termion::{async_stdin, AsyncReader, raw::{RawTerminal, IntoRawMode}};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame, Terminal,
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
    pub fn new(sample_window: usize, seconds_to_record : usize, sample_rate : usize) -> Result<Self, Box<dyn Error>> {

        let mut stdout = stdout().into_raw_mode()?;
        write!(stdout, "{}", termion::clear::All).unwrap();

        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut stdin = async_stdin().bytes();

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
        self.samples[self.total_samples % capacity] = (self.total_samples as f64 / self.sample_rate as f64, sample as f64);
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

    fn fft_frame(&self, sample_window: usize) -> (f64, f64, Vec<(f64, f64)>) {
        use crate::fft::do_fft;
        use crate::complex::Complex;
        let (first_time, last_time, frame) = self.frame(sample_window);
        let frame : Vec<Complex> = frame.iter().map(|(_, x)| Complex::real(*x)).collect();
        let frame = do_fft(&frame, false);
        let frame = frame.iter().enumerate().map(|(i, x)| (i as f64, x.real)).collect();
        (0., sample_window as f64, frame)
    }

    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {

        while let Some(item) = self.stdin.next() {
            match item {
              Ok(b'+') => {
                  self.sample_window += 200;
              },
              Ok(b'-') => {
                  if self.sample_window > 400 {
                      self.sample_window -= 200;
                  } else {
                      self.sample_window = 1;
                  }
              },
              Ok(b'q') => std::process::exit(0),
              _ => {}
            };
        }

        Ok(())
    }

    pub fn draw(&mut self) -> Result<(), Box<dyn Error>> {

        let (first_time, last_time, frame) = self.fft_frame(self.sample_window);

        if frame.len() == 0 {
            return Ok(());
        }

        self.terminal.draw(|f| {
          let datasets = vec![Dataset::default()
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(Color::Green))
        .graph_type(GraphType::Line)
        .data(&frame[..])];
        let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(Span::styled(
                    format!("samples: {}", frame.len()),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("time (s)")
                .style(Style::default().fg(Color::Gray))
                .bounds([first_time, last_time])
                .labels(vec![
                    Span::styled(format!("{:.2}s", last_time), Style::default().add_modifier(Modifier::BOLD)),
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
        );
    f.render_widget(chart, f.size());
        })?;
        Ok(())
    }

}
