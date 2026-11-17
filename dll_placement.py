"""
Does Windows load a different DLL at the address another DLL just left?

DLL "A" is Python's _socket.pyd. For every other DLL "B" in Python's DLLs folder:
  1. load a copy of A, note its address, unload it
  2. load a copy of B, note its address, unload it
  3. print both addresses, and B's size in memory in 64 KB blocks

Each load uses a new copy with a new file name, so Windows treats it as a DLL
it has never seen, like a freshly built plugin.
"""

import ctypes
import glob
import os
import shutil
import struct
import sys
import tempfile
import uuid

kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
kernel32.LoadLibraryW.argtypes = [ctypes.c_wchar_p]
kernel32.LoadLibraryW.restype = ctypes.c_void_p  # the module handle = its base address
kernel32.FreeLibrary.argtypes = [ctypes.c_void_p]

DLL_FOLDER = os.path.join(sys.base_prefix, "DLLs")  # the running Python's own DLLs
A = os.path.join(DLL_FOLDER, "_socket.pyd")
COPIES = tempfile.mkdtemp(prefix="dll-placement-")


def size_in_64k_blocks(path):
    """SizeOfImage from the DLL's PE header, rounded up to 64 KB blocks."""
    header = open(path, "rb").read(4096)
    pe_offset = struct.unpack_from("<I", header, 0x3C)[0]
    size_of_image = struct.unpack_from("<I", header, pe_offset + 0x50)[0]
    return -(-size_of_image // 0x10000)


def load_and_unload(path):
    """Load a new copy of the DLL, unload it, return the address it loaded at."""
    copy = os.path.join(COPIES, uuid.uuid4().hex + ".dll")
    shutil.copyfile(path, copy)
    address = kernel32.LoadLibraryW(copy)
    if not address:
        raise ctypes.WinError(ctypes.get_last_error())
    kernel32.FreeLibrary(address)
    return address


print(f"A = {os.path.basename(A)}, {size_in_64k_blocks(A)} blocks\n")
print(f"{'B':26} {'blocks':>6}  {'A loaded at':>16}  {'B loaded at':>16}  same?")

others = [p for p in glob.glob(os.path.join(DLL_FOLDER, "*.pyd")) if p != A]
for b in sorted(others, key=size_in_64k_blocks):
    a_address = load_and_unload(A)
    try:
        b_address = load_and_unload(b)
    except OSError:  # needs a DLL that sits next to the original, e.g. OpenSSL
        print(f"{os.path.basename(b):26} skipped: its dependencies are not found from a copy")
        continue
    same = "SAME" if a_address == b_address else ""
    print(f"{os.path.basename(b):26} {size_in_64k_blocks(b):>6}  "
          f"{a_address:#16x}  {b_address:#16x}  {same}")

shutil.rmtree(COPIES, ignore_errors=True)
