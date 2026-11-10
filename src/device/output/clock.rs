use crate::{SdkError, sdk};

/// Hardware reference time of an output device, see [`DecklinkOutputHardwareClock::time`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecklinkHardwareTime {
    /// Time of the hardware reference clock
    pub hardware_time: i64,
    /// Time elapsed since the start of the current frame
    pub time_in_frame: i64,
    /// Duration of a frame, or 0 when no video output is enabled
    pub ticks_per_frame: i64,
}

/// Reads the hardware reference clock of an output device.
///
/// Unlike the other output objects, it can be sent to and shared between threads,
/// to drive the timing of the content rendered for the device on any thread.
/// It keeps the output device alive.
pub struct DecklinkOutputHardwareClock {
    dev: *mut sdk::cdecklink_output_t,
}

// The DeckLink API objects are free-threaded, and this only calls a read-only method
unsafe impl Send for DecklinkOutputHardwareClock {}
unsafe impl Sync for DecklinkOutputHardwareClock {}

impl DecklinkOutputHardwareClock {
    pub(crate) fn new(dev: *mut sdk::cdecklink_output_t) -> Self {
        unsafe { sdk::cdecklink_output_add_ref(dev) };
        Self { dev }
    }

    /// Time of the hardware reference clock, in `time_scale` units per second.
    /// The clock is monotonic and runs as long as the device is present.
    pub fn time(&self, time_scale: i64) -> Result<DecklinkHardwareTime, SdkError> {
        let mut hardware_time = 0;
        let mut time_in_frame = 0;
        let mut ticks_per_frame = 0;

        let result = unsafe {
            sdk::cdecklink_output_get_hardware_reference_clock(
                self.dev,
                time_scale,
                &mut hardware_time,
                &mut time_in_frame,
                &mut ticks_per_frame,
            )
        };

        SdkError::result_or(
            result,
            DecklinkHardwareTime {
                hardware_time,
                time_in_frame,
                ticks_per_frame,
            },
        )
    }
}

impl Clone for DecklinkOutputHardwareClock {
    fn clone(&self) -> Self {
        Self::new(self.dev)
    }
}

impl Drop for DecklinkOutputHardwareClock {
    fn drop(&mut self) {
        unsafe { sdk::cdecklink_output_release(self.dev) };
    }
}
