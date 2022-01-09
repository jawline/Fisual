use rand::{distributions::uniform::Uniform, rngs::SmallRng, Rng};
use variant_count::VariantCount;

#[derive(Debug, VariantCount)]
pub enum Sample {
    Sin {
        rate: f32,
        frequency: f32,
    },
    Sawtooth {
        frequency: f32,
        rate: f32,
    },
    Square {
        duty: f32,
        rate: f32,
        frequency: f32,
    },
    Triangle {
        rate: f32,
        frequency: f32,
    },
}

impl Sample {
    pub fn next(&self, clock: f32) -> f32 {
        match self {
            Sample::Sin { rate, frequency } => {
                (2.0 * std::f32::consts::PI * frequency * (clock * (1. / rate))).sin()
            }
            Sample::Sawtooth { rate, frequency } => {
                -1. + ((((clock * frequency) / rate) % 1.) * 2.)
            }
            Sample::Square {
                duty,
                rate,
                frequency,
            } => {
                if (clock * frequency / rate) % 1. > *duty {
                    1.
                } else {
                    -1.
                }
            }
            Sample::Triangle { rate, frequency } => {
                let stage = ((clock * frequency) / rate) % 4.;
                if stage <= 2. {
                    -1. + (stage * 4. % 2.)
                } else {
                    1. - (stage * 4. % 2.)
                }
            }
        }
    }

    pub fn middle_a(sample_rate: f32) -> Self {
        Sample::Sin {
            rate: sample_rate,
            frequency: 440.,
        }
    }

    pub fn middle_b(sample_rate: f32) -> Self {
        Sample::Sin {
            rate: sample_rate,
            frequency: 493.883,
        }
    }

    pub fn middle_c(sample_rate: f32) -> Self {
        Sample::Sin {
            rate: sample_rate,
            frequency: 261.63,
        }
    }

    pub fn middle_d(sample_rate: f32) -> Self {
        Sample::Sin {
            rate: sample_rate,
            frequency: 293.665,
        }
    }

    pub fn c6(sample_rate: f32) -> Self {
        Sample::Sin {
            rate: sample_rate,
            frequency: 1046.50,
        }
    }

    pub fn c8(sample_rate: f32) -> Self {
        Sample::Sin {
            rate: sample_rate,
            frequency: 4186.01,
        }
    }

    pub fn random(rng: &mut SmallRng, sample_rate: f32) -> Self {
        let random_sine = Sample::Sin {
            rate: sample_rate,
            frequency: rng.sample(Uniform::new(200., 801.)),
        };

        let random_sawtooth = Sample::Sawtooth {
            rate: sample_rate,
            frequency: rng.sample(Uniform::new(150., 600.)),
        };

        let random_square = Sample::Square {
            rate: sample_rate,
            duty: rng.sample(Uniform::new(0.3, 0.8)),
            frequency: rng.sample(Uniform::new(250., 600.)),
        };

        let random_triangle = Sample::Triangle {
            rate: sample_rate,
            frequency: rng.sample(Uniform::new(250., 500.)),
        };

        match rng.sample(Uniform::new(0, 4)) {
            0 => random_sine,
            1 => random_sawtooth,
            2 => random_square,
            3 => random_triangle,
            n => panic!("random out of range: {}", n),
        }
    }
}
