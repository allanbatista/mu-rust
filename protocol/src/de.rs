//! Deserialization helpers for protocol packets.

/// Errors that can happen while interpreting raw bytes as packet structures.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum DeserializeError {
    #[error("buffer size mismatch: expected {expected} bytes got {actual}")]
    SizeMismatch { expected: usize, actual: usize },
}

/// Attempts to reconstruct a packed C-compatible structure from the provided bytes.
pub fn deserialize<T: Copy>(bytes: &[u8]) -> Result<T, DeserializeError> {
    let size = core::mem::size_of::<T>();
    if bytes.len() != size {
        return Err(DeserializeError::SizeMismatch {
            expected: size,
            actual: bytes.len(),
        });
    }

    let mut value = core::mem::MaybeUninit::<T>::uninit();
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), value.as_mut_ptr() as *mut u8, size);
        Ok(value.assume_init())
    }
}
