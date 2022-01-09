use crate::sample::Sample;

/// This enum keeps track of what stage in the Adsr state machine a given adsr envelope is currently
/// in.
#[derive(Debug)]
enum AdsrState {
    Attack,
    Decay,
    Sustain,
    Release,
    Finished,
}

use AdsrState::*;

/// An Adsr envelope for synthesized sounds. This structure models linear ramp up of a sound to it's peak, then
/// a linear decrease to a sustain level. The envelope will hold the sample amplitude at the sustain level for a
/// fixed amount of time. Once the sustain period has elapsed the sound will linearly decrease from the sustain level
/// to zero amplitude.
#[derive(Debug)]
pub struct Adsr {
    // The current state of the adsr envelope
    current_state: AdsrState,

    // The time spent in the current state
    time_in_state: f32,

    // How quickly (in seconds) should the sound hit it's peak.
    attack: f32,

    // What should the amplitude of the sample be multiplied by at the peak.
    peak_scalar: f32,

    // How quickly (in seconds) should the sound go from it's peak to it's sustain level.
    decay: f32,

    // How long (in seconds) should the sound stay at it's sustain level
    sustain: f32,

    // What should the amplitude of the sample be multiplied by at it's sustain level
    sustain_scalar: f32,

    // How long (in seconds) should the sound take to go from it's sustain amplitude to zero
    // amplitude.
    release: f32,

    // The sample to modify
    sample: Sample,

    // The sample rate of the output stream
    sample_rate: f32,
}

impl Adsr {
    pub fn new(
        sample: Sample,
        sample_rate: f32,
        attack: f32,
        peak_scalar: f32,
        decay: f32,
        sustain: f32,
        sustain_scalar: f32,
        release: f32,
    ) -> Self {
        Adsr {
            current_state: AdsrState::Attack,
            time_in_state: 0.,
            sample,
            sample_rate,
            attack,
            peak_scalar,
            decay,
            sustain,
            sustain_scalar,
            release,
        }
    }

    /// Step the adsr envelope forward by one sample, update the state and return the amplitude
    /// of the envelope at the adjusted time.
    fn step_state(
        &mut self,
        clock: f32,
        start_scalar: f32,
        end_scalar: f32,
        max_time: f32,
        next_state: AdsrState,
    ) -> f32 {
        let sampled = self.sample.next(clock);
        self.time_in_state += 1. / self.sample_rate;
        if self.time_in_state > max_time {
            self.current_state = next_state;
            sampled * end_scalar
        } else {

            let low_sample = start_scalar * sampled;
            let high_sample = end_scalar * sampled;

            low_sample + ((high_sample - low_sample) * (self.time_in_state / max_time))
        }
    }

    /// Return the amplitude of the next sample for this adsr envelope.
    pub fn next(&mut self, clock: f32) -> f32 {
        match self.current_state {
            Attack => self.step_state(clock, 0., self.peak_scalar, self.attack, AdsrState::Decay),
            Decay => self.step_state(
                clock,
                self.peak_scalar,
                self.sustain_scalar,
                self.decay,
                AdsrState::Sustain,
            ),
            Sustain => self.step_state(
                clock,
                self.sustain_scalar,
                self.sustain_scalar,
                self.sustain,
                AdsrState::Release,
            ),
            Release => self.step_state(
                clock,
                self.sustain_scalar,
                0.,
                self.release,
                AdsrState::Finished,
            ),
            Finished => 0.,
        }
    }

    /// Returns true when this envelope is finished, at which point next will return zero forever.
    pub fn finished(&self) -> bool {
        match self.current_state {
            Attack | Decay | Sustain | Release => false,
            Finished => true
        }
    }
}
