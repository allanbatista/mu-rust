//! Serialization helpers for protocol packets.

/// Serializes a packed C-compatible structure into a byte vector.
///
/// # Safety
/// The caller must guarantee that `T` is a plain-old-data type with a
/// deterministic layout (typically `#[repr(C, packed)]`).
pub fn serialize<T: Copy>(value: &T) -> Vec<u8> {
    let size = core::mem::size_of::<T>();
    let mut buffer = Vec::with_capacity(size);
    unsafe {
        let src = value as *const T as *const u8;
        let slice = core::slice::from_raw_parts(src, size);
        buffer.extend_from_slice(slice);
    }
    buffer
}

/// Serializes the structure into an existing buffer, appending the bytes.
pub fn serialize_into<T: Copy>(value: &T, dst: &mut Vec<u8>) {
    let size = core::mem::size_of::<T>();
    unsafe {
        let src = value as *const T as *const u8;
        let slice = core::slice::from_raw_parts(src, size);
        dst.extend_from_slice(slice);
    }
}
