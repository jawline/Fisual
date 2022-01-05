#![feature(drain_filter)]
extern crate cpal;
extern crate num;
extern crate rand;
extern crate variant_count;

mod complex;
mod fft;
mod mixer;
mod sample;
mod ui;

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use mixer::Mixer;
use rand::{distributions::uniform::Uniform, rngs::SmallRng, Rng, SeedableRng};
use sample::Sample;
use std::error::Error;

use crate::ui::{LoopState, Ui};

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    #[clap(help = "seed argument for output")]
    seed: i64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let seed = args.seed;

    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no device found")?;
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), seed),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), seed),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), seed),
    }
}

pub fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    seed: i64,
) -> Result<(), Box<dyn Error>>
where
    T: cpal::Sample,
{
    let mut rng = SmallRng::seed_from_u64(seed as u64);
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let mut sample = Mixer::new();
    let mut continue_samples = 0.;

    let mut sample_clock = 0f32;

    let min_spawn: f32 = rng.sample(Uniform::new(0.0, 2.0));
    let max_spawn: f32 = min_spawn + rng.sample(Uniform::new(0.0, 2.0));

    // The audio thread sends samples on a channel back to the main thread for visualization
    let (sample_tx, sample_rx): (Sender<f32>, Receiver<f32>) = mpsc::channel();

    // We use a channel to communicate when the audio thread should stop generating random data
    let (finished_tx, finished_rx): (Sender<()>, Receiver<()>) = mpsc::channel();

    // This closure captures the new mixer we created and yields a function that will sample the
    // next value from it, refilling the mixer when samples end.
    let mut next_value = move || {
        sample_clock = (sample_clock + 1.0) % sample_rate;

        continue_samples = continue_samples - 1.;

        if sample_clock == 0. && continue_samples < 0. {
            continue_samples = rng.sample(Uniform::new(
                sample_rate * min_spawn,
                sample_rate * max_spawn,
            ));
            let decay_rate = rng.sample(Uniform::new(sample_rate / 1., sample_rate * 1.1));
            let decay = rng.sample(Uniform::new(0.5, 0.7));
            sample.add_sample(
                Sample::random(&mut rng, sample_rate),
                decay,
                1. / decay_rate,
            );
            // sample.add_sample(Sample::c6(sample_rate), decay, 1. / decay_rate);
        }

        let next = sample.next();

        sample_tx.send(next).unwrap();

        next
    };

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let mut finished = false;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            if let Ok(()) = finished_rx.try_recv() {
                finished = true;
            }

            if !finished {
                write_data(data, channels, &mut next_value)
            }
        },
        err_fn,
    )?;

    stream.play()?;

    let mut ui = Ui::new(1500, 1, sample_rate as usize).unwrap();
    let mut should_continue = true;

    while should_continue {
        for sample in sample_rx.try_iter().take(sample_rate as usize * 4) {
            ui.add_sample(sample);
        }

        should_continue = match ui.update().unwrap() {
            LoopState::Continue => true,
            LoopState::Exit => false,
        };

        if !should_continue {
            finished_tx.send(())?;
        }

        ui.draw().unwrap();
        thread::sleep(std::time::Duration::from_millis(2));
    }

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let value: T = cpal::Sample::from::<f32>(&next_sample());
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
