# Forza Horizon Discord Rich Presence

**Forza Horizon 6** ⚠️ Coming soon (needs proper database with cars)

**Forza Horizon 5** ✅ Supported

**Forza Horizon 4** ✅ Supported

<img src="assets/fh4status.png" width="45%" alt="Discord Status Example" />

## Setup Guide

1. Launch forzarichpresence.exe (download from [releases](https://github.com/1Stalk/Forza-Horizon-Discord-Rich-Presence/releases/))
2. Launch Forza Horizon and go to **Settings** -> **HUD and Gameplay**.
3. Scroll to the bottom and configure the **Data Out** settings:
   - **Data Out:** `ON`
   - **Data Out IP Address:** `127.0.0.1`
   - **Data Out IP Port:** `9909`
4. Create api key at [xbl.io](https://xbl.io/)
5. Paste api key into OpenXBL Input field

## Microsoft Store / Xbox App Users

Windows blocks UWP apps from sending data to local programs. If you play the Microsoft Store version, you need to apply a network fix:
- Click the **Fix Network** button in the app.
- Accept the Administrator prompt to add a Windows Loopback Exemption. 
- You only need to do this **once**.

## Features

- **Car Database Updates:** Click "Update Cars" to automatically fetch the latest car list from this repository.
- **Set & Forget:** Enable "Run on Startup" and "Launch Minimized" to let the app run silently in your system tray.
- **SimHub:** Fully compatible with SimHub and other software that uses your forza telemetry.
