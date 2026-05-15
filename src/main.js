const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

document.addEventListener("DOMContentLoaded", () => {
  const fixUwpBtn = document.getElementById("fix-uwp-btn");
  const uwpSuccess = document.getElementById("uwp-success");
  const uwpError = document.getElementById("uwp-error");

  const updateDbBtn = document.getElementById("update-db-btn");
  const uwpHint = document.getElementById("uwp-hint");

  const statusIndicator = document.getElementById("status-indicator");
  const statusText = document.getElementById("status-text");
  const pulseDot = document.querySelector(".pulse-dot");
  const statusDetail = document.getElementById("status-detail");

  // Fix UWP Isolation Button
  // Check if already fixed
  async function checkUwpStatus() {
    try {
      const isFixedBackend = await invoke("check_uwp_status");
      const isFixedLocal = localStorage.getItem("uwp_fixed_v2") === "true";

      if (isFixedBackend || isFixedLocal) {
        fixUwpBtn.classList.add("hidden");
        if (uwpHint) uwpHint.classList.add("hidden");
        uwpSuccess.classList.remove("hidden");
        uwpSuccess.textContent = "Network already fixed";
        if (uwpError) uwpError.classList.add("hidden");
      }
    } catch (e) {
      console.error("Failed to check UWP status", e);
    }
  }

  checkUwpStatus();

  fixUwpBtn.addEventListener("click", async () => {
    fixUwpBtn.disabled = true;
    fixUwpBtn.textContent = "Fixing...";
    if (uwpSuccess) uwpSuccess.classList.add("hidden");
    if (uwpError) uwpError.classList.add("hidden");

    try {
      // Call Rust backend command
      await invoke("fix_uwp_isolation");
      fixUwpBtn.textContent = "Fixed!";
      fixUwpBtn.classList.add("hidden");
      if (uwpHint) uwpHint.classList.add("hidden");
      uwpSuccess.classList.remove("hidden");
      uwpSuccess.textContent = "Network fixed";
      localStorage.setItem("uwp_fixed_v2", "true");
    } catch (error) {
      console.error(error);
      fixUwpBtn.textContent = "Error";
      setTimeout(() => { fixUwpBtn.textContent = "Fix Network"; fixUwpBtn.disabled = false; }, 3000);
    }
  });

  // Check DB Updates Button
  updateDbBtn.addEventListener("click", async () => {
    updateDbBtn.disabled = true;

    // Animate button
    updateDbBtn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="animation: rotateBg 2s linear infinite;"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg> Checking...`;

    try {
      await invoke("check_db_updates");
      updateDbBtn.textContent = "Updated!";
      setTimeout(() => { updateDbBtn.textContent = "Update Cars"; updateDbBtn.disabled = false; }, 3000);
    } catch (error) {
      console.error(error);
      updateDbBtn.textContent = "Error";
      setTimeout(() => { updateDbBtn.textContent = "Update Cars"; updateDbBtn.disabled = false; }, 3000);
    }
  });

  // Check Autostart Status
  const autostartCheck = document.getElementById("autostart-check");

  async function checkAutostart() {
    try {
      const isEnabled = await invoke("is_autostart_enabled");
      autostartCheck.checked = isEnabled;
    } catch (error) {
      console.error("Failed to check autostart:", error);
    }
  }

  autostartCheck.addEventListener("change", async (e) => {
    autostartCheck.disabled = true;
    try {
      await invoke("toggle_autostart", { enable: e.target.checked });
    } catch (error) {
      console.error("Failed to toggle autostart:", error);
      // Revert if failed
      autostartCheck.checked = !e.target.checked;
    } finally {
      autostartCheck.disabled = false;
    }
  });

  // Call it on load
  checkAutostart();

  // Start Minimized Setting
  const startMinimizedCheck = document.getElementById("start-minimized-check");
  const autoUpdateCheck = document.getElementById("auto-update-check");

  // Default values
  const hasRunBefore = localStorage.getItem("has_run_before");

  let startMinimized = localStorage.getItem("start_minimized");
  if (startMinimized === null) {
    startMinimized = "true";
    localStorage.setItem("start_minimized", "true");
  }

  let autoUpdate = localStorage.getItem("auto_update");
  if (autoUpdate === null) {
    autoUpdate = "true";
    localStorage.setItem("auto_update", "true");
  }

  startMinimizedCheck.checked = startMinimized === "true";
  autoUpdateCheck.checked = autoUpdate === "true";

  if (!hasRunBefore) {
    // First run: keep window visible
    localStorage.setItem("has_run_before", "true");
    invoke("show_window").catch(console.error);
  } else if (startMinimized !== "true") {
    // Not first run and setting is off: show
    invoke("show_window").catch(console.error);
  }

  // Automatic update on startup
  if (autoUpdate === "true") {
    // We can just trigger a click on the button or call the function
    updateDbBtn.click();
  }

  startMinimizedCheck.addEventListener("change", (e) => {
    localStorage.setItem("start_minimized", e.target.checked.toString());
  });

  autoUpdateCheck.addEventListener("change", (e) => {
    localStorage.setItem("auto_update", e.target.checked.toString());
  });

  // Listen for status updates from Rust backend
  listen("status_update", (event) => {
    const { status, game, details, xbl_status } = event.payload;

    if (status === "connected") {
      pulseDot.classList.add("active");
      pulseDot.classList.add("active-pulse");
      pulseDot.style.animationName = "pulse-success";

      statusText.textContent = `${game}`;
      statusText.style.color = "var(--success-color)";
      statusDetail.textContent = details || "Broadcasting presence to Discord.";

      const xblText = document.getElementById("xbl-status-text");
      if (xbl_status) {
        xblText.textContent = xbl_status;
        xblText.style.color = "var(--success-color)";
      }
    } else {
      pulseDot.classList.remove("active");
      pulseDot.classList.remove("active-pulse");
      pulseDot.style.animationName = "pulse";

      statusText.textContent = "Waiting...";
      statusText.style.color = "inherit";
      statusDetail.textContent = details || "Launch game to broadcast";

      const xblText = document.getElementById("xbl-status-text");
      const hasXblKey = !!localStorage.getItem("xbl_api_key");
      xblText.textContent = xbl_status || (hasXblKey ? "Waiting for game..." : "Disconnected");

      if (xbl_status && (xbl_status.includes("Error:") || xbl_status.includes("error:"))) {
        xblText.style.color = "var(--error-color)";
      } else {
        xblText.style.color = "inherit";
      }
    }
  }).then(() => {
    // Load XBL Api Key
    const xblKeyInput = document.getElementById("xbl-api-key");
    const xblSavedIndicator = document.getElementById("xbl-saved-indicator");
    let savedXblKey = localStorage.getItem("xbl_api_key") || "";
    xblKeyInput.value = savedXblKey;

    if (savedXblKey) {
      document.getElementById("xbl-status-text").textContent = "Waiting for game...";
    }

    xblKeyInput.addEventListener("blur", async () => {
      const key = xblKeyInput.value.trim();
      const currentSavedKey = localStorage.getItem("xbl_api_key") || "";

      if (key !== currentSavedKey) {
        localStorage.setItem("xbl_api_key", key);
        try {
          await invoke("update_xbl_settings", { apiKey: key });

          // Show indicator
          xblSavedIndicator.classList.add("visible");
          setTimeout(() => {
            xblSavedIndicator.classList.remove("visible");
          }, 2000);

          if (!key) {
            document.getElementById("xbl-status-text").textContent = "Disconnected";
          }
        } catch (err) {
          console.error("Failed to auto-save XBL key:", err);
        }
      }
    });

    // Port Settings
    const portInput = document.getElementById("telemetry-port");
    const portSavedIndicator = document.getElementById("port-saved-indicator");
    let savedPort = localStorage.getItem("telemetry_port") || "8001";
    portInput.value = savedPort;

    portInput.addEventListener("blur", async () => {
      const portVal = parseInt(portInput.value) || 8001;
      const currentSavedPort = localStorage.getItem("telemetry_port") || "8001";

      // Only save if changed
      if (portVal.toString() !== currentSavedPort) {
        localStorage.setItem("telemetry_port", portVal.toString());
        try {
          await invoke("update_telemetry_port", { port: portVal });

          // Show indicator
          portSavedIndicator.classList.add("visible");
          setTimeout(() => {
            portSavedIndicator.classList.remove("visible");
          }, 2000);
        } catch (err) {
          console.error("Failed to auto-save port:", err);
        }
      }
    });

    // UDP Forwarding Panel
    const udpForwardBtn = document.getElementById("udp-forward-btn");
    const udpForwardPanel = document.getElementById("udp-forward-panel");
    const relayActiveToggle = document.getElementById("relay-active");
    const relayIpInput = document.getElementById("relay-ip");
    const relayPortInput = document.getElementById("relay-port");
    const relaySavedIndicator = document.getElementById("relay-saved-indicator");

    // Restore saved values
    const savedRelayIp = localStorage.getItem("relay_ip") || "127.0.0.1";
    const savedRelayPort = localStorage.getItem("relay_port") || "8000";
    const relayEnabled = localStorage.getItem("relay_enabled") === "true";
    // Toggle is stored separately; default OFF
    const relayActive = localStorage.getItem("relay_active") === "true";

    relayIpInput.value = savedRelayIp;
    relayPortInput.value = savedRelayPort;
    relayActiveToggle.checked = relayActive;

    // Helper: dim/enable the IP+port fields based on toggle state
    // Also toggle the 'active' (spinning) class on the gear button
    function updateFieldsState(active) {
      const rows = udpForwardPanel.querySelectorAll(".udp-forward-row");
      rows.forEach(row => {
        if (active) {
          row.classList.remove("udp-fields-disabled");
        } else {
          row.classList.add("udp-fields-disabled");
        }
      });

      // Gear button spins if relay is actually active
      if (active) {
        udpForwardBtn.classList.add("active");
      } else {
        udpForwardBtn.classList.remove("active");
      }
    }
    updateFieldsState(relayActive);

    const BASE_HEIGHT = 365; // must match tauri.conf.json "height"
    const tauriWindow = window.__TAURI__.window;
    const getCurrentWindow = tauriWindow.getCurrentWindow;
    // LogicalSize may live in .window or .dpi depending on Tauri 2 minor version
    const LogicalSize = tauriWindow.LogicalSize ?? window.__TAURI__.dpi?.LogicalSize;
    const appWindow = getCurrentWindow();

    async function setWindowHeight(height) {
      try {
        await appWindow.setSize(new LogicalSize(530, height));
      } catch (e) {
        console.error("Failed to resize window:", e);
      }
    }

    // Restore panel visibility + window height
    if (relayEnabled) {
      udpForwardPanel.classList.remove("hidden");
      // Measure panel height after it becomes visible, then expand window
      requestAnimationFrame(() => {
        const panelH = udpForwardPanel.offsetHeight;
        setWindowHeight(BASE_HEIGHT + panelH + 25); // extra gap
      });
    }

    // Gear button toggle (only controls visibility/window size now)
    udpForwardBtn.addEventListener("click", () => {
      const isOpen = !udpForwardPanel.classList.contains("hidden");
      if (isOpen) {
        // Collapse: shrink window first, then hide panel
        setWindowHeight(BASE_HEIGHT).then(() => {
          udpForwardPanel.classList.add("hidden");
        });
        localStorage.setItem("relay_enabled", "false");
      } else {
        // Expand: show panel, then measure and grow window
        udpForwardPanel.classList.remove("hidden");
        localStorage.setItem("relay_enabled", "true");
        requestAnimationFrame(() => {
          const panelH = udpForwardPanel.offsetHeight;
          setWindowHeight(BASE_HEIGHT + panelH + 25);
        });
      }
    });

    // Toggle on/off switch
    relayActiveToggle.addEventListener("change", () => {
      const active = relayActiveToggle.checked;
      localStorage.setItem("relay_active", active.toString());
      updateFieldsState(active);
      if (active) {
        applyRelaySettings();
      } else {
        invoke("update_relay_ports", { targets: [] }).catch(console.error);
      }
    });

    async function applyRelaySettings() {
      const ip = relayIpInput.value.trim() || "127.0.0.1";
      const port = parseInt(relayPortInput.value) || 8000;
      if (port < 1 || port > 65535) return;

      // Prevent infinite UDP loop
      const telemetryPort = parseInt(document.getElementById("telemetry-port").value) || 8001;
      const isLocalhost = ip === "127.0.0.1" || ip === "localhost" || ip === "0.0.0.0";
      if (isLocalhost && port === telemetryPort) {
        console.error("Infinite loop prevented: Cannot forward to the same telemetry port!");

        // Show visual feedback that this is invalid
        relayPortInput.style.borderColor = "var(--error-color, #ff4d4d)";
        setTimeout(() => relayPortInput.style.borderColor = "", 2000);

        // Force toggle off and disable relay
        if (relayActiveToggle.checked) {
          relayActiveToggle.checked = false;
          localStorage.setItem("relay_active", "false");
          updateFieldsState(false);
          invoke("update_relay_ports", { targets: [] }).catch(console.error);
        }
        return;
      }

      try {
        await invoke("update_relay_ports", { targets: [{ ip, port }] });
        relaySavedIndicator.classList.add("visible");
        setTimeout(() => relaySavedIndicator.classList.remove("visible"), 2000);
      } catch (err) {
        console.error("Failed to apply relay settings:", err);
      }
    }

    relayIpInput.addEventListener("blur", async () => {
      if (!udpForwardPanel.classList.contains("hidden") && relayActiveToggle.checked) {
        localStorage.setItem("relay_ip", relayIpInput.value.trim() || "127.0.0.1");
        await applyRelaySettings();
      }
    });

    relayPortInput.addEventListener("blur", async () => {
      if (!udpForwardPanel.classList.contains("hidden") && relayActiveToggle.checked) {
        localStorage.setItem("relay_port", relayPortInput.value.trim() || "8000");
        await applyRelaySettings();
      }
    });

    // Tell backend we are ready to receive initial status and send initial key
    invoke("ui_ready").catch(console.error);
    invoke("update_xbl_settings", { apiKey: savedXblKey }).catch(console.error);
    invoke("update_telemetry_port", { port: parseInt(savedPort) || 8001 }).catch(console.error);
    // Apply saved relay settings on startup if toggle is on (even if panel is closed)
    if (relayActive) {
      applyRelaySettings().catch(console.error);
    }
  });

  // Listen for unknown car warnings & reporting
  const unknownCarSection = document.getElementById("unknown-car-section");
  const unknownCarWarning = document.getElementById("unknown-car-warning");
  const carNameInput = document.getElementById("car-name-input");
  const sendReportBtn = document.getElementById("send-report-btn");

  let currentUnknownCar = null;
  let currentGameName = "Forza Horizon";

  // Capture game name from status updates
  listen("status_update", (event) => {
    if (event.payload.game) {
      currentGameName = event.payload.game;
    }
  });

  listen("unknown_car", (event) => {
    const data = event.payload;
    if (data) {
      currentUnknownCar = data;
      unknownCarWarning.textContent = `Unknown car: ID ${data.id}`;
      unknownCarSection.classList.remove("invisible");
    } else {
      unknownCarSection.classList.add("invisible");
    }
  });

  sendReportBtn.addEventListener("click", async () => {
    const name = carNameInput.value.trim();
    if (!name || !currentUnknownCar) return;

    sendReportBtn.disabled = true;
    sendReportBtn.textContent = "Waiting...";

    try {
      const msg = await invoke("report_car_name", {
        carId: currentUnknownCar.id,
        carName: name,
        game: currentGameName
      });
      carNameInput.value = "";
      unknownCarWarning.textContent = msg;
      setTimeout(() => {
        unknownCarSection.classList.add("invisible");
      }, 2000);
    } catch (err) {
      console.error(err);
      alert("Failed to send report: " + err);
    } finally {
      sendReportBtn.disabled = false;
      sendReportBtn.textContent = "Report";
    }
  });

  // Handle external links
  document.querySelectorAll('a[target="_blank"]').forEach(link => {
    link.addEventListener('click', (e) => {
      e.preventDefault();
      invoke("open_url", { url: link.href }).catch(console.error);
    });
  });
});
