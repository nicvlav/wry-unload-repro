//! guest.dll: a DLL that embeds wry, as a plug-in does.

use std::num::NonZeroIsize;

use wry::raw_window_handle::{RawWindowHandle, Win32WindowHandle, WindowHandle};
use wry::{WebView, WebViewBuilder};

/// Builds a WebView as a child of `parent`, a window of the host's.
#[unsafe(no_mangle)]
pub extern "C" fn guest_open(parent: isize) -> *mut WebView {
    let parent = Win32WindowHandle::new(NonZeroIsize::new(parent).unwrap());
    let parent = unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(parent)) };
    let webview = WebViewBuilder::new()
        .with_html("guest")
        .build_as_child(&parent)
        .unwrap();
    Box::into_raw(Box::new(webview))
}

/// Drops the WebView.
///
/// # Safety
///
/// `webview` came from `guest_open` and has not been closed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn guest_close(webview: *mut WebView) {
    drop(unsafe { Box::from_raw(webview) });
}
