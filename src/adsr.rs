use crate::sample::Sample;

enum AdsrState {
    Attack,
    Decay,
    Sustain,
    Release,
}

/// An Adsr envelope for synthesized sounds. This structure odels linear ramp up of a sound to it's peak, then
/// a linear decrease to a sustain level. Once the sound is finished the sound will linearly decrease
/// from the sustain level to zero amplitude.
pub struct Adsr {
    // The current state of the adsr envelope
    current_state: AdsrState,

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
