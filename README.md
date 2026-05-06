# Forza Horizon Discord Rich Presence

Works with **Forza Horizon 4** and **Forza Horizon 5** 
**Forza Horizon 6** coming soon (needs proper database with cars)

## Setup Guide

1. Launch forzarichpresence.exe
2. Launch Forza Horizon and go to **Settings** -> **HUD and Gameplay**.
3. Scroll to the bottom and configure the **Data Out** settings:
   - **Data Out:** `ON`
   - **Data Out IP Address:** `127.0.0.1`
   - **Data Out IP Port:** `9909`
4. Return to the open world (driving). Your Discord status will update automatically!

## Microsoft Store / Xbox App Users

Windows blocks UWP apps from sending data to local programs. If you play the Microsoft Store version, you need to apply a network fix:
- Click the **Fix Network** button in the app.
- Accept the Administrator prompt to add a Windows Loopback Exemption. 
- You only need to do this **once**.

## Features

- **Car Database Updates:** Click "Update Cars" to automatically fetch the latest car list from this repository.
- **Set & Forget:** Enable "Run on Startup" and "Launch Minimized" to let the app run silently in your system tray.
