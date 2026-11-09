use crate::device::output::{DecklinkAudioSampleType, DecklinkOutputDevicePtr};
use crate::{SdkError, sdk};
use std::rc::Rc;
use std::sync::atomic::Ordering;

mod private {
    pub trait Sealed {}
    impl Sealed for i16 {}
    impl Sealed for i32 {}
}

/// A PCM sample type accepted by the audio output (`i16` or `i32`).
pub trait DecklinkAudioSample: private::Sealed + Copy {
    const SAMPLE_TYPE: DecklinkAudioSampleType;
}
impl DecklinkAudioSample for i16 {
    const SAMPLE_TYPE: DecklinkAudioSampleType = DecklinkAudioSampleType::Int16;
}
impl DecklinkAudioSample for i32 {
    const SAMPLE_TYPE: DecklinkAudioSampleType = DecklinkAudioSampleType::Int32;
}

pub struct DecklinkOutputDeviceAudio {
    ptr: Rc<DecklinkOutputDevicePtr>,
    sample_type: DecklinkAudioSampleType,
    channels: u32,
}
impl Drop for DecklinkOutputDeviceAudio {
    fn drop(&mut self) {
        unsafe {
            sdk::cdecklink_output_disable_audio_output(self.ptr.dev);
        }
        self.ptr.audio_active.store(false, Ordering::Relaxed)
    }
}
impl DecklinkOutputDeviceAudio {
    pub(crate) fn from(
        ptr: &Rc<DecklinkOutputDevicePtr>,
        sample_type: DecklinkAudioSampleType,
        channels: u32,
    ) -> DecklinkOutputDeviceAudio {
        DecklinkOutputDeviceAudio {
            ptr: ptr.clone(),
            sample_type,
            channels,
        }
    }

    pub fn sample_type(&self) -> DecklinkAudioSampleType {
        self.sample_type
    }
    pub fn channels(&self) -> u32 {
        self.channels
    }

    /// Ensure `buffer` matches the enabled sample type and holds `sample_frame_count`
    /// interleaved frames, so the SDK never reads past the end of the slice.
    fn check_buffer<T: DecklinkAudioSample>(
        &self,
        buffer: &[T],
        sample_frame_count: u32,
    ) -> Result<(), SdkError> {
        let required = sample_frame_count as usize * self.channels as usize;
        if T::SAMPLE_TYPE != self.sample_type || buffer.len() < required {
            Err(SdkError::INVALIDARG)
        } else {
            Ok(())
        }
    }

    /// Write interleaved samples for immediate playback.
    /// Returns the number of sample frames actually accepted, which may be fewer than requested.
    pub fn write_audio_samples_sync<T: DecklinkAudioSample>(
        &self,
        buffer: &[T],
        sample_frame_count: u32,
    ) -> Result<u32, SdkError> {
        self.check_buffer(buffer, sample_frame_count)?;
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

    /// Schedule interleaved samples at `stream_time` (in `time_scale` units).
    /// Returns the number of sample frames actually accepted, which may be fewer than requested.
    pub fn schedule_audio_samples<T: DecklinkAudioSample>(
        &self,
        buffer: &[T],
        sample_frame_count: u32,
        stream_time: i64,
        time_scale: i64,
    ) -> Result<u32, SdkError> {
        self.check_buffer(buffer, sample_frame_count)?;
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
