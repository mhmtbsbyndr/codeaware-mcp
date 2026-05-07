#!/usr/bin/env bash
set -euo pipefail

LABEL="com.mhmtbsbyndr.codeaware-mcp"
LAUNCH_DIR="$HOME/Library/LaunchAgents"
PLIST="$LAUNCH_DIR/$LABEL.plist"
WRAPPER_DIR="$HOME/.local/bin"
WRAPPER="$WRAPPER_DIR/codeaware-mcp-dashboard-daemon.sh"
LOG_DIR="$HOME/Library/Logs/codeaware-mcp-dashboard"

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

if [ "$ACTION" = "install" ]; then
  if [ "$BINARY" = "" ]; then
    BINARY="${CODEAWARE_MCP_BIN:-${CODEAWARE_BINARY:-$(command -v codeaware-mcp || true)}}"
  fi
  if [ ! -x "$BINARY" ]; then
    echo "Invalid binary: $BINARY"
    exit 1
  fi

  mkdir -p "$LAUNCH_DIR" "$WRAPPER_DIR" "$LOG_DIR"

  cat > "$WRAPPER" <<EOF
#!/usr/bin/env sh
exec tail -f /dev/null | "$BINARY"
EOF
  chmod +x "$WRAPPER"

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
  echo "Wrapper: $WRAPPER"
  echo "View URL from MCP 'xray' tool response (or server log)."
  exit 0
fi

if [ "$ACTION" = "start" ]; then
  launchctl bootstrap "gui/$(id -u)" "$PLIST"
  launchctl kickstart -k "gui/$(id -u)/$LABEL"
  echo "Started $LABEL"
  exit 0
fi

if [ "$ACTION" = "stop" ]; then
  launchctl bootout "gui/$(id -u)" "$PLIST" 2>/dev/null || true
  echo "Stopped $LABEL (if it was running)."
  exit 0
fi

if [ "$ACTION" = "uninstall" ]; then
  launchctl bootout "gui/$(id -u)" "$PLIST" 2>/dev/null || true
  rm -f "$PLIST" "$WRAPPER"
  echo "Uninstalled $LABEL"
  exit 0
fi

echo "Unknown action: $ACTION"
echo "Usage: $0 {install|start|stop|uninstall} [codeaware-mcp-binary]"
exit 1

