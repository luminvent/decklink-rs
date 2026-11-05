extern crate decklink;
#[macro_use]
extern crate text_io;

use decklink::device::output::{
    DeckLinkVideoOutputCallback, DecklinkOutputDevice, DecklinkOutputFrameCompletionResult,
    DecklinkVideoOutputFlags,
};
use decklink::device::DecklinkDeviceDisplayModes;
use decklink::device::{get_devices, DecklinkDevice};
use decklink::display_mode::DecklinkDisplayMode;
use decklink::frame::{
    DecklinkFrameFlags, DecklinkPixelFormat, DecklinkVideoFrame, DecklinkVideoMutableFrame,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

// How many frames to queue before starting playback
const PREFILL_FRAMES: u32 = 5;
// Refill the queue when it drops below this threshold
const MIN_BUFFERED_FRAMES: u32 = 3;

fn select_output_and_format() -> Option<(DecklinkDevice, DecklinkOutputDevice, DecklinkDisplayMode)>
{
    let device = {
        let mut devices = get_devices().expect("list devices failed");
        println!("Found {} devices", devices.len());
        for i in 0..devices.len() {
            println!(
                "{}: {}",
                i,
                devices[i]
                    .display_name()
                    .unwrap_or_else(|| "Unknown".to_string())
            );
        }

        let index: usize = text_io::read!();
        if index >= devices.len() {
            println!("Invalid device index");
            return None;
        }
        devices.swap_remove(index)
    };

    println!(
        "Selected device: {}\n",
        device
            .display_name()
            .unwrap_or_else(|| "Unknown".to_string())
    );

    let output = match device.output() {
        None => {
            println!("Failed to create device output");
            return None;
        }
        Some(o) => o,
    };

    let mode = {
        let mut supported_modes = output
            .display_modes()
            .expect("Failed to list display modes");
        for i in 0..supported_modes.len() {
            println!(
                "{}: {}",
                i,
                supported_modes[i]
                    .name()
                    .unwrap_or_else(|| "Unknown".to_string())
            );
        }

        let index: usize = read!();
        if index >= supported_modes.len() {
            println!("Invalid mode index");
            return None;
        }
        supported_modes.swap_remove(index)
    };

    Some((device, output, mode))
}

struct ScheduledPlaybackCallback {
    stopped: Arc<AtomicBool>,
}

impl DeckLinkVideoOutputCallback for ScheduledPlaybackCallback {
    fn schedule_frame_completed_callback(
        &self,
        _frame: Option<DecklinkVideoFrame>,
        result: DecklinkOutputFrameCompletionResult,
    ) -> bool {
        if result != DecklinkOutputFrameCompletionResult::Completed {
            eprintln!("Frame not completed as expected: {:?}", result);
        }
        true
    }

    fn playback_stopped(&self) -> bool {
        self.stopped.store(true, Ordering::Relaxed);
        true
    }
}

// Generates a solid-colour frame that cycles through hues as frame_index increments.
fn build_frame(width: usize, height: usize, frame_index: usize) -> DecklinkVideoMutableFrame {
    let mut frame = DecklinkVideoMutableFrame::create(
        width,
        height,
        width * 4,
        DecklinkPixelFormat::Format8BitBGRA,
        DecklinkFrameFlags::empty(),
    );

    let t = frame_index as u8;
    let mut bytes = vec![0u8; width * height * 4];
    for i in (0..bytes.len()).step_by(4) {
        bytes[i] = t; // B
        bytes[i + 1] = t.wrapping_add(85); // G
        bytes[i + 2] = t.wrapping_add(170); // R
    }

    frame.copy_bytes(&bytes).expect("Failed to set frame bytes");
    frame
}

fn main() {
    let Some((_device, output, mode)) = select_output_and_format() else {
        return;
    };

    let (frame_duration, timescale) = mode
        .framerate()
        .expect("Failed to get frame rate from display mode");

    println!(
        "Mode: {} {}x{}, timescale={} frame_duration={}",
        mode.name().unwrap_or_else(|| "Unknown".to_string()),
        mode.width(),
        mode.height(),
        timescale,
        frame_duration,
    );

    let mut video_output = output
        .enable_video_output_scheduled(mode.mode(), DecklinkVideoOutputFlags::empty(), timescale)
        .expect("Failed to enable scheduled video output");

    let stopped = Arc::new(AtomicBool::new(false));
    let callback = Arc::new(ScheduledPlaybackCallback {
        stopped: stopped.clone(),
    });
    video_output
        .set_callback(Some(callback))
        .expect("Failed to set callback");

    // Pre-fill the buffer before starting playback to avoid underruns at startup
    let mut next_display_time: i64 = 0;
    let mut frame_index: usize = 0;
    for _ in 0..PREFILL_FRAMES {
        println!("Scheduling frame {}", frame_index);
        let frame = build_frame(mode.width(), mode.height(), frame_index);
        video_output
            .schedule_frame_copy(&frame, next_display_time, frame_duration)
            .expect("Failed to pre-fill frame");
        next_display_time += frame_duration;
        frame_index += 1;
    }

    video_output
        .start_playback(0, 1.0)
        .expect("Failed to start playback");

    println!("Playing. Press enter to stop.");

    // Wait for enter on a background thread so the main loop stays free
    let quit = Arc::new(AtomicBool::new(false));
    let quit_clone = quit.clone();
    std::thread::spawn(move || {
        let _: String = read!();
        quit_clone.store(true, Ordering::Relaxed);
    });

    loop {
        if quit.load(Ordering::Relaxed) {
            break;
        }

        // Keep the buffer topped up so the hardware never runs dry
        while video_output.buffered_video_frame_count().unwrap_or(0) < MIN_BUFFERED_FRAMES {
            let frame = build_frame(mode.width(), mode.height(), frame_index);
            video_output
                .schedule_frame_copy(&frame, next_display_time, frame_duration)
                .expect("Failed to schedule frame");
            next_display_time += frame_duration;
            frame_index += 1;
        }

        sleep(Duration::from_millis(10));
    }

    let actual_stop = video_output
        .stop_playback(next_display_time)
        .unwrap_or(next_display_time);
    println!("Stopped at display time {}", actual_stop);
}
