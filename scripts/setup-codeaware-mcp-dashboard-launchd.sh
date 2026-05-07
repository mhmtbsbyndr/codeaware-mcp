#!/usr/bin/env bash
set -euo pipefail

OS="$(uname -s)"
LABEL="com.mhmtbsbyndr.codeaware-mcp"
resolve_wrapper_dir() {
  for dir in "$HOME/.local/bin" "$HOME/.cache/codeaware-mcp" "/tmp/codeaware-mcp-dashboard"; do
    if mkdir -p "$dir" 2>/dev/null && touch "$dir/.codeaware-mcp-dashboard-test-write" 2>/dev/null; then
      rm -f "$dir/.codeaware-mcp-dashboard-test-write"
      echo "$dir"
      return 0
    fi
  done
  return 1
}

WRAPPER_DIR="$(resolve_wrapper_dir)" || {
  echo "No writable wrapper directory found."
  exit 1
}

WRAPPER="$WRAPPER_DIR/codeaware-mcp-dashboard-daemon.sh"
LOG_DIR_CANDIDATES=("$HOME/Library/Logs/codeaware-mcp-dashboard" "/tmp/codeaware-mcp-dashboard")
resolve_log_dir() {
  for dir in "$@"; do
    if mkdir -p "$dir" 2>/dev/null && touch "$dir/.codeaware-mcp-dashboard-test-write" 2>/dev/null; then
      rm -f "$dir/.codeaware-mcp-dashboard-test-write"
      echo "$dir"
      return 0
    fi
  done
  return 1
}

ACTION="${1:-}"
BINARY="${2:-}"

if [ "$ACTION" = "" ]; then
  echo "Usage: $0 {install|start|stop|uninstall} [codeaware-mcp-binary]"
  echo "Examples:"
  echo "  $0 install /usr/local/bin/codeaware-mcp"
  echo "  $0 install \"$HOME/.cargo/bin/codeaware-mcp\""
  echo "  $0 start"
  echo "  $0 stop"
  echo "  $0 uninstall"
  exit 1
fi

if [ "$OS" = "Darwin" ]; then
resolve_launch_dir() {
  for dir in "$HOME/Library/LaunchAgents" "/tmp/codeaware-mcp-dashboard"; do
      if mkdir -p "$dir" 2>/dev/null && touch "$dir/.codeaware-mcp-dashboard-test-write" 2>/dev/null; then
        rm -f "$dir/.codeaware-mcp-dashboard-test-write"
        echo "$dir"
        return 0
      fi
    done
    return 1
  }
  LAUNCH_DIR="$(resolve_launch_dir)" || {
    echo "No writable launch dir found."
    exit 1
  }
  PLIST="$LAUNCH_DIR/$LABEL.plist"
  LOG_DIR="$(resolve_log_dir "${LOG_DIR_CANDIDATES[@]}")" || {
    echo "No writable log directory found."
    exit 1
  }
else
  SERVICE_DIR="$HOME/.config/systemd/user"
  SERVICE_FILE="$SERVICE_DIR/$LABEL.service"
fi

if [ "$ACTION" = "install" ]; then
  if [ "$BINARY" = "" ]; then
    BINARY="${CODEAWARE_MCP_BIN:-${CODEAWARE_BINARY:-$(command -v codeaware-mcp || true)}}"
  fi
  if [ ! -x "$BINARY" ]; then
    echo "Invalid binary: $BINARY"
    exit 1
  fi

  mkdir -p "$WRAPPER_DIR"
  cat > "$WRAPPER" <<EOF
#!/usr/bin/env sh
exec tail -f /dev/null | "$BINARY"
EOF
  chmod +x "$WRAPPER"

  if [ "$OS" = "Darwin" ]; then
    mkdir -p "$LAUNCH_DIR" "$LOG_DIR"
    cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${WRAPPER}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>${LOG_DIR}/stdout.log</string>
  <key>StandardErrorPath</key>
  <string>${LOG_DIR}/stderr.log</string>
  <key>ThrottleInterval</key>
  <integer>5</integer>
</dict>
</plist>
EOF
    launchctl bootout "gui/$(id -u)" "$PLIST" 2>/dev/null || true
    launchctl bootstrap "gui/$(id -u)" "$PLIST"
    echo "Installed and started launchd job $LABEL."
    echo "Plist: $PLIST"
  else
    mkdir -p "$SERVICE_DIR"
    cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=CodeAware MCP dashboard daemon

[Service]
Type=simple
ExecStart=$WRAPPER
Restart=always
RestartSec=3

[Install]
WantedBy=default.target
EOF
    systemctl --user daemon-reload
    systemctl --user enable --now "$LABEL.service"
    echo "Installed and started user service $LABEL."
    echo "Unit: $SERVICE_FILE"
  fi

  echo "Wrapper: $WRAPPER"
  echo "View URL from MCP 'xray' tool response (or server logs)."
  exit 0
fi

if [ "$ACTION" = "start" ]; then
  if [ "$OS" = "Darwin" ]; then
    launchctl bootstrap "gui/$(id -u)" "$PLIST"
    launchctl kickstart -k "gui/$(id -u)/$LABEL"
    echo "Started $LABEL"
  else
    systemctl --user daemon-reload
    systemctl --user start "$LABEL.service"
    echo "Started $LABEL.service"
  fi
  exit 0
fi

if [ "$ACTION" = "stop" ]; then
  if [ "$OS" = "Darwin" ]; then
    launchctl bootout "gui/$(id -u)" "$PLIST" 2>/dev/null || true
    echo "Stopped $LABEL (if it was running)."
  else
    systemctl --user stop "$LABEL.service" 2>/dev/null || true
    echo "Stopped $LABEL.service (if it was running)."
  fi
  exit 0
fi

if [ "$ACTION" = "uninstall" ]; then
  if [ "$OS" = "Darwin" ]; then
    launchctl bootout "gui/$(id -u)" "$PLIST" 2>/dev/null || true
    rm -f "$PLIST"
  else
    systemctl --user stop "$LABEL.service" 2>/dev/null || true
    systemctl --user disable "$LABEL.service" 2>/dev/null || true
    rm -f "$SERVICE_FILE"
    systemctl --user daemon-reload
  fi
  rm -f "$WRAPPER"
  echo "Uninstalled $LABEL"
  exit 0
fi

echo "Unknown action: $ACTION"
echo "Usage: $0 {install|start|stop|uninstall} [codeaware-mcp-binary]"
exit 1
