use crate::sample::Sample;

pub struct Chunk {
    pub sample: Sample,
    pub decay: f32,
    pub decay_rate: f32,
}

pub struct Mixer {
    chunks: Vec<Chunk>,
}

impl Mixer {

    pub fn new() -> Self {
        Mixer {
            chunks: Vec::new()
        }
    }

    pub fn add_sample(&mut self, sample: Sample, decay: f32, decay_rate: f32) {
        self.chunks.push(Chunk {
            sample,
            decay,
            decay_rate,
        });
    }

    pub fn next(&mut self, count: f32) -> f32 {
        let mut sampled = 0.;

        self.chunks.drain_filter(|sample| {
            sampled += sample.sample.next(count) * sample.decay;
            sample.decay -= sample.decay_rate;
            sample.decay < 0.
        });

        sampled
    }

}
