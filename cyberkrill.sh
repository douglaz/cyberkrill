#!/usr/bin/env bash
set -euo pipefail

# Configuration
REPO="douglaz/cyberkrill"
BINARY_NAME="cyberkrill"
INSTALL_DIR="${HOME}/.local/bin"
INSTALLED_VERSION_FILE="${INSTALL_DIR}/.${BINARY_NAME}.version"
# Determine script directory (only available when run as a file, not via stdin)
if [[ -n "${BASH_SOURCE[0]:-}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
    SCRIPT_DIR=""
fi

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

# Get latest release info from GitHub. Echoes "<tag_name><TAB><published_at>".
# Both fields are used: tag_name builds the download URL, published_at is the
# cache-invalidation key. published_at is required because the project ships
# under a rolling tag ("latest-master") whose name never changes — a tag-name
# comparison would never detect new builds. published_at advances every time
# the release-binaries workflow recreates the release.
get_latest_release_info() {
    # Get the latest release (including pre-releases since they pass CI)
    local releases=$(curl -s "https://api.github.com/repos/$REPO/releases" 2>/dev/null)

    # Check if jq is available for proper JSON parsing
    if command -v jq >/dev/null 2>&1; then
        # Check if the response is an array (successful) or an object (error/rate limit)
        local is_array=$(echo "$releases" | jq -r 'if type == "array" then "yes" else "no" end' 2>/dev/null)
        if [[ "$is_array" == "yes" ]]; then
            local info=$(echo "$releases" | jq -r '.[0] | (.tag_name // empty) + "\t" + (.published_at // "unknown")' 2>/dev/null)
            if [[ -n "$info" && "$info" != $'\tunknown' ]]; then
                echo "$info"
                return
            fi
        fi
    else
        # Fallback to grep-based parsing (less reliable but works without jq).
        # Grab the first tag_name and the first published_at — release entries
        # appear newest-first in the API response, so the first hit of each
        # belongs to the same release.
        local tag_name=$(echo "$releases" | grep -m1 '"tag_name":' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
        local published_at=$(echo "$releases" | grep -m1 '"published_at":' | sed 's/.*"published_at": *"\([^"]*\)".*/\1/')
        if [[ -n "$tag_name" ]]; then
            echo -e "${tag_name}\t${published_at:-unknown}"
            return
        fi
    fi

    # Fall back to latest-master if API call fails or no releases found.
    # The "unknown" marker guarantees a cache miss on the next successful API
    # call, so a stale cache cannot persist past a transient API outage.
    echo -e "latest-master\tunknown"
}

# Get currently installed version
get_installed_version() {
    if [[ -f "$INSTALLED_VERSION_FILE" ]]; then
        cat "$INSTALLED_VERSION_FILE"
    else
        echo "none"
    fi
}

# Download and install binary.
# - tag:      drives the download URL (e.g. "latest-master" or "v0.3.0").
# - marker:   written to the version file; used by future runs to detect a
#             new build behind a rolling tag (published_at is the canonical
#             marker, see get_latest_release_info).
install_binary() {
    local tag="$1"
    local marker="$2"
    local platform="$3"

    echo "Downloading ${BINARY_NAME} ${tag} (${marker}) for ${platform}..." >&2

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Determine file extension based on platform
    local ext="tar.gz"
    if [[ "$platform" == windows-* ]]; then
        ext="zip"
    fi

    # Construct download URL
    local url="https://github.com/${REPO}/releases/download/${tag}/${BINARY_NAME}-${platform}.${ext}"
    
    # Download to temporary file
    local temp_file=$(mktemp)
    if ! curl -sL -o "$temp_file" "$url"; then
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
    
    # Record installed marker (published_at, or "unknown" if the API was down).
    # Future runs compare this against get_latest_release_info to detect new
    # builds, including ones reusing the same rolling tag name.
    echo "$marker" > "$INSTALLED_VERSION_FILE"

    echo "${BINARY_NAME} ${tag} (${marker}) installed successfully" >&2
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
    if [[ -n "$SCRIPT_DIR" && -d "${SCRIPT_DIR}/.git" ]]; then
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
        local release_info=$(get_latest_release_info)

        if [[ -n "$release_info" ]]; then
            local latest_tag="${release_info%%$'\t'*}"
            local latest_marker="${release_info#*$'\t'}"
            local installed_marker=$(get_installed_version)

            if [[ "$latest_marker" != "$installed_marker" ]]; then
                echo "New build available: ${latest_tag} published ${latest_marker} (installed: ${installed_marker})" >&2
                install_binary "$latest_tag" "$latest_marker" "$platform"
            fi
        fi
    fi

    # Check if binary exists in install dir
    if [[ -f "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
        exec "${INSTALL_DIR}/${BINARY_NAME}" "$@"
    fi

    # No installed binary - try to download latest release
    local release_info=$(get_latest_release_info)
    if [[ -n "$release_info" ]]; then
        local latest_tag="${release_info%%$'\t'*}"
        local latest_marker="${release_info#*$'\t'}"
        echo "Installing cyberkrill ${latest_tag} (${latest_marker})..." >&2
        install_binary "$latest_tag" "$latest_marker" "$platform"

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