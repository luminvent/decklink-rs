use crate::device::output::DecklinkOutputDevicePtr;
use crate::{sdk, SdkError};
use std::rc::Rc;
use std::sync::atomic::Ordering;

pub struct DecklinkOutputDeviceAudio {
    ptr: Rc<DecklinkOutputDevicePtr>,
}
impl Drop for DecklinkOutputDeviceAudio {
    fn drop(&mut self) {
        // TODO - this is not safe!
        unsafe {
            sdk::cdecklink_output_disable_audio_output(self.ptr.dev);
            self.ptr.audio_active.store(false, Ordering::Relaxed)
        }
    }
}
impl DecklinkOutputDeviceAudio {
    pub(crate) fn from(ptr: &Rc<DecklinkOutputDevicePtr>) -> DecklinkOutputDeviceAudio {
        DecklinkOutputDeviceAudio { ptr: ptr.clone() }
    }
    pub fn write_audio_samples_sync(
        &self,
        buffer: &[i16],
        sample_frame_count: u32,
    ) -> Result<u32, SdkError> {
        unsafe {
            let mut samples_written = 0u32;
            let result = sdk::cdecklink_output_write_audio_samples_sync(
                self.ptr.dev,
                buffer.as_ptr() as *mut std::ffi::c_void,
                sample_frame_count,
                &mut samples_written,
            );
            SdkError::result_or(result, samples_written)
        }
    }

    pub fn schedule_audio_samples(
        &self,
        buffer: &[i16],
        sample_frame_count: u32,
        stream_time: i64,
        time_scale: i64,
    ) -> Result<u32, SdkError> {
        unsafe {
            let mut samples_written = 0u32;
            let result = sdk::cdecklink_output_schedule_audio_samples(
                self.ptr.dev,
                buffer.as_ptr() as *mut std::ffi::c_void,
                sample_frame_count,
                stream_time,
                time_scale,
                &mut samples_written,
            );
            SdkError::result_or(result, samples_written)
        }
    }

    pub fn begin_audio_preroll(&self) -> Result<(), SdkError> {
        unsafe {
            let result = sdk::cdecklink_output_begin_audio_preroll(self.ptr.dev);
            SdkError::result(result)
        }
    }
    pub fn end_audio_preroll(&self) -> Result<(), SdkError> {
        unsafe {
            let result = sdk::cdecklink_output_end_audio_preroll(self.ptr.dev);
            SdkError::result(result)
        }
    }

    pub fn buffered_audio_sample_frame_count(&self) -> Result<u32, SdkError> {
        unsafe {
            let mut count = 0;
            let result = sdk::cdecklink_output_get_buffered_audio_sample_frame_count(
                self.ptr.dev,
                &mut count,
            );
            SdkError::result_or(result, count)
        }
    }
    pub fn flush_buffered_audio_samples(&self) -> Result<(), SdkError> {
        unsafe {
            let result = sdk::cdecklink_output_flush_buffered_audio_samples(self.ptr.dev);
            SdkError::result(result)
        }
    }
}
