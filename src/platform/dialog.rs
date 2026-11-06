//! Native dialog helpers.
//!
//! Provides UTF-16 conversions and Win32 message box popups.

use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

/// Converts a UTF-8 Rust string slice into a nul-terminated UTF-16 wide vector.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Displays a native Win32 modal error dialog box.
///
/// Note: This call blocks the calling thread until the user dismisses the dialog.
pub fn show_error_box(title: &str, message: &str) {
    let wide_title = to_wide(title);
    let wide_message = to_wide(message);

    // SAFETY: MessageBoxW is called with valid nul-terminated UTF-16 strings
    // and a null window handle pointer.
    unsafe {
        MessageBoxW(
            core::ptr::null_mut(),
            wide_message.as_ptr(),
            wide_title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_wide_ascii() {
        let wide = to_wide("hello");
        assert_eq!(wide.len(), 6);
        assert_eq!(wide.last(), Some(&0));
        assert_eq!(
            &wide[..5],
            &[
                b'h' as u16,
                b'e' as u16,
                b'l' as u16,
                b'l' as u16,
                b'o' as u16
            ]
        );
    }

    #[test]
    fn test_to_wide_empty() {
        let wide = to_wide("");
        assert_eq!(wide, vec![0]);
    }

    #[test]
    fn test_to_wide_vietnamese_diacritics() {
        let sample = "Tiến trình làm chậm máy: Ơ ư Đ đ";
        let wide = to_wide(sample);
        assert_eq!(wide.last(), Some(&0));
        let decoded = String::from_utf16(&wide[..wide.len() - 1]).unwrap();
        assert_eq!(decoded, sample);
    }
}
