# Add this to your NixOS configuration.nix to enable Coldcard USB access

{
  # Coldcard hardware wallet udev rules
  services.udev.extraRules = ''
    # Coldcard USB rules
    # Coinkite Coldcard - USB interface
    SUBSYSTEM=="usb", ATTRS{idVendor}=="d13e", ATTRS{idProduct}=="cc10", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    
    # Coldcard - HID interface
    KERNEL=="hidraw*", ATTRS{idVendor}=="d13e", ATTRS{idProduct}=="cc10", MODE="0666", GROUP="plugdev", TAG+="uaccess"
    
    # Alternative with more specific matching
    SUBSYSTEMS=="usb", ATTRS{idVendor}=="d13e", ATTRS{idProduct}=="cc10", MODE="0660", GROUP="plugdev", TAG+="uaccess", TAG+="udev-acl"
  '';

  # Ensure the plugdev group exists
  users.groups.plugdev = {};
  
  # Add your user to the plugdev group (replace 'youruser' with your actual username)
  # users.users.youruser = {
  #   extraGroups = [ "plugdev" ];
  # };
}