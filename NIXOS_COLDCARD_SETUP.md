# NixOS Configuration for Coldcard Support

## Option 1: Direct Addition to configuration.nix

Add the following to your `/etc/nixos/configuration.nix`:

```nix
{ config, pkgs, ... }:

{
  # ... your other configuration ...

  # Coldcard hardware wallet support
  services.udev.extraRules = ''
    # Coinkite Coldcard Wallet
    SUBSYSTEM=="usb", ATTRS{idVendor}=="d13e", ATTRS{idProduct}=="cc10", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    KERNEL=="hidraw*", ATTRS{idVendor}=="d13e", ATTRS{idProduct}=="cc10", MODE="0666", GROUP="plugdev", TAG+="uaccess"
  '';

  # Ensure plugdev group exists
  users.groups.plugdev = {};
  
  # Add your user to plugdev group
  users.users.youruser = {
    extraGroups = [ "plugdev" ];
  };
}
```

## Option 2: Modular Configuration

Create a separate module file `/etc/nixos/hardware-wallets.nix`:

```nix
{ config, pkgs, lib, ... }:

{
  options = {
    hardware.coldcard = {
      enable = lib.mkEnableOption "Coldcard hardware wallet support";
    };
  };

  config = lib.mkIf config.hardware.coldcard.enable {
    # Coldcard udev rules
    services.udev.extraRules = ''
      # Coinkite Coldcard - All product IDs
      SUBSYSTEM=="usb", ATTRS{idVendor}=="d13e", MODE="0666", GROUP="plugdev", TAG+="uaccess"
      KERNEL=="hidraw*", SUBSYSTEM=="hidraw", ATTRS{idVendor}=="d13e", MODE="0666", GROUP="plugdev", TAG+="uaccess"
      
      # Specific product IDs for different Coldcard versions
      SUBSYSTEM=="usb", ATTRS{idVendor}=="d13e", ATTRS{idProduct}=="cc10", MODE="0666", GROUP="plugdev", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTRS{idVendor}=="d13e", ATTRS{idProduct}=="cc15", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    '';

    # Create plugdev group
    users.groups.plugdev = {};
  };
}
```

Then in your `configuration.nix`:

```nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./hardware-wallets.nix
  ];

  # Enable Coldcard support
  hardware.coldcard.enable = true;
  
  # Add your user to plugdev
  users.users.youruser = {
    extraGroups = [ "plugdev" ];
  };
}
```

## Option 3: Using hardware.ledger Pattern

If you already use other hardware wallets, you can follow the same pattern:

```nix
{ config, pkgs, ... }:

{
  # If you use Ledger
  hardware.ledger.enable = true;
  
  # Add Coldcard rules alongside
  services.udev.extraRules = ''
    # Coldcard rules (in addition to Ledger rules)
    SUBSYSTEM=="usb", ATTRS{idVendor}=="d13e", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    KERNEL=="hidraw*", ATTRS{idVendor}=="d13e", MODE="0666", GROUP="plugdev", TAG+="uaccess"
  '';
  
  users.users.youruser = {
    extraGroups = [ "plugdev" ];
  };
}
```

## Option 4: Complete Hardware Wallet Setup

For a comprehensive hardware wallet setup:

```nix
{ config, pkgs, ... }:

{
  # Enable various hardware wallets
  hardware.ledger.enable = true;  # If you have Ledger
  
  # Coldcard and other wallets via udev rules
  services.udev.extraRules = ''
    # Coldcard
    SUBSYSTEM=="usb", ATTRS{idVendor}=="d13e", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    KERNEL=="hidraw*", ATTRS{idVendor}=="d13e", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    
    # Trezor (if needed)
    SUBSYSTEM=="usb", ATTRS{idVendor}=="534c", ATTRS{idProduct}=="0001", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    SUBSYSTEM=="usb", ATTRS{idVendor}=="1209", ATTRS{idProduct}=="53c0", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    SUBSYSTEM=="usb", ATTRS{idVendor}=="1209", ATTRS{idProduct}=="53c1", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    
    # Jade (if needed)
    SUBSYSTEM=="usb", ATTRS{idVendor}=="10c4", ATTRS{idProduct}=="ea60", MODE="0666", GROUP="plugdev", TAG+="uaccess"
  '';
  
  # Required groups
  users.groups.plugdev = {};
  
  # User configuration
  users.users.youruser = {
    extraGroups = [ "plugdev" "dialout" ];  # dialout for serial devices
  };
  
  # Optional: Install related packages
  environment.systemPackages = with pkgs; [
    # Hardware wallet packages
    ledger-live-desktop  # If using Ledger
    # Add cyberkrill here when packaged
  ];
}
```

## Applying the Configuration

After modifying your configuration:

```bash
# Rebuild and switch to the new configuration
sudo nixos-rebuild switch

# Or test first
sudo nixos-rebuild test
```

## Verification

After applying:

1. Check if rules are loaded:
```bash
# Check udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Verify the device permissions
ls -la /dev/hidraw*
```

2. Test without sudo:
```bash
# Should work without sudo after rules are applied
cyberkrill coldcard-address --path "m/84'/0'/0'/0/0" --network mainnet
```

## Troubleshooting

If it still doesn't work:

1. Ensure you're in the plugdev group:
```bash
groups
# Should show: users wheel ... plugdev ...
```

2. Log out and back in for group changes to take effect

3. Check device detection:
```bash
# Install usbutils if needed
nix-shell -p usbutils -c "lsusb | grep d13e"
```

4. Monitor udev events:
```bash
sudo udevadm monitor --environment --udev
# Then plug/unplug the Coldcard
```

## Notes

- The `TAG+="uaccess"` ensures systemd-logind also grants access to the active user
- MODE="0666" makes the device world-readable/writable (you can use "0660" for group-only access)
- Some Coldcard models may use different product IDs (cc10, cc15, etc.)
- The rules cover both USB and HID interfaces for maximum compatibility