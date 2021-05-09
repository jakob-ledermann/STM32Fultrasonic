use core::ops::Add;
use core::time::Duration;
use micromath::F32Ext;
use num_complex::{Complex, Complex32};

use super::complex_ext::ComplexExt;

use core::f32::consts::PI;

pub struct SinWaveGenerator {
    complex_amplitude: Complex32,
    frequency: f32,
    amplitude_offset: f32,
}

pub struct WaveGeneratorIterator<'a> {
    generator: &'a SinWaveGenerator,
    last_sample_offset: Duration,
    sample_step: Duration,
}

impl SinWaveGenerator {
    pub fn sample(&self, offset_from_start: &Duration) -> f32 {
        let time = offset_from_start.as_secs_f32();
        self.amplitude_offset
            + (self.complex_amplitude
                * Complex32::new(0f32, 2f32 * PI * self.frequency * time).exp())
            .re
    }

    pub fn wave_duration(&self) -> Duration {
        Duration::from_secs_f32(1f32 / self.frequency)
    }

    pub fn sample_with_frequency(&self, sampling_frequency: f32) -> WaveGeneratorIterator {
        let sampling_interval = 1f32 / sampling_frequency;
        let sampling_interval = Duration::from_secs_f32(sampling_interval);

        self.sample_with_interval(sampling_interval)
    }

    pub fn sample_with_interval(&self, sampling_interval: Duration) -> WaveGeneratorIterator {
        WaveGeneratorIterator {
            generator: self,
            last_sample_offset: Duration::default(),
            sample_step: sampling_interval,
        }
    }
}

impl Add<SinWaveGenerator> for SinWaveGenerator {
    type Output = ();

    fn add(self, rhs: SinWaveGenerator) -> Self::Output {
        unimplemented!()
    }
}

impl Iterator for WaveGeneratorIterator<'_> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let max_duration = self.generator.wave_duration();
        let offset_from_start = self.last_sample_offset + self.sample_step;
        if offset_from_start < max_duration {
            self.last_sample_offset = offset_from_start;
            Some(self.generator.sample(&offset_from_start))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_duration = self.generator.wave_duration() - self.last_sample_offset;
        ////    let remaining_steps = remaining_duration.div_duration_f64(self.sample_step);
        let remaining_steps = remaining_duration.as_secs_f32() / self.sample_step.as_secs_f32();

        (
            remaining_steps.floor() as usize,
            Some(remaining_steps.ceil() as usize),
        )
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use super::SinWaveGenerator;
    use alloc::vec::*;

    #[test]
    fn generates_sinus_with_1hz_and_amplitude_1() {
        let generator = SinWaveGenerator {
            frequency: 1f32,
            amplitude: 1f32,
            amplitude_offset: 0f32,
            phase_offset: 0f32,
        };

        let iter = generator.sample_with_frequency(1000f32);
        let result: Vec<f64> = iter.collect();

        assert_eq!(result.len(), 1000);

        for i in 0..1000 {
            let angle = i as f64 / 500f64 * f64::PI;

            let expected = f64::sin(angle);
            assert_eq!(expected, result[i])
        }
    }
}
