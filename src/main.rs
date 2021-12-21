extern crate rand;
extern crate cpal;

use rand::{rngs::SmallRng, distributions::uniform::{Uniform}, Rng, SeedableRng};
use std::error::Error;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use clap::Parser;

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

#[derive(Debug)]
enum Sample {
    Sin { rate : f32, amplitude : f32 },
    Sawtooth { pitch : f32, rate : f32 },
}

impl Sample {
    fn next(&self, clock : f32) -> f32 {
      match self {
          Sample::Sin { rate, amplitude } => {
            (clock * amplitude * 2.0 * std::f32::consts::PI / (rate)).sin()
          },
          Sample::Sawtooth { rate, pitch } => {
              -1. + (((clock / rate) * pitch % 1.) * 2.)
          }
      }
    }

    fn random(rng: &mut SmallRng, sample_rate: f32) -> Self {
        let random_sine = Sample::Sin { rate: sample_rate, amplitude: rng.sample(Uniform::new(290., 360.)) };
        let random_sawtooth = Sample::Sawtooth { rate: sample_rate, pitch: rng.sample(Uniform::new(100., 600.)) };
        match rng.sample(Uniform::new(0,1)) {
            0 => random_sine,
            1 => random_sawtooth,
            _ => panic!("random out of range"),
        }
    }
}

pub fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig, seed : i64) -> Result<(), Box<dyn Error>>
where
    T: cpal::Sample,
{
    let mut rng = SmallRng::seed_from_u64(seed as u64);
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let mut sample = Sample::Sawtooth { rate: sample_rate, pitch: 300. };
    let mut continue_samples = 0.;

    let mut sample_clock = 0f32;
    let mut next_value = move || {

        sample_clock = (sample_clock + 1.0) % sample_rate;

        continue_samples = continue_samples - 1.;
        if sample_clock == 0. && continue_samples < 0. {
            continue_samples = rng.sample(Uniform::new((sample_rate / 2.), sample_rate * 2.));
            sample = Sample::random(&mut rng, sample_rate);
        }
        let next = sample.next(sample_clock);
        println!("{}", next);
        next
    };

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut next_value)
        },
        err_fn,
    )?;
    stream.play()?;

    std::thread::sleep(std::time::Duration::from_millis(30000));

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
