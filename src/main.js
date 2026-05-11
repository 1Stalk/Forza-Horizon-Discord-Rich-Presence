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
      xblText.textContent = xbl_status || "Connecting...";
      
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
    let savedPort = localStorage.getItem("telemetry_port") || "9909";
    portInput.value = savedPort;

    portInput.addEventListener("blur", async () => {
      const portVal = parseInt(portInput.value) || 9909;
      const currentSavedPort = localStorage.getItem("telemetry_port") || "9909";
      
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

    // Tell backend we are ready to receive initial status and send initial key
    invoke("ui_ready").catch(console.error);
    invoke("update_xbl_settings", { apiKey: savedXblKey }).catch(console.error);
    invoke("update_telemetry_port", { port: parseInt(savedPort) }).catch(console.error);
  });

  // Handle external links
  document.querySelectorAll('a[target="_blank"]').forEach(link => {
    link.addEventListener('click', (e) => {
      e.preventDefault();
      invoke("open_url", { url: link.href }).catch(console.error);
    });
  });
});
