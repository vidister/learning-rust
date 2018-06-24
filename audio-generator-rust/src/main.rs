#![feature(fn_traits, unboxed_closures)]
#![allow(dead_code)]

extern crate byteorder;
extern crate num;

use std::fs::OpenOptions;
use std::io::Result;
use std::path::Path;
use std::f64::consts::PI;
use std::mem::size_of;
use std::iter::Iterator;

use num::Float;
use num::traits::{Bounded, FromPrimitive, Num};

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};



/// Invokes the waveform function `f` at time `t` to return the amplitude at that time.
pub fn generate<F>(t: f64, f: &F) -> f64
where
    F: Fn(f64) -> f64,
{
    f.call((t,))
}

pub fn make_samples<F>(length: f64, sample_rate: usize, waveform: F) -> Vec<f64>
where
    F: Fn(f64) -> f64,
{
    let num_samples = (sample_rate as f64 * length).floor() as usize;
    let mut samples: Vec<f64> = Vec::with_capacity(num_samples);

    for i in 0usize..num_samples {
        let t = i as f64 / sample_rate as f64;
        samples.push(generate(t, &waveform));
    }

    samples
}



pub fn quantize<T>(input: f64) -> T
where
    T: Num + FromPrimitive + Bounded,
{
    let quantization_levels = 2.0.powf(size_of::<T>() as f64 * 8.0) - 1.0;
    T::from_f64(input * (quantization_levels / 2.0)).expect("failed to quantize to given type")
}

pub fn quantize_samples<T>(input: &[f64]) -> Vec<T>
where
    T: Num + FromPrimitive + Bounded,
{
    input.iter().map(|s| quantize::<T>(*s)).collect()
}






#[derive(Clone, Copy)]
pub struct SineWave(pub f64);

impl Fn<(f64,)> for SineWave {
    extern "rust-call" fn call(&self, (t,): (f64,)) -> f64 {
        let SineWave(frequency) = *self;
        (t * frequency * 2.0 * PI).sin()
    }
}
impl FnMut<(f64,)> for SineWave {
    extern "rust-call" fn call_mut(&mut self, (t,): (f64,)) -> f64 {
        self.call((t,))
    }
}
impl FnOnce<(f64,)> for SineWave {
    type Output = f64;
    extern "rust-call" fn call_once(self, (t,): (f64,)) -> f64 {
        self.call((t,))
    }
}


// See: https://ccrma.stanford.edu/courses/422/projects/WaveFormat/
pub fn write_wav(filename: &str, sample_rate: usize, samples: &[i16]) -> Result<()> {
    let path = Path::new(filename);
    let mut f = try!(
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&path)
    );

    // Some WAV header fields
    let channels = 1;
    let bit_depth = 16;
    let subchunk_2_size = samples.len() * channels * bit_depth / 8;
    let chunk_size = 36 + subchunk_2_size as i32;
    let byte_rate = (sample_rate * channels * bit_depth / 8) as i32;
    let block_align = (channels * bit_depth / 8) as i16;

    f.write_i32::<BigEndian>(0x5249_4646)?; // ChunkID, RIFF
    f.write_i32::<LittleEndian>(chunk_size)?; // ChunkSize
    f.write_i32::<BigEndian>(0x5741_5645)?; // Format, WAVE

    f.write_i32::<BigEndian>(0x666d_7420)?; // Subchunk1ID, fmt
    f.write_i32::<LittleEndian>(16)?; // Subchunk1Size, 16 for PCM
    f.write_i16::<LittleEndian>(1)?; // AudioFormat, PCM = 1 (linear quantization)
    f.write_i16::<LittleEndian>(channels as i16)?; // NumChannels
    f.write_i32::<LittleEndian>(sample_rate as i32)?; // SampleRate
    f.write_i32::<LittleEndian>(byte_rate)?; // ByteRate
    f.write_i16::<LittleEndian>(block_align)?; // BlockAlign
    f.write_i16::<LittleEndian>(bit_depth as i16)?; // BitsPerSample

    f.write_i32::<BigEndian>(0x6461_7461)?; // Subchunk2ID, data
    f.write_i32::<LittleEndian>(subchunk_2_size as i32)?; // Subchunk2Size, number of bytes in the data

    for sample in samples {
        f.write_i16::<LittleEndian>(*sample)?
    }

    Ok(())
}

fn main() {

    write_wav(
        "out/sin.wav",
        44_100,
        &quantize_samples::<i16>(&make_samples(1.0, 44_100, SineWave(1440.0))),
    ).expect("failed");

}
