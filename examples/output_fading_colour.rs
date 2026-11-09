#[macro_use]
extern crate text_io;

use blackmagic_decklink::device::DecklinkDeviceDisplayModes;
use blackmagic_decklink::device::output::DecklinkVideoOutputFlags;
use blackmagic_decklink::device::output::{
    DeckLinkVideoOutputCallback, DecklinkOutputDevice, DecklinkOutputFrameCompletionResult,
};
use blackmagic_decklink::device::{DecklinkDevice, get_devices};
use blackmagic_decklink::display_mode::DecklinkDisplayMode;
use blackmagic_decklink::frame::{
    DecklinkFrameFlags, DecklinkPixelFormat, DecklinkVideoFrame, DecklinkVideoMutableFrame,
};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

fn select_output_and_format() -> Option<(DecklinkDevice, DecklinkOutputDevice, DecklinkDisplayMode)>
{
    let device = {
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

        devices.swap_remove(index)
    };

    println!(
        "Selected device: {}\n",
        device
            .display_name()
            .unwrap_or_else(|| "Unknown".to_string())
    );

    if device.output().is_none() {
        println!("Failed to create device output");
    }

    let output = device.output()?;

    let mode = {
        let mut decklink_display_modes = output
            .display_modes()
            .expect("Failed to list display modes");

        for (index, decklink_display_mode) in decklink_display_modes.iter().enumerate() {
            println!(
                "{}: {}",
                index,
                decklink_display_mode
                    .name()
                    .unwrap_or_else(|| "Unknown".to_string())
            );
        }

        let index: usize = read!();
        if index >= decklink_display_modes.len() {
            println!("Invalid mode index");
            return None;
        }

        decklink_display_modes.swap_remove(index)
    };

    Some((device, output, mode))
}

struct CompletionCallback {}
impl DeckLinkVideoOutputCallback for CompletionCallback {
    fn schedule_frame_completed_callback(
        &self,
        _frame: Option<DecklinkVideoFrame>,
        _result: DecklinkOutputFrameCompletionResult,
    ) -> bool {
        println!("Frame completed");
        sleep(Duration::from_millis(100));
        true
    }

    fn playback_stopped(&self) -> bool {
        println!("Playback stopped");
        true
    }
}

fn main() {
    if let Some((_device, output, mode)) = select_output_and_format() {
        let mut frame = Box::new(DecklinkVideoMutableFrame::create(
            mode.width(),
            mode.height(),
            mode.width() * 4,
            DecklinkPixelFormat::Format8BitBGRA,
            DecklinkFrameFlags::empty(),
        ));

        let bytes = vec![120u8; mode.width() * mode.height() * 4];
        if frame.copy_bytes(&bytes).is_err() {
            println!("Failed to set frame bytes");
            return;
        }

        {
            let mut video_output = output
                .enable_video_output_scheduled(
                    mode.mode(),
                    DecklinkVideoOutputFlags::empty(),
                    25000,
                )
                .expect("Failed to enable video output");

            video_output
                .schedule_frame_copy(frame.as_ref(), 1000, 1000)
                .expect("Failed to schedule frame");
            video_output
                .schedule_frame_copy(frame.as_ref(), 2000, 1000)
                .expect("Failed to schedule frame");
            video_output
                .schedule_frame_copy(frame.as_ref(), 200000000, 1000)
                .expect("Failed to schedule frame");
            video_output
                .schedule_frame_copy(frame.as_ref(), 300000000, 1000)
                .expect("Failed to schedule frame");
            video_output
                .schedule_frame_copy(frame.as_ref(), 400000000, 1000)
                .expect("Failed to schedule frame");

            // Nothing is buffered until frames are scheduled, so check after queuing them
            let buffered_video_frame_count = video_output
                .buffered_video_frame_count()
                .expect("Failed to get buffered frame count");
            println!("Buffered frames: {}", buffered_video_frame_count);
            if buffered_video_frame_count < 5 {
                println!("Buffered frame count is less than 5");
                return;
            }

            let handler = Arc::new(CompletionCallback {});
            video_output
                .set_callback(Some(handler.clone()))
                .expect("Failed to set callback");

            video_output
                .start_playback(0, 1.0)
                .expect("Playback to start");

            println!("Press enter to continue");
            let _s: String = read!();

            // All done
        }
    }
}
