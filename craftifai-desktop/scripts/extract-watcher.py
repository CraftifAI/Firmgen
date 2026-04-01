"""
Watches the electron-builder winCodeSign cache dir.
When a .7z file appears, extracts it with py7zr (which handles
macOS symlinks as regular files on Windows) before 7zip fails.
"""
import os, time, sys, py7zr, hashlib

cache_dir = os.path.join(
    os.environ.get("LOCALAPPDATA", ""),
    "electron-builder", "Cache", "winCodeSign"
)

print(f"Watching: {cache_dir}")
os.makedirs(cache_dir, exist_ok=True)

seen = set()
deadline = time.time() + 300  # give up after 5 minutes

while time.time() < deadline:
    if os.path.isdir(cache_dir):
        for fname in os.listdir(cache_dir):
            if fname.endswith(".7z") and fname not in seen:
                seen.add(fname)
                archive = os.path.join(cache_dir, fname)
                extract_dir = os.path.join(cache_dir, fname[:-3])

                # Wait for the download to finish (check file grows stable)
                prev_size = -1
                for _ in range(30):
                    try:
                        size = os.path.getsize(archive)
                    except OSError:
                        size = 0
                    if size > 0 and size == prev_size:
                        break
                    prev_size = size
                    time.sleep(0.5)

                print(f"Extracting {fname} -> {extract_dir} ...")
                os.makedirs(extract_dir, exist_ok=True)
                try:
                    with py7zr.SevenZipFile(archive, mode="r") as z:
                        # filter=None means extract all, symlinks become regular files
                        z.extract(path=extract_dir, recursive=True)
                    print(f"Extracted OK: {extract_dir}")
                except Exception as e:
                    print(f"py7zr error (non-fatal): {e}")
                    # Create placeholder files for the macOS dylibs that are symlinks
                    darwin_lib = os.path.join(extract_dir, "darwin", "10.12", "lib")
                    os.makedirs(darwin_lib, exist_ok=True)
                    for fname2 in ("libcrypto.dylib", "libssl.dylib"):
                        p = os.path.join(darwin_lib, fname2)
                        if not os.path.exists(p):
                            open(p, "w").close()
                    print("Placeholder macOS dylibs created")
    time.sleep(0.3)

print("Watcher done.")
