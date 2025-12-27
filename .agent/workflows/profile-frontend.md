---
description: How to profile frontend performance using Chrome DevTools
---

# Browser Profiling for Frontend Performance

## Prerequisites
- Browser: Chrome or Edge (Chromium-based)
- Frontend running locally via `trunk serve` or similar

## Steps

// turbo
1. Start the frontend development server:
```bash
cd frontend && trunk serve
```

2. Open the app in Chrome at `http://localhost:8080`

3. Open Chrome DevTools: Right-click → Inspect, or press `Cmd+Option+I` (Mac) / `Ctrl+Shift+I` (Windows/Linux)

4. Go to the **Performance** tab

5. Click the **Record** button (circle icon) or press `Cmd+E`

6. Perform the action you want to profile (e.g., toggle the drawer, type in input)

7. Click **Stop** to end recording

8. Analyze the flame chart:
   - **Main thread work** appears in the main section
   - Look for long yellow (scripting) or purple (rendering) bars
   - Hover over bars to see function names and durations
   - Target anything taking >16ms (causes frame drops)

## Key Metrics to Watch
- **Total Blocking Time (TBT)**: Should be minimal
- **Long Tasks**: Any task >50ms is a candidate for optimization
- **Scripting vs Rendering**: Identifies if issue is JS or layout/paint

## Tips
- Profile in **Incognito** mode to avoid extension interference
- Use **CPU throttling** (6x slowdown) to exaggerate issues
- Check **Bottom-Up** view to find slowest functions
- Use **Call Tree** view to understand call hierarchy

## Exporting Results
1. Right-click on the recording → Save Profile
2. Share `.json` file for collaborative debugging
