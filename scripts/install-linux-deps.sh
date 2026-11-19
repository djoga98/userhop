#!/usr/bin/env bash
# Installs the Ubuntu packages Tauri needs on CI runners. Ubuntu archive mirrors occasionally stall
# or answer 503, so the whole update + install is retried with a pause before giving up.
set -euo pipefail

packages=(
  libwebkit2gtk-4.1-dev
  libayatana-appindicator3-dev
  librsvg2-dev
  libxdo-dev
  libssl-dev
  build-essential
  file
  patchelf
)
options=(-o Acquire::Retries=3 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30)
attempts=3
delay="${APT_RETRY_DELAY_SECONDS:-60}"

for ((attempt = 1; attempt <= attempts; attempt++)); do
  if sudo apt-get "${options[@]}" update && sudo apt-get "${options[@]}" install -y "${packages[@]}"; then
    exit 0
  fi
  echo "::warning::Installing Linux packages failed (attempt ${attempt} of ${attempts})"
  if ((attempt < attempts)); then
    sleep $((attempt * delay))
  fi
done

echo "::error::Installing Linux packages failed after ${attempts} attempts"
exit 1
