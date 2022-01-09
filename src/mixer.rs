use crate::adsr::Adsr;

/// A mixer chunk stores the sample being played and the number of times it has been
/// sampled (it's clock).
pub struct Chunk {
    pub sample: Adsr,
    pub samples: f32,
}

/// The mixer combines a set of playing samples wrapped in adsr envelopes and mixes them together, removing samples once they are finished.
pub struct Mixer {
    chunks: Vec<Chunk>,
}

impl Mixer {
    pub fn new() -> Self {
        Mixer { chunks: Vec::new() }
    }

    pub fn add_sample(&mut self, sample: Adsr) {
        self.chunks.push(Chunk {
            sample,
            samples: 0.,
        });
    }

    pub fn next(&mut self) -> f32 {
        let mut sampled = 0.;

        self.chunks.drain_filter(|sample| {
            sampled += sample.sample.next(sample.samples) ;
            sample.samples += 1.;
            sample.sample.finished()
        });

        f32::max(f32::min(sampled, 1.0), -1.)
    }
}
