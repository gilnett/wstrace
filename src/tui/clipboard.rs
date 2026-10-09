// wstrace - Native Win32 Clipboard Integration
// Zero external dependencies - directly interfaces with Win32 user32/kernel32 APIs

#[cfg(windows)]
mod win32_clip {
    use std::ptr;

    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(hwnd: *mut std::ffi::c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(u_format: u32, h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(u_flags: u32, dw_bytes: usize) -> *mut std::ffi::c_void;
        fn GlobalLock(h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GlobalUnlock(h_mem: *mut std::ffi::c_void) -> i32;
    }

    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;

    pub fn set_clipboard_text(text: &str) -> bool {
        let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let size_bytes = utf16.len() * std::mem::size_of::<u16>();

        unsafe {
            // Open clipboard with desktop/null owner (retry up to 10 times if another thread holds lock)
            let mut opened = false;
            for _ in 0..10 {
                if OpenClipboard(ptr::null_mut()) != 0 {
                    opened = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            if !opened {
                return false;
            }

            // Clear existing clipboard content
            EmptyClipboard();

            // Allocate moveable memory block required by Windows clipboard architecture
            let h_mem = GlobalAlloc(GMEM_MOVEABLE, size_bytes);
            if h_mem.is_null() {
                CloseClipboard();
                return false;
            }

            // Lock and write UTF-16 encoded string buffer
            let locked = GlobalLock(h_mem);
            if locked.is_null() {
                CloseClipboard();
                return false;
            }

            ptr::copy_nonoverlapping(utf16.as_ptr() as *const u8, locked as *mut u8, size_bytes);
            GlobalUnlock(h_mem);

            // Transfer memory ownership to Windows clipboard
            let res = SetClipboardData(CF_UNICODETEXT, h_mem);
            CloseClipboard();

            !res.is_null()
        }
    }
}

#[cfg(not(windows))]
mod win32_clip {
    pub fn set_clipboard_text(_text: &str) -> bool {
        false
    }
}

pub use win32_clip::set_clipboard_text;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_invocation_safeness() {
        // Verification test: calling clipboard function does not crash or corrupt memory
        let _ = set_clipboard_text("wstrace-test-payload");
    }
}
