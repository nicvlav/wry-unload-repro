//! host.exe: loads guest.dll, has it build a WebView and drop it, and unloads
//! it. It does that twice, as a plug-in host does when a project is closed
//! and opened again. With wry 0.57.2 the second round crashes.

use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::FreeLibrary;
use windows_sys::Win32::System::Diagnostics::Debug::{SEM_NOGPFAULTERRORBOX, SetErrorMode};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Memory::{MEM_RESERVE, PAGE_NOACCESS, VirtualAlloc};
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, WS_OVERLAPPEDWINDOW};
use windows_sys::{s, w};

type Open = unsafe extern "C" fn(parent: isize) -> *mut c_void;
type Close = unsafe extern "C" fn(webview: *mut c_void);

fn main() {
    unsafe {
        // On a crash, exit with its code and show no dialog.
        SetErrorMode(SEM_NOGPFAULTERRORBOX);

        // A window for the WebView to be a child of. It is never shown.
        let parent = CreateWindowExW(
            0,
            w!("STATIC"),
            null(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            640,
            480,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );

        for round in 1..=2 {
            let dll = LoadLibraryW(w!("guest.dll"));
            println!("round {round}: guest.dll loaded at {dll:?}");
            let open: Open = std::mem::transmute(GetProcAddress(dll, s!("guest_open")).unwrap());
            let close: Close = std::mem::transmute(GetProcAddress(dll, s!("guest_close")).unwrap());

            let webview = open(parent as isize);
            println!("round {round}: a WebView was built");
            close(webview);
            FreeLibrary(dll);

            // Reserve the address the DLL had, so the next load lands
            // elsewhere. The loader often reuses it otherwise, and then the
            // crash does not show.
            VirtualAlloc(dll, 1, MEM_RESERVE, PAGE_NOACCESS);
        }
        println!("no crash");
    }
}
