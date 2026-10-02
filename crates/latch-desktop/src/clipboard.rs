use gpui_kit::App;
#[cfg(any(not(target_os = "windows"), test))]
use gpui_kit::ClipboardItem;

pub fn write(text: &str, marker: &str, cx: &mut App) -> Result<(), String> {
    #[cfg(all(target_os = "windows", not(test)))]
    {
        use gpui_kit::AppContext;
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let window = cx
            .windows()
            .into_iter()
            .next()
            .ok_or("Latch window unavailable")?;
        cx.update_window(window, |_, window, _| {
            let handle = window
                .window_handle()
                .map_err(|_| "Window handle unavailable")?;
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return Err("Windows handle unavailable");
            };
            platform::write(
                text,
                marker,
                windows::Win32::Foundation::HWND(handle.hwnd.get() as *mut std::ffi::c_void),
            )
            .map_err(|_| "Clipboard unavailable. Try again.")
        })
        .map_err(|_| "Latch window unavailable")?
        .map_err(str::to_string)
    }
    #[cfg(any(not(target_os = "windows"), test))]
    {
        cx.write_to_clipboard(ClipboardItem::new_string_with_metadata(
            text.into(),
            marker.into(),
        ));
        #[cfg(all(target_os = "macos", not(test)))]
        {
            use objc2_app_kit::NSPasteboard;
            use objc2_foundation::{NSArray, NSString};
            let board = NSPasteboard::generalPasteboard();
            let concealed = NSString::from_str("org.nspasteboard.ConcealedType");
            let transient = NSString::from_str("org.nspasteboard.TransientType");
            let types = NSArray::from_slice(&[&*concealed, &*transient]);
            // SAFETY: no owner callback is installed; retained strings outlive the call.
            unsafe {
                board.addTypes_owner(&types, None);
            }
            let flag = NSString::from_str("true");
            board.setString_forType(&flag, &concealed);
            board.setString_forType(&flag, &transient);
        }
        if cx.read_from_clipboard().is_some_and(|item| {
            item.metadata().map(String::as_str) == Some(marker)
                && item.text().as_deref() == Some(text)
        }) {
            Ok(())
        } else {
            Err("Clipboard unavailable. Try again.".into())
        }
    }
}

#[cfg(all(target_os = "windows", not(test)))]
mod platform {
    use windows::{
        Win32::{
            Foundation::HANDLE,
            System::{
                DataExchange::{
                    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
                    SetClipboardData,
                },
                Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
                Ole::CF_UNICODETEXT,
            },
        },
        core::{Owned, w},
    };
    use zeroize::Zeroizing;
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseClipboard();
            }
        }
    }
    fn put(format: u32, bytes: &[u8]) -> windows::core::Result<()> {
        // SAFETY: the allocation is exactly bytes.len() bytes; a successful lock
        // yields writable memory of that size. Ownership passes to Windows only
        // after SetClipboardData succeeds; Owned frees it on every error path.
        unsafe {
            let allocation = Owned::new(GlobalAlloc(GMEM_MOVEABLE, bytes.len())?);
            let pointer = GlobalLock(*allocation);
            if pointer.is_null() {
                return Err(windows::core::Error::from_win32());
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast(), bytes.len());
            let _ = GlobalUnlock(*allocation);
            SetClipboardData(format, Some(HANDLE(allocation.0)))?;
            std::mem::forget(allocation);
        }
        Ok(())
    }
    fn wide(text: &str) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(
            text.encode_utf16()
                .chain(Some(0))
                .flat_map(u16::to_ne_bytes)
                .collect(),
        )
    }
    pub fn write(
        text: &str,
        marker: &str,
        owner: windows::Win32::Foundation::HWND,
    ) -> windows::core::Result<()> {
        // Set privacy flags in the same clipboard transaction as the text, before
        // closing it. A later second transaction can be too late for history.
        unsafe {
            OpenClipboard(Some(owner))?;
        }
        let _guard = Guard;
        unsafe {
            EmptyClipboard()?;
            for name in [
                w!("CanIncludeInClipboardHistory"),
                w!("CanUploadToCloudClipboard"),
            ] {
                let format = RegisterClipboardFormatW(name);
                if format == 0 {
                    return Err(windows::core::Error::from_win32());
                }
                put(format, &0u32.to_ne_bytes())?;
            }
            put(
                RegisterClipboardFormatW(w!("GPUI internal text hash")),
                &gpui_kit::ClipboardString::text_hash(text).to_ne_bytes(),
            )?;
            put(
                RegisterClipboardFormatW(w!("GPUI internal metadata")),
                &wide(marker),
            )?;
        }
        put(CF_UNICODETEXT.0 as u32, &wide(text))
    }
}
