#!/usr/bin/env bash
set -euo pipefail

# Configuration
REPO="douglaz/cyberkrill"
BINARY_NAME="cyberkrill"
INSTALL_DIR="${HOME}/.local/bin"
INSTALLED_VERSION_FILE="${INSTALL_DIR}/.${BINARY_NAME}.version"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Platform detection
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case "$os" in
        linux)
            case "$arch" in
                x86_64) echo "linux-x86_64" ;;
                aarch64) echo "linux-aarch64" ;;
                *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64) echo "macos-x86_64" ;;
                arm64) echo "macos-aarch64" ;;
                *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        mingw*|msys*|cygwin*)
            case "$arch" in
                x86_64) echo "windows-x86_64" ;;
                *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        *) echo "Unsupported OS: $os" >&2; exit 1 ;;
    esac
}

# Get latest release version from GitHub
get_latest_version() {
    # Get the latest release (including pre-releases since they pass CI)
    local releases=$(curl -s "https://api.github.com/repos/$REPO/releases")
    
    # Check if jq is available for proper JSON parsing
    if command -v jq >/dev/null 2>&1; then
        local latest_version=$(echo "$releases" | jq -r '.[0].tag_name // empty')
        if [[ -n "$latest_version" ]]; then
            echo "$latest_version"
            return
        fi
    else
        # Fallback to grep-based parsing (less reliable but works without jq)
        # Get the first tag_name from the JSON response
        local tag_name=$(echo "$releases" | grep -m1 '"tag_name":' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
        if [[ -n "$tag_name" ]]; then
            echo "$tag_name"
            return
        fi
    fi
    
    # Fall back to latest-master if API call fails or no releases found
    echo "latest-master"
}

# Get currently installed version
get_installed_version() {
    if [[ -f "$INSTALLED_VERSION_FILE" ]]; then
        cat "$INSTALLED_VERSION_FILE"
    else
        echo "none"
    fi
}

# Download and install binary
install_binary() {
    local version="$1"
    local platform="$2"
    
    echo "Downloading ${BINARY_NAME} ${version} for ${platform}..." >&2
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    
    # Determine file extension based on platform
    local ext="tar.gz"
    if [[ "$platform" == windows-* ]]; then
        ext="zip"
    fi
    
    # Construct download URL
    local url="https://github.com/${REPO}/releases/download/${version}/${BINARY_NAME}-${platform}.${ext}"
    
    # Download to temporary file
    local temp_file=$(mktemp)
    if ! curl -L -o "$temp_file" "$url"; then
        rm -f "$temp_file"
        echo "Failed to download ${BINARY_NAME}" >&2
        exit 1
    fi
    
    # Extract the archive
    local temp_dir=$(mktemp -d)
    if [[ "$ext" == "zip" ]]; then
        unzip -q "$temp_file" -d "$temp_dir"
    else
        tar -xzf "$temp_file" -C "$temp_dir"
    fi
    rm -f "$temp_file"
    
    # Find and move the binary
    # The archive contains just the platform directory (e.g., linux-x86_64/)
    local binary_path="${temp_dir}/${platform}/${BINARY_NAME}"
    if [[ "$platform" == windows-* ]]; then
        binary_path="${temp_dir}/${platform}/${BINARY_NAME}.exe"
    fi
    
    if [[ ! -f "$binary_path" ]]; then
        echo "Error: Binary not found in archive" >&2
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Make executable and move to install directory
    chmod +x "$binary_path"
    mv "$binary_path" "${INSTALL_DIR}/${BINARY_NAME}"
    rm -rf "$temp_dir"
    
    # Record installed version
    echo "$version" > "$INSTALLED_VERSION_FILE"
    
    echo "${BINARY_NAME} ${version} installed successfully" >&2
}

# Check for updates periodically (once per day)
should_check_update() {
    local check_file="${INSTALL_DIR}/.${BINARY_NAME}.last_check"
    
    # Always check if binary doesn't exist
    if [[ ! -f "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
        return 0
    fi
    
    # Check if we've checked recently
    if [[ -f "$check_file" ]]; then
        local last_check=$(stat -c %Y "$check_file" 2>/dev/null || stat -f %m "$check_file" 2>/dev/null || echo 0)
        local current_time=$(date +%s)
        local day_in_seconds=86400
        
        if (( current_time - last_check < day_in_seconds )); then
            return 1
        fi
    fi
    
    # Mark that we're checking now
    touch "$check_file"
    return 0
}


# Main logic
main() {
    # First, check if we're in the repository and can run locally
    if [[ -d "${SCRIPT_DIR}/.git" ]]; then
        # Check if we have a local build
        local local_binary="${SCRIPT_DIR}/target/release/${BINARY_NAME}"
        if [[ ! -f "$local_binary" ]]; then
            local_binary="${SCRIPT_DIR}/target/x86_64-unknown-linux-musl/release/${BINARY_NAME}"
        fi
        
        if [[ -f "$local_binary" ]]; then
            # Use local build directly
            exec "$local_binary" "$@"
        fi
    fi
    
    local platform=$(detect_platform)
    
    # Check if we should look for updates
    if should_check_update; then
        local latest_version=$(get_latest_version)
        
        if [[ -n "$latest_version" ]]; then
            local installed_version=$(get_installed_version)
            
            if [[ "$latest_version" != "$installed_version" ]]; then
                echo "New version available: ${latest_version} (installed: ${installed_version})" >&2
                install_binary "$latest_version" "$platform"
            fi
        fi
    fi
    
    # Check if binary exists in install dir
    if [[ -f "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
        exec "${INSTALL_DIR}/${BINARY_NAME}" "$@"
    fi
    
    # No installed binary - try to download latest release
    local latest_version=$(get_latest_version)
    if [[ -n "$latest_version" ]]; then
        echo "Installing cyberkrill ${latest_version}..." >&2
        install_binary "$latest_version" "$platform"
        
        # After successful install, run the binary
        if [[ -f "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
            exec "${INSTALL_DIR}/${BINARY_NAME}" "$@"
        fi
    fi
    
    # No releases available
    echo "Error: No cyberkrill releases available for download." >&2
    echo "Please check https://github.com/${REPO}/releases" >&2
    exit 1
}

# Run main function
main "$@"