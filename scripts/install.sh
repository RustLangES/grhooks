#!/usr/bin/env bash
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

REPO="RustLangES/grhooks"
VERSION=${VERSION:-"latest"}
CONFIG_DIR=${CONFIG_DIR:-"/etc/grhooks"}
SERVICE_NAME=${SERVICE_NAME:-"grhooks"}
LOG_LEVEL=${LOG_LEVEL:-"info"}

if [[ "$(uname)" == "Darwin" ]]; then
    echo -e "${RED}Error: This script is just for linux systems${NC}"
    exit 1
fi

ARCH=$(uname -m)
case $ARCH in
    x86_64) ARCH="x86_64"; ARCH_NAME="x86_64 (64-bit)" ;;
    aarch64) ARCH="arm64"; ARCH_NAME="ARM64" ;;
    *) echo -e "${RED}Architecture not supported: $ARCH${NC}"; exit 1 ;;
esac

if [[ "$(uname)" == "Darwin" ]]; then
    echo -e "${RED}Error: This script is just for linux systems${NC}"
    exit 1
fi

if [ -f /etc/debian_version ]; then
    PKG_TYPE="deb"
elif [ -f /etc/redhat-release ]; then
    PKG_TYPE="rpm"
else
    PKG_TYPE="tar.xz"
fi

if [[ "$PKG_TYPE" == "tar.xz" ]]; then
    INSTALL_DIR=${INSTALL_DIR:-"/usr/local/bin"}
fi

echo -e "${YELLOW}System Information:${NC}"
echo -e "Architecture:  ${GREEN}${ARCH_NAME}${NC}"
echo -e "Package Type:  ${GREEN}${PKG_TYPE}${NC}"
echo -e "Version:       ${GREEN}${VERSION}${NC}"
[[ -n "$INSTALL_DIR" ]] && echo -e "Install Dir:   ${GREEN}${INSTALL_DIR}${NC}"
echo -e "Configuration: ${GREEN}${CONFIG_DIR}${NC}"
echo -e "Service:       ${GREEN}${SERVICE_NAME}${NC}"
echo -e "Log Level:     ${GREEN}${LOG_LEVEL}${NC}"
echo ""

function prompt_yes_no {
    while true; do
        read -p "$1 [y/N]: " yn
        case $yn in
            [Yy]* ) return 0;;
            [Nn]* ) return 1;;
            * ) return 1;;
        esac
    done
}

confirmed=$(prompt_yes_no "Do you want to continue with the installation?")
if [ "$confirmed" -eq 1 ]; then
    echo -e "${RED}Installation aborted.${NC}"
    exit 1
fi

echo -e "${YELLOW}[1/4] Downloading package...${NC}"

if [ "$VERSION" == "latest" ]; then
    DOWNLOAD_URL=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep "browser_download_url.*$ARCH" | grep "linux.*$PKG_TYPE\"" | cut -d '"' -f 4)
else
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/grhooks_${VERSION}_linux_${ARCH}.${PKG_TYPE}"
fi

if [ -z "$DOWNLOAD_URL" ]; then
    echo -e "${RED}Cannot find package for your system (Arch: $ARCH, Pkg Type: $PKG_TYPE)${NC}"
    exit 1
fi

echo "Downloading: $DOWNLOAD_URL"
TEMP_DIR=$(mktemp -d)
curl -sSL "$DOWNLOAD_URL" -o "$TEMP_DIR/grhooks.${PKG_TYPE}" || {
    echo -e "${RED}Failed to download package${NC}"
    exit 1
}

echo -e "${YELLOW}[2/4] Installing package...${NC}"

if [[ "$PKG_TYPE" == "deb" || "$PKG_TYPE" == "rpm" ]]; then
    if [[ "$PKG_TYPE" == "deb" && $(dpkg -l | grep -q "grhooks") ]]; then
        echo -e "${YELLOW}Removing previous version...${NC}"
        sudo dpkg --remove grhooks || true
    elif [[ "$PKG_TYPE" == "rpm" && $(rpm -qa | grep -q "grhooks") ]]; then
        echo -e "${YELLOW}Removing previous version...${NC}"
        sudo rpm --erase grhooks || true
    fi

    if [[ "$PKG_TYPE" == "deb" ]]; then
        sudo dpkg -i "$TEMP_DIR/grhooks.deb" || sudo apt-get install -f -y
        INSTALL_DIR=$(which grhooks || echo "/usr/bin/grhooks")
    else
        sudo rpm -ivh "$TEMP_DIR/grhooks.rpm" || sudo yum install -y
        INSTALL_DIR=$(which grhooks || echo "/usr/bin/grhooks")
    fi
else
    sudo tar -xJf "$TEMP_DIR/grhooks.tar.xz" -C "$TEMP_DIR"
    sudo install -Dm755 "$TEMP_DIR/grhooks" "$INSTALL_DIR/grhooks"
    sudo chmod +x "$INSTALL_DIR/grhooks"
fi

echo -e "${YELLOW}[3/4] Configuring directories...${NC}"
sudo mkdir -p "$CONFIG_DIR"
sudo chown "$USER:$USER" "$CONFIG_DIR"

if prompt_yes_no "Do you want to configure GRHooks as a systemd service?"; then
    echo -e "${YELLOW}[4/4] Configuring systemd service...${NC}"

    SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
    cat <<EOL | sudo tee "$SERVICE_FILE" > /dev/null
[Unit]
Description=GRHooks Webhook Server
After=network.target

[Service]
Type=simple
User=$USER
ExecStart=${INSTALL_DIR}/grhooks --config-dir ${CONFIG_DIR}
Restart=always
RestartSec=5
Environment="GRHOOKS_LOG=${LOG_LEVEL}"

[Install]
WantedBy=multi-user.target
EOL

    sudo systemctl daemon-reload
    sudo systemctl enable "$SERVICE_NAME"

    if prompt_yes_no "Do you want to start the service now?"; then
        sudo systemctl start "$SERVICE_NAME"
        echo -e "${GREEN}Service started. You can view the logs with: journalctl -u $SERVICE_NAME -f${NC}"
    fi
else
    echo -e "${YELLOW}[4/4] Skipping systemd service configuration.${NC}"
fi

rm -rf "$TEMP_DIR"

echo -e "${GREEN}Installation completed successfully!${NC}"
echo ""
echo -e "Configuration directory: ${YELLOW}${CONFIG_DIR}${NC}"
echo -e "Binary location:         ${YELLOW}${INSTALL_DIR}/grhooks${NC}"
if [ -f "$SERVICE_FILE" ]; then
    echo -e "Service commands:"
    echo -e "  Status:  ${YELLOW}systemctl status ${SERVICE_NAME}${NC}"
    echo -e "  Start:   ${YELLOW}systemctl start ${SERVICE_NAME}${NC}"
    echo -e "  Stop:    ${YELLOW}systemctl stop ${SERVICE_NAME}${NC}"
    echo -e "  Restart: ${YELLOW}systemctl restart ${SERVICE_NAME}${NC}"
fi
