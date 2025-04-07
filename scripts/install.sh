#!/bin/env bash
set -e

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

REPO="RustLangES/grhooks"
VERSION=${GRHOOKS_VERSION:-"latest"}
INSTALL_DIR=${GRHOOKS_INSTALL_DIR:-"/usr/local/bin"}
CONFIG_DIR=${GRHOOKS_CONFIG_DIR:-"/etc/grhooks"}
SERVICE_NAME=${GRHOOKS_SERVICE_NAME:-"grhooks"}
LOG_LEVEL=${GRHOOKS_LOG_LEVEL:-"info"}

if [[ "$(uname)" == "Darwin" ]]; then
    echo -e "${RED}Error: This script is just for linux systems${NC}"
    exit 1
fi

echo -e "${YELLOW}Install Configuration:${NC}"
echo -e "Version:       ${GREEN}${VERSION}${NC}"
echo -e "Install Dir:   ${GREEN}${INSTALL_DIR}${NC}"
echo -e "Configuration: ${GREEN}${CONFIG_DIR}${NC}"
echo -e "Service:       ${GREEN}${SERVICE_NAME}${NC}"
echo -e "Log Level:     ${GREEN}${LOG_LEVEL}${NC}"
echo ""

ARCH=$(uname -m)
case $ARCH in
    x86_64) ARCH="x86_64" ;;
    aarch64) ARCH="arm64" ;;
    *) echo -e "${RED}Arquitectura no soportada: $ARCH${NC}"; exit 1 ;;
esac

if [ -f /etc/debian_version ]; then
    PKG_TYPE="deb"
elif [ -f /etc/redhat-release ]; then
    PKG_TYPE="rpm"
else
    PKG_TYPE="tar.xz"
fi

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

if ! prompt_yes_no "Do you want to continue with the installation?"; then
    echo -e "${RED}Installation aborted.${NC}"
    exit 1
fi

echo -e "${YELLOW}[1/4] Downloading ...${NC}"

if [ "$VERSION" == "latest" ]; then
    DOWNLOAD_URL=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep "browser_download_url.*$ARCH" | grep "linux.*$PKG_TYPE\"" | cut -d '"' -f 4)
else
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/grhooks_${VERSION}_linux_${ARCH}.${PKG_TYPE}"
fi

if [ -z "$DOWNLOAD_URL" ]; then
    echo -e "${RED}Cannot found package for your system (Arch: $ARCH, Pkg Type: $PKG_TYPE)${NC}"
    exit 1
fi

echo "Downloading: $DOWNLOAD_URL"
TEMP_DIR=$(mktemp -d)
curl -sSL $DOWNLOAD_URL -o "$TEMP_DIR/grhooks.${PKG_TYPE}" || {
    echo -e "${RED}Fail to download package${NC}"
    exit 1
}

echo -e "${YELLOW}[2/4] Installing...${NC}"

case $PKG_TYPE in
    deb)
        sudo dpkg -i "$TEMP_DIR/grhooks.deb" || sudo apt-get install -f -y
        ;;
    rpm)
        sudo rpm -ivh "$TEMP_DIR/grhooks.rpm" || sudo yum install -y
        ;;
    tar.xz)
        sudo tar -xJf "$TEMP_DIR/grhooks.tar.xz" -C $TEMP_DIR
        sudo install -Dm755 "$TEMP_DIR/grhooks" "$INSTALL_DIR/grhooks"
        sudo chmod +x "$INSTALL_DIR/grhooks"
        ;;
esac

echo -e "${YELLOW}[3/4] Creating Config Directory...${NC}"
sudo mkdir -p $CONFIG_DIR
sudo chown $USER:$USER $CONFIG_DIR

if prompt_yes_no "Do you want to configure GRHooks as a systemd service?"; then
    echo -e "${YELLOW}[4/4] Configuring systemd service...${NC}"

    SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
    cat <<EOL | sudo tee $SERVICE_FILE > /dev/null
[Unit]
Description=GRHooks Webhook Server
After=network.target

[Service]
Type=simple
User=$USER
ExecStart=${INSTALL_DIR}/grhooks ${CONFIG_DIR}
Restart=always
RestartSec=5
Environment="GRHOOKS_LOG=${LOG_LEVEL}"

[Install]
WantedBy=multi-user.target
EOL

    sudo systemctl daemon-reload
    sudo systemctl enable $SERVICE_NAME

    if prompt_yes_no "Do you want to start the service now?"; then
        sudo systemctl start $SERVICE_NAME
        echo -e "${GREEN}Service started. You can view the logs with: journalctl -u $SERVICE_NAME -f${NC}"
    fi
else
    echo -e "${YELLOW}[4/4] Skipping systemd service configuration.${NC}"
fi

rm -rf $TEMP_DIR

echo -e "${GREEN}Installation completed!${NC}"
echo ""
echo -e "Post your manifests here:       ${YELLOW}${CONFIG_DIR}${NC}"
echo -e "Binary:       ${YELLOW}${INSTALL_DIR}/grhooks${NC}"
if [ -f "$SERVICE_FILE" ]; then
    echo -e "Service:       ${YELLOW}systemctl status ${SERVICE_NAME}${NC}"
fi
