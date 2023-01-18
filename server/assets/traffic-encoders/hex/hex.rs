extern crate core;
extern crate hex;

use std::mem::MaybeUninit;
use std::slice;

fn encode(input: &[u8]) -> Vec<u8> {
    let mut output: Vec<u8> = vec![0; input.len() * 2];
    hex::encode_to_slice(input, &mut output).unwrap();
    return output;
}

fn decode(input: &[u8]) -> Vec<u8> {
    let mut output: Vec<u8> = vec![0; input.len() / 2];
    hex::decode_to_slice(input, &mut output).unwrap();
    return output;
}

#[link(wasm_import_module = "hex")]
extern "C" {
    /// WebAssembly import which prints a string (linear memory offset,
    /// byteCount) to the console.
    ///
    /// Note: This is not an ownership transfer: Rust still owns the pointer
    /// and ensures it isn't deallocated during this call.
    #[link_name = "log"]
    fn _log(ptr: u32, size: u32);
}

/// WebAssembly export that accepts a string (linear memory offset, byteCount)
/// and returns a pointer/size pair packed into a u64.
///
/// Note: The return value is leaked to the caller, so it must call
/// [`deallocate`] when finished.
/// Note: This uses a u64 instead of two result values for compatibility with
/// WebAssembly 1.0.
#[cfg_attr(all(target_arch = "wasm32"), export_name = "encode")]
#[no_mangle]
pub unsafe extern "C" fn _encode(ptr: u32, len: u32) -> u64 {
    let input = slice::from_raw_parts_mut(ptr as *mut u8, len as usize);
    let output = encode(input);
    let (ptr, len) = (output.as_ptr(), output.len());
    // Note: This changes ownership of the pointer to the external caller. If
    // we didn't call forget, the caller would read back a corrupt value. Since
    // we call forget, the caller must deallocate externally to prevent leaks.
    std::mem::forget(output);
    return ((ptr as u64) << 32) | len as u64;
}

/// WebAssembly export that accepts a string (linear memory offset, byteCount)
/// and returns a pointer/size pair packed into a u64.
///
/// Note: The return value is leaked to the caller, so it must call
/// [`deallocate`] when finished.
/// Note: This uses a u64 instead of two result values for compatibility with
/// WebAssembly 1.0.
#[cfg_attr(all(target_arch = "wasm32"), export_name = "decode")]
#[no_mangle]
pub unsafe extern "C" fn _decode(ptr: u32, len: u32) -> u64 {
    let input = slice::from_raw_parts_mut(ptr as *mut u8, len as usize);
    log(&format!("input size: {:?}", input.len()));

    let output = decode(input);
    let (ptr, len) = (output.as_ptr(), output.len());
    // Note: This changes ownership of the pointer to the external caller. If
    // we didn't call forget, the caller would read back a corrupt value. Since
    // we call forget, the caller must deallocate externally to prevent leaks.
    std::mem::forget(output);
    return ((ptr as u64) << 32) | len as u64;
}

/// Logs a message to the console using [`_log`].
fn log(message: &String) {
    unsafe {
        let (ptr, len) = string_to_ptr(message);
        _log(ptr, len);
    }
}

/// Returns a pointer and size pair for the given string in a way compatible
/// with WebAssembly numeric types.
///
/// Note: This doesn't change the ownership of the String. To intentionally
/// leak it, use [`std::mem::forget`] on the input after calling this.
unsafe fn string_to_ptr(s: &String) -> (u32, u32) {
    return (s.as_ptr() as u32, s.len() as u32);
}

/// WebAssembly export that allocates a pointer (linear memory offset) that can
/// be used for a string.
///
/// This is an ownership transfer, which means the caller must call
/// [`deallocate`] when finished.
#[cfg_attr(all(target_arch = "wasm32"), export_name = "malloc")]
#[no_mangle]
pub extern "C" fn _allocate(size: u32) -> *mut u8 {
    let ptr = allocate(size as usize);
    log(&format!("box raw allocated at: {:?}", ptr));
    return ptr;
}

/// Allocates size bytes and leaks the pointer where they start.
fn allocate(size: usize) -> *mut u8 {
    // Allocate the amount of bytes needed.
    let buf: Vec<MaybeUninit<u8>> = Vec::with_capacity(size);
    log(&format!("vec allocated at: {:?}", buf.as_ptr()));
    // into_raw leaks the memory to the caller.
    let boxed_slice = buf.into_boxed_slice();
    log(&format!(
        "boxed_slice allocated at: {:?}",
        boxed_slice.as_ptr()
    ));
    return Box::into_raw(boxed_slice) as *mut u8;
}

/// WebAssembly export that deallocates a pointer of the given size (linear
/// memory offset, byteCount) allocated by [`allocate`].
#[cfg_attr(all(target_arch = "wasm32"), export_name = "free")]
#[no_mangle]
pub unsafe extern "C" fn _deallocate(ptr: u32, size: u32) {
    deallocate(ptr as *mut u8, size as usize);
}

/// Retakes the pointer which allows its memory to be freed.
unsafe fn deallocate(ptr: *mut u8, size: usize) {
    let _ = Vec::from_raw_parts(ptr, 0, size);
}
