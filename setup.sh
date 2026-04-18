#!/bin/bash

# Dirs
BIN_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/blackout"
CONFIG_DIR="$HOME/.config/blackout"
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check dependency
check_prerequisites() {
    if [ "$EUID" -eq 0 ]; then
        echo -e "${RED}Error: Don't run this script as root (sudo).${NC}"
        echo "Blackout is designed to run in the user space for IPC isolation."
        exit 1
    fi

    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: Cargo (Rust) not found in PATH.${NC}"
        exit 1
    fi
}

# Install
install_blackout() {

    # Try stop blackoutd (update)
    systemctl --user stop blackout.service 2>/dev/null

    local USE_MLOCK=""
    if [[ "$1" == "--mlock" ]]; then
        USE_MLOCK="--mlock"
        echo -e "${YELLOW}Flaged --mlock: Daemon will attempt to prevent memory swapping.${NC}"
    fi

    local branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
    echo -e "${GREEN}Installing Blackout from branch: ${branch}${NC}"
    
    # 1. Compilation
    echo "Compiling binaries in release mode..."
    if ! cargo build --release; then
        echo -e "${RED}Failed to compile.${NC}"
        exit 1
    fi

    # 2. Making Directories
    echo "Creating directories and applying restrictive permissions (chmod 700)..."
    mkdir -p "$BIN_DIR"
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$SYSTEMD_USER_DIR"
    
    mkdir -p "$DATA_DIR"
    chmod 700 "$DATA_DIR"

    # 3. Copying Binaries
    echo "Installing binaries in $BIN_DIR..."
    cp target/release/blackoutd "$BIN_DIR/"
    cp target/release/blackout "$BIN_DIR/"
    chmod +x "$BIN_DIR/blackoutd" "$BIN_DIR/blackout"

    # 4. Touching service file
    echo "Touching service file..."
    cat > "$SYSTEMD_USER_DIR/blackout.service" << EOF
[Unit]
Description=Blackout Password Manager Daemon
After=network.target

[Service]
Type=simple
Environment="RUST_LOG=info"
ExecStart=$BIN_DIR/blackoutd $USE_MLOCK
Restart=on-failure
RestartSec=5

# Aditional security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=$DATA_DIR
ReadWritePaths=/run/user/%U
PrivateNetwork=yes
LimitMEMLOCK=infinity

[Install]
WantedBy=default.target
EOF

    # 5. Ativando o Daemon
    echo "Configuring systemd..."
    systemctl --user daemon-reload
    systemctl --user enable blackout.service
    systemctl --user start blackout.service

    echo -e "${GREEN}Installation completed successfully!${NC}"
    echo "The daemon is running in the background. Test with: blackout --help"
}

# Uninstall
uninstall_blackout() {
    local PURGE=false
    if [[ "$1" == "--purge" ]]; then
        PURGE=true
    fi

    echo -e "${YELLOW}Uninstalling...${NC}"

    # 1. Stopping the daemon
    if systemctl --user is-active --quiet blackout.service; then
        echo "Stopping the daemon..."
        systemctl --user stop blackout.service
        systemctl --user disable blackout.service
    fi

    # 2. Cleaning
    echo "Cleaning up..."
    rm -f "$BIN_DIR/blackoutd"
    rm -f "$BIN_DIR/blackout"
    rm -f "$SYSTEMD_USER_DIR/blackout.service"
    systemctl --user daemon-reload

    echo "Cleaning Cargo cache..."
    cargo clean

    # 3. Purging data
    if [ "$PURGE" = true ]; then
        echo -e "${RED}Purging data...${NC}"
        rm -rf "$DATA_DIR"
        rm -rf "$CONFIG_DIR"
    else
        echo -e "${YELLOW}The encrypted vault is still in: $DATA_DIR${NC}"
        echo "To permanently remove your passwords, run: ./setup.sh uninstall --purge"
    fi

    echo -e "${GREEN}Uninstallation completed successfully.${NC}"
}

# CLIs
check_prerequisites

case "$1" in
    install)
        install_blackout "$2"
        ;;
    uninstall)
        uninstall_blackout "$2"
        ;;
    *)
        echo "Use: $0 {install|uninstall}"
        echo ""
        echo "Install:"
        echo ""
        echo -e "  install\t\tinstall blackout to current user."
	    echo -e "  install --mlock\tinstall blackout to current user. (Using 'mlock' flag)"
        echo ""
        echo "Uninstall:"
        echo -e "  uninstall\t\tRemove binaries and service"
	    echo -e "  uninstall --purge\tRemove binaries, service and vault (your passwords will be delete forever)"
        exit 1
        ;;
esac
