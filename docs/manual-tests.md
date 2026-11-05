# Manual tests

These cover what automated tests can't: the real Windows Service, real browsers and the installer. Each item maps to PRD Section 8 or a task with a manual acceptance check.

**Run these on a Windows PC or VM you're happy to lock.** Use a short lock (a few minutes, or 1 week at most). A real lock can't be undone until it ends. A VM with a snapshot is the safest choice.

Open an admin terminal for any step marked **(admin)**.

## Setup

1. Run the installer from `app/src-tauri/target/release/bundle/nsis/` (or the CI artifact). Approve the UAC prompt.
2. Follow the setup wizard, including loading the extension in Chrome, Edge and Brave and enabling **Allow in Incognito**.
3. Check the service is up **(admin)**: `sc query MonkModeService` shows `RUNNING`.

Record each result as pass or fail, with the date, under [Results](#results).

## Blocking

| #   | Test                                                                        | Expected                                                                                                            |
| --- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| B1  | With a 1-minute allowance, watch YouTube in the active tab                  | Blocked in Chrome and Edge within 5 s of it reaching 0, and in other apps too (`curl -I https://youtube.com` fails) |
| B2  | Leave YouTube playing in a background tab and work in another for 2 minutes | Allowance doesn't change                                                                                            |
| B3  | Right after install, open `tiktok.com` in every browser                     | Blocked                                                                                                             |
| B4  | Open `youtube.com/shorts/x` with allowance left                             | Block page                                                                                                          |
| B5  | Open `youtube.com/watch?v=dQw4w9WgXcQ` with allowance left                  | Video plays                                                                                                         |
| B6  | Click a Short from inside YouTube, without a page load                      | Block page                                                                                                          |
| B7  | Check the YouTube home page                                                 | No Shorts shelf or sidebar link, no console errors, normal videos unaffected                                        |
| B8  | Start a Pomodoro with allowance left, then open YouTube                     | Blocked until the focus session ends                                                                                |
| B9  | Open the Spotify and Discord desktop apps                                   | Still running and not killed                                                                                        |
| B10 | Close the extension in Edge while Edge is open, and wait 40 s               | Allowance sites blocked. Re-enabling lifts it within about 5 s                                                      |
| B11 | Leave a video playing with no mouse or keyboard input for 3 minutes         | Allowance keeps counting                                                                                            |
| B12 | Open a blocked site's tab, then let the allowance hit 0                     | The open tab moves to the block page                                                                                |
| B13 | Check the block page                                                        | Shows "Back to work.", today's progress and the streak, and "Open my plan" opens the app. There's no unblock button |
| B14 | Watch for the blocked count in Stats after hitting the block page           | The count goes up                                                                                                   |

## Tamper protection

| #   | Test                                                                                                                                                                     | Expected                                                                      |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| T1  | **(admin)** Remove the Monk Mode lines from `C:\Windows\System32\drivers\etc\hosts`                                                                                      | Back within 5 s. Your other lines are kept                                    |
| T2  | **(admin)** Mark the hosts file read-only and repeat T1                                                                                                                  | Same result                                                                   |
| T3  | Kill `monk-mode-app.exe` in Task Manager                                                                                                                                 | Back within 10 s                                                              |
| T4  | Choose **Quit** in the tray menu                                                                                                                                         | The app stays closed and the service keeps running. Starting it again works   |
| T5  | While locked **(admin)**: `sc stop MonkModeService`                                                                                                                      | Fails with an error, and the service keeps running                            |
| T6  | **(admin)** `taskkill /F /IM monk-service.exe`, then check again after 10 s                                                                                              | The service comes back and blocks resume                                      |
| T7  | Locked: in the app, try to remove a site, raise the allowance and turn off the Shorts blocker                                                                            | Controls are disabled                                                         |
| T8  | Locked: run the uninstaller                                                                                                                                              | Refuses, showing the lock's end date                                          |
| T9  | **(admin)** Locked: set the system clock back 1 month, then restart the service                                                                                          | Still locked, and the allowance isn't refilled. Set the clock back afterwards |
| T10 | **(admin)** Locked: delete `HKLM\SOFTWARE\MonkMode`, then restart the service                                                                                            | Still locked, and the key is back                                             |
| T11 | **(admin)** Locked: stop-free deletion of the database is not possible while the service runs. Boot into Safe Mode, delete `C:ProgramDataMonkModemonk.db`, boot normally | Still locked, restored from the registry                                      |
| T12 | Not locked: run the uninstaller                                                                                                                                          | Removes the service, the hosts block and the registry key                     |

## Secure DNS

Hosts file blocks can be skipped when a browser uses DNS over HTTPS (PRD Section 10).

| #   | Test                                                                    | Expected                                                                   |
| --- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| D1  | Turn on Secure DNS in Chrome, let the allowance hit 0, and open YouTube | Still blocked. The extension's rules catch it even if DNS doesn't          |
| D2  | Same, with the extension disabled for over 30 s                         | Record the result. If this site loads, add the policy task from Section 10 |

## App

| #   | Test                                                             | Expected                                                |
| --- | ---------------------------------------------------------------- | ------------------------------------------------------- |
| A1  | Check the tray menu                                              | Open, Start Pomodoro, remaining allowance text and Quit |
| A2  | Sign out and back in                                             | The app starts in the tray                              |
| A3  | Open the app twice                                               | One window. The second launch focuses it                |
| A4  | Close the window                                                 | Stays in the tray                                       |
| A5  | Let the allowance reach 2 min, 30 s and 0, and finish a Pomodoro | A notification for each                                 |
| A6  | Change Windows between light and dark                            | The app follows                                         |
| A7  | Turn on "reduce animations" in Windows                           | Animations stop                                         |
| A8  | Run idle for 10 minutes and check Task Manager                   | CPU under 1%, service plus UI RAM under 120 MB          |

## Results

| Date | Build | Tests run | Failures |
| ---- | ----- | --------- | -------- |
|      |       |           |          |
