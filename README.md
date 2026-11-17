# Reloading a DLL that embeds wry crashes its host

Windows only. Two files:

- `src/lib.rs` is `guest.dll`. It embeds
  [wry](https://github.com/tauri-apps/wry) and builds a child WebView in a
  window its host made, as a plug-in does.
- `src/main.rs` is the host. It loads the DLL, has it build a WebView and
  drop it, and unloads the DLL. It does that twice, as a plug-in host does
  when a project is closed and opened again.

```text
cargo run
```

With wry 0.57.2 the second round crashes:

```text
round 1: guest.dll loaded at 0x7ffdf4430000
round 1: a WebView was built
round 2: guest.dll loaded at 0x1874b2e0000
error: process didn't exit successfully: `target\debug\host.exe` (exit code: 0xc000041d)
```

`0xc000041d` is an unhandled exception inside a window procedure. Under a
debugger it is an access violation: `CreateWindowExW`, called from wry's
`create_container_hwnd`, runs a procedure in the first, unloaded copy of the
DLL.

## Why

wry registers its container's window class, `WRY_WEBVIEW`, under the
executable, with a window procedure that lives in the DLL
([source](https://github.com/tauri-apps/wry/blob/wry-v0.57.2/src/webview2/mod.rs#L220-L237)).
Nothing unregisters the class, so it outlives the DLL. In the second round
the registration fails, because the class exists, and the new window is
created on the old class, whose procedure is no longer mapped.

The host reserves the address the DLL had, so the second load lands
elsewhere every time. Without that the loader often gives the DLL its old
address back, and the stale procedure is valid code by accident.

## Trying a fix

Point the build at a local checkout of wry:

```text
cargo run --config "patch.crates-io.wry.path='C:/path/to/wry'"
```

A fixed wry ends with `no crash`.

## Where a different DLL lands

Once wry registers its class under the DLL itself, a stale class is only
found again by a module loaded at the address the first DLL had. If that
module is a different build that embeds wry, its procedure sits at another
offset, and the stale class sends the window's messages to whatever code is
now at the old one.

`dll_placement.py` checks whether Windows puts a different DLL at that
address without anything forced. It loads and unloads one DLL, then loads a
different one, and prints where each landed. It uses the DLLs that ship
with Python, so it needs Python on Windows and nothing else:

```text
python -I dll_placement.py
```

Every different DLL with the same size in memory, counted in 64 KB blocks,
landed at exactly the address the first one had left. DLLs of other sizes
landed elsewhere. An excerpt, on Windows 11 x64 with ASLR on:

```text
A = _socket.pyd, 2 blocks

B                          blocks       A loaded at       B loaded at  same?
_zoneinfo.pyd                   1    0x7ff99f0d0000    0x7ff9a2570000
_asyncio.pyd                    2    0x7ff99f0d0000    0x7ff99f0d0000  SAME
_bz2.pyd                        2    0x7ff99f0d0000    0x7ff99f0d0000  SAME
_testclinic.pyd                 2    0x7ff99f0d0000    0x7ff99f0d0000  SAME
_testinternalcapi.pyd           2    0x7ff99f0d0000    0x7ff99f0d0000  SAME
_ctypes.pyd                     3    0x7ff99f0d0000    0x7ff98faa0000
```

It shows where DLLs land, not a crash: none of these DLLs embeds wry. A
plug-in rebuilt and loaded again into a host that is still running is the
likely way a different build of the same size meets the old class.
