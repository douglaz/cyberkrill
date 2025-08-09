# Coldcard Setup Guide for cyberkrill

## Important: Critical Runtime Issue - Stack Buffer Overflow

Coldcard support is implemented but has a **critical runtime issue**:

### Stack Smashing Detected

When attempting to connect to a Coldcard device, the program crashes with:
```
*** stack smashing detected ***: terminated
```

This occurs in both debug and release builds, with or without sudo permissions.

### Root Cause Analysis

Core dump analysis reveals a buffer overflow in the hidapi C library:
```
#6  hid_read_timeout (hidapi)
#7  hid_read (hidapi)
...
#11 coldcard::Coldcard::open
```

The crash occurs when the coldcard crate calls `hid_read_timeout`, which writes more data than expected to the buffer, corrupting the stack.

### Current Status

1. **Device Detection**: Working (USB ID: d13e:cc10)
2. **Permissions**: Fixed (/dev/hidraw12 has correct permissions)
3. **Python Library**: Works correctly (tested with ckcc-protocol)
4. **Rust Implementation**: **CRASHES** due to hidapi buffer overflow

### Known Issue

This is a known issue with hidapi where `hid_read`/`hid_read_timeout` can read more bytes than the buffer size, especially when:
- The device uses numbered reports
- There's a mismatch between expected and actual report sizes
- Different backends (hidraw vs libusb) behave differently

### Tested Configurations

| Backend | Musl Compilation | Runtime | Notes |
|---------|-----------------|---------|-------|
| linux-static-hidraw | ❌ Fails | N/A | Default backend, requires libudev |
| linux-static-libusb | ✅ Success | ❌ Fails | Compiles but runtime error |
| linux-native | ❌ Fails | N/A | Pure Rust, but coldcard crate doesn't expose feature control |

### Python Test Results

The Python ckcc-protocol library can successfully connect to the Coldcard:
```bash
# This works with sudo:
sudo nix-shell -p python3 python3Packages.ckcc-protocol --run \
  "python3 -c 'from ckcc.client import ColdcardDevice; dev = ColdcardDevice(); print(\"Connected\")'"
```

This confirms:
- The Coldcard device is properly connected (USB ID: d13e:cc10)
- It's accessible with proper permissions
- The issue is specific to the Rust hidapi bindings

## Troubleshooting "hid_error is not implemented yet"

This error occurs with all musl builds, both static and dynamic. The issue is inherent to hidapi's implementation.

### Solution 1: Use GNU/Linux Build (Recommended)

Build explicitly for the GNU target to avoid musl entirely:

```bash
# Build with GNU target (dynamic linking)
nix develop -c cargo build --release --target x86_64-unknown-linux-gnu --features coldcard

# Run from the GNU build
./target/x86_64-unknown-linux-gnu/release/cyberkrill coldcard-address
```

### Solution 2: Fix USB Permissions

The hidraw devices need proper permissions. Create a udev rule:

1. Create `/etc/udev/rules.d/51-coldcard.rules`:
```
# Coldcard USB rules
SUBSYSTEM=="usb", ATTRS{idVendor}=="d13e", ATTRS{idProduct}=="0100", MODE="0666", GROUP="plugdev"
KERNEL=="hidraw*", ATTRS{idVendor}=="d13e", ATTRS{idProduct}=="0100", MODE="0666", GROUP="plugdev"
```

2. Reload udev rules:
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

3. Ensure your user is in the `plugdev` group:
```bash
sudo usermod -a -G plugdev $USER
# Log out and back in for group changes to take effect
```

### Solution 3: Run with Elevated Permissions (Not Recommended)

As a temporary workaround:
```bash
sudo ./target/x86_64-unknown-linux-musl/release/cyberkrill coldcard-address
```

### Attempted Fix

We attempted to fix the buffer overflow by:
1. Increasing buffer sizes from 64 to 65 bytes (to account for report ID)
2. Adjusting indices to skip the report ID byte
3. Further increasing buffer to 256 bytes as a safety margin
4. Clearing buffers before reads

**Result**: The stack overflow persists. The crash occurs inside the hidapi C library's `hid_read_timeout` function, before our Rust code receives the data.

### Root Cause

The issue appears to be in the hidapi C library itself or a compatibility issue between:
- The hidapi version used by the Rust bindings (2.6.3)
- The Linux kernel's hidraw implementation
- The specific USB descriptors of the Coldcard device

### Recommended Solutions

1. **Python Subprocess Workaround** (Most Reliable):
   - The Python ckcc-protocol library works correctly
   - Call it via subprocess as a temporary solution
   - This avoids all the hidapi issues

2. **Use GNU/Linux Build Instead of Musl**:
   - Build with `--target x86_64-unknown-linux-gnu`
   - This avoids musl-specific issues but still has buffer overflow

3. **Wait for Upstream Fix**:
   - Report to hidapi maintainers with stack trace
   - The issue needs to be fixed in the C library

4. **Alternative Rust HID Libraries** (Limited Options):
   - Currently no pure Rust HID library exists for reading devices
   - The `hidraw` crate exists but lacks documentation
   - `nusb` is pure Rust USB but requires implementing HID protocol
   - `uhid-virt` is for creating virtual devices, not reading from them
   - `hidapi` with `linux-native` backend requires libudev (no static linking)

5. **Note on Serial/ACM Interface**:
   - While Coldcard exposes `/dev/ttyACM0`, this is NOT used for protocol communication
   - The ckcc-protocol exclusively uses HID for all communication
   - The ACM interface is likely for debug/firmware purposes only

## Verifying Coldcard Connection

1. Check if Coldcard is connected:
```bash
# Install usbutils if needed
nix-shell -p usbutils --run "lsusb | grep -E 'd13e:0100'"
```

2. Check hidraw devices:
```bash
ls -la /dev/hidraw*
```

3. Test with Python (if available):
```bash
# Install ckcc-protocol
pip install ckcc-protocol

# Test connection
python -c "from ckcc.client import ColdcardDevice; print(ColdcardDevice.enumerate())"
```

## Alternative: Use Coldcard in HSM Mode

For production use, consider using Coldcard in HSM (Hardware Security Module) mode. Note that HSM mode still uses the HID interface, not serial/ACM.

## How Other Projects Handle Coldcard

**Sparrow Wallet** (Java):
- Uses the Lark library with hid4java (Java wrapper around hidapi)
- JNA provides a buffer between Java and native C library
- Avoids direct Rust ↔ hidapi integration issues
- Maintains their own fork of hid4java (com.sparrowwallet:hid4java:0.8.0)

## Known Issues

1. **musl static builds**: The hidapi library has known issues with error reporting in statically linked musl builds. This is why you see "hid_error is not implemented yet".

2. **USB permissions**: Linux requires either root access or proper udev rules for USB HID devices.

3. **Backend selection**: The `linux-static-libusb` backend has issues with error handling. The `linux-static-hidraw` backend may work better but requires hidraw device access.

4. **Serial Interface Misconception**: The `/dev/ttyACM0` device exposed by Coldcard is not used for protocol communication. All normal operations use the HID interface.

## Recommended Production Setup

For production use:
1. Use the dynamic build on systems where Coldcard access is needed
2. Set up proper udev rules for security
3. Consider using Coldcard's air-gapped mode with SD card for signing
4. For automated systems, investigate HSM mode (still uses HID, not serial)