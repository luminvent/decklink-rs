#[macro_use]
extern crate text_io;

use blackmagic_decklink::device::output::{
    DecklinkAudioOutputStreamType, DecklinkAudioSampleRate, DecklinkAudioSampleType,
    DecklinkOutputDevice,
};
use blackmagic_decklink::device::{DecklinkDevice, get_devices};
use std::f64::consts::PI;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: u32 = 2;
const TONE_HZ: f64 = 440.0;
const AMPLITUDE: f64 = i16::MAX as f64 * 0.5;
// ~33 ms of audio per write call
const CHUNK_FRAMES: u32 = 1_600;

fn select_device() -> Option<(DecklinkDevice, DecklinkOutputDevice)> {
    let mut devices = get_devices().expect("list devices failed");
    println!("Found {} devices", devices.len());

    for (index, decklink_device) in devices.iter().enumerate() {
        println!(
            "{}: {}",
            index,
            decklink_device
                .display_name()
                .unwrap_or_else(|| "Unknown".to_string())
        );
    }

    let index: usize = text_io::read!();
    if index >= devices.len() {
        println!("Invalid device index");
        return None;
    }

    let device = devices.swap_remove(index);
    println!(
        "Selected device: {}\n",
        device
            .display_name()
            .unwrap_or_else(|| "Unknown".to_string())
    );

    let output = match device.output() {
        None => {
            println!("Failed to open device output");
            return None;
        }
        Some(o) => o,
    };

    Some((device, output))
}

// Returns interleaved stereo i16 samples for one chunk.
// `phase` is a normalised phase accumulator [0, 1) advanced in place.
fn sine_chunk(phase: &mut f64) -> Vec<i16> {
    let mut buf = Vec::with_capacity((CHUNK_FRAMES * CHANNELS) as usize);
    let phase_step = TONE_HZ / SAMPLE_RATE as f64;
    for _ in 0..CHUNK_FRAMES {
        let sample = ((2.0 * PI * *phase).sin() * AMPLITUDE) as i16;
        for _ in 0..CHANNELS {
            buf.push(sample);
        }
        *phase += phase_step;
        if *phase >= 1.0 {
            *phase -= 1.0;
        }
    }
    buf
}

fn main() {
    let Some((_device, output)) = select_device() else {
        return;
    };

    let audio = output
        .enable_audio_output(
            DecklinkAudioSampleRate::Rate48kHz,
            DecklinkAudioSampleType::Int16,
            CHANNELS,
            DecklinkAudioOutputStreamType::Continuous,
        )
        .expect("Failed to enable audio output");

    println!("Playing 440 Hz tone at 48 kHz stereo. Press enter to stop.");

    let quit = Arc::new(AtomicBool::new(false));
    let quit_clone = quit.clone();
    std::thread::spawn(move || {
        let _: String = read!();
        quit_clone.store(true, Ordering::Relaxed);
    });

    let start = Instant::now();
    let mut phase = 0.0f64;
    let mut pending: Vec<i16> = Vec::new();
    let mut total_written: u64 = 0;
    loop {
        if quit.load(Ordering::Relaxed) {
            break;
        }

        if pending.is_empty() {
            pending = sine_chunk(&mut phase);
        }
        let frames = pending.len() as u32 / CHANNELS;
        let written = audio
            .write_audio_samples_sync(&pending, frames)
            .expect("Failed to write audio samples");
        total_written += written as u64;

        // Keep the samples the device did not accept, so the tone stays continuous
        pending.drain(..(written * CHANNELS) as usize);

        // Back off briefly if the internal buffer was full to avoid spinning
        if written < frames {
            sleep(Duration::from_millis(5));
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "Wrote {} sample frames in {:.2}s ({:.0} frames/s, expected {})",
        total_written,
        elapsed,
        total_written as f64 / elapsed,
        SAMPLE_RATE
    );
}
