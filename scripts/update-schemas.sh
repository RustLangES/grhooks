#!/usr/bin/env bash
set -euo pipefail

PACKAGE_NAME="@octokit/webhooks-schemas"
CURRENT_VERSION="7.6.1"
SCHEMA_URL="https://unpkg.com/${PACKAGE_NAME}@${CURRENT_VERSION}/schema.json"
SCHEMA_FILE="crates/gh-schemes/schema.json"
OUTPUT_FILE="crates/gh-schemes/src/lib.rs"
VERSION_FILE="crates/gh-schemes/schema.version"

echo "Checking for latest version of ${PACKAGE_NAME}..."
LATEST_VERSION=$(npm view ${PACKAGE_NAME} version)

if [ -f "$VERSION_FILE" ]; then
    CURRENT_INSTALLED_VERSION=$(cat "$VERSION_FILE")
    if [ "$CURRENT_INSTALLED_VERSION" == "$LATEST_VERSION" ]; then
        echo "Schema is up to date (version ${LATEST_VERSION}). No changes needed."
        exit 0
    fi
fi

echo "New version available: ${LATEST_VERSION}"
echo "Current version: ${CURRENT_INSTALLED_VERSION:-not installed}"

echo "Downloading GitHub webhooks schema (version ${LATEST_VERSION})..."
curl -sSL "https://unpkg.com/${PACKAGE_NAME}@${LATEST_VERSION}/schema.json" -o "$SCHEMA_FILE"

echo "Generating Rust types..."
cargo typify "$SCHEMA_FILE" -o "$OUTPUT_FILE"

echo "Formatting generated code..."
rustfmt "$OUTPUT_FILE"

echo "$LATEST_VERSION" > "$VERSION_FILE"

echo "Types updated in ${OUTPUT_FILE}"
echo "Schema version ${LATEST_VERSION} successfully processed."
