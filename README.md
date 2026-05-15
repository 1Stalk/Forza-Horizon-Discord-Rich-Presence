# Forza Horizon Discord Rich Presence

**Forza Horizon 6** ✅ Supported

<img src="assets/fh6status.png" width="50%" alt="Discord Status Example" />

**Forza Horizon 5** ✅ Supported

<img src="assets/fh5status.png" width="50%" alt="Discord Status Example" />

**Forza Horizon 4** ✅ Supported

<img src="assets/fh4status.png" width="50%" alt="Discord Status Example" />

## Setup Guide

1. Launch forzarichpresence.exe (download from [releases](https://github.com/1Stalk/Forza-Horizon-Discord-Rich-Presence/releases/))
2. Launch Forza Horizon and go to **Settings** -> **HUD and Gameplay**.
3. Scroll to the bottom and configure the **Data Out** settings:
   - **Data Out:** `ON`
   - **Data Out IP Address:** `127.0.0.1`
   - **Data Out IP Port:** `8001`
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
- **OpenXBL:** Update frequency is optimized to preserve your free API limits.
- **100% Safe:** No game file modifications or memory hooking/reading involved.

## Reporting Unknown Cars
 ⚠️ In Forza Horizon 6 some cars are not recognized because I don't have the database for them yet.
 Please consider report missing cars directly in app to help speed up the process.
## Acknowledgements

- **CringeGaming** — for testing assistance during development
- **MrCoolAndroid** — author of [Xbox Rich Presence Discord](https://github.com/MrCoolAndroid/Xbox-Rich-Presence-Discord). Idea to use OpenXBL for the Rich Presence status
- **jaaiden** — author of [FH5RP](https://github.com/jaaiden/FH5RP) and [FH4RP](https://github.com/jaaiden/FH4RP). Idea to use telemetry for the Discord status
- **addidotlol** — author of [FH4-Car-ID-List](https://github.com/addidotlol/FH4-Car-ID-List). Initial FH4 car database
- **Tinase-nau** — author of [FH4-car-IDs](https://github.com/Tinase-nau/FH4-car-IDs). Updated FH4 car database
- **ForzaMods** — authors of [FH5-Car-ID-List](https://github.com/ForzaMods/FH5-Car-ID-List). FH5 car database

