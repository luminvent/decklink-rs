use num_traits::FromPrimitive;
use std::ffi::CStr;

// TODO - refactor the error type to abstract away weird errors?
/// HRESULT codes returned by the DeckLink SDK (values match the SDK's `E_*` definitions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(i32)]
pub enum SdkError {
    FALSE = 0x0000_0001,
    UNEXPECTED = 0x8000_FFFF_u32 as i32,
    NOTIMPL = 0x8000_0001_u32 as i32,
    OUTOFMEMORY = 0x8000_0002_u32 as i32,
    INVALIDARG = 0x8000_0003_u32 as i32,
    NOINTERFACE = 0x8000_0004_u32 as i32,
    POINTER = 0x8000_0005_u32 as i32,
    HANDLE = 0x8000_0006_u32 as i32,
    ABORT = 0x8000_0007_u32 as i32,
    FAIL = 0x8000_0008_u32 as i32,
    ACCESSDENIED = 0x8000_0009_u32 as i32,
}

impl SdkError {
    /// Return the raw HRESULT code for this error.
    pub(crate) fn code(&self) -> i32 {
        *self as i32
    }

    pub(crate) fn from(value: i32) -> SdkError {
        // Unknown failure codes are reported as a generic failure rather than S_FALSE
        Self::from_i32(value).unwrap_or(if value < 0 {
            SdkError::FAIL
        } else {
            SdkError::FALSE
        })
    }
    pub(crate) fn is_false(value: i32) -> bool {
        value == (SdkError::FALSE as i32)
    }
    pub(crate) fn is_ok(value: i32) -> bool {
        value == 0
    }

    pub(crate) fn result<T>(r: i32) -> Result<T, SdkError>
    where
        T: Default,
    {
        Self::result_or_else(r, Default::default)
    }
    pub(crate) fn result_or<T>(r: i32, def: T) -> Result<T, SdkError> {
        if Self::is_ok(r) {
            Ok(def)
        } else {
            Err(Self::from(r))
        }
    }
    pub(crate) fn result_or_else<T, F: FnOnce() -> T>(r: i32, ok: F) -> Result<T, SdkError> {
        if Self::is_ok(r) {
            Ok(ok())
        } else {
            Err(Self::from(r))
        }
    }
}

pub(crate) unsafe fn convert_c_string(ptr: *const ::std::os::raw::c_char) -> String {
    CStr::from_ptr(ptr).to_str().unwrap_or_default().to_string()
}

pub(crate) unsafe fn convert_and_release_c_string(ptr: *const ::std::os::raw::c_char) -> String {
    let str = convert_c_string(ptr);
    crate::sdk::cdecklink_free_string(ptr);
    str
}

#[cfg(test)]
mod tests {
    use super::SdkError;

    #[test]
    fn maps_sdk_hresults() {
        assert_eq!(SdkError::from(0x8000_0008_u32 as i32), SdkError::FAIL);
        assert_eq!(
            SdkError::from(0x8000_0009_u32 as i32),
            SdkError::ACCESSDENIED
        );
        assert_eq!(SdkError::from(0x8000_FFFF_u32 as i32), SdkError::UNEXPECTED);
        assert_eq!(SdkError::from(1), SdkError::FALSE);
        assert_eq!(SdkError::INVALIDARG.code(), 0x8000_0003_u32 as i32);
    }
}
