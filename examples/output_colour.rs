extern crate decklink;
#[macro_use]
extern crate text_io;

use decklink::device::output::{DecklinkOutputDevice, DecklinkVideoOutputFlags};
use decklink::device::DecklinkDeviceDisplayModes;
use decklink::device::{get_devices, DecklinkDevice};
use decklink::display_mode::DecklinkDisplayMode;
use decklink::frame::{DecklinkFrameFlags, DecklinkPixelFormat, DecklinkVideoMutableFrame};

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

    let output = match device.output() {
        None => {
            println!("Failed to create device output");
            return None;
        }
        Some(o) => o,
    };

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

fn main() {
    if let Some((_device, output, mode)) = select_output_and_format() {
        let mut frame = Box::new(DecklinkVideoMutableFrame::create(
            mode.width(),
            mode.height(),
            mode.width() * 4,
            DecklinkPixelFormat::Format8BitBGRA,
            DecklinkFrameFlags::empty(),
        ));

        let mut bytes = vec![0u8; mode.width() * mode.height() * 4];
        for index in 0..mode.width() * mode.height() {
            bytes[index * 4 + 2] = 255u8;
        }

        if frame.copy_bytes(&bytes).is_err() {
            println!("Failed to set frame bytes");
            return;
        }

        let video_output = output
            .enable_video_output_sync(mode.mode(), DecklinkVideoOutputFlags::empty())
            .expect("Failed to enable video output");

        video_output
            .display_custom_frame(frame)
            .expect("Failed to display frame");

        println!("Press enter to continue");
        let _s: String = read!();

        // All done
    }
}
