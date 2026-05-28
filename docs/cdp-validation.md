# CDP visual validation

DLSSync renders in a Tauri **WebView2** window, not a browser. To validate the UI with CDP, attach
to the **app's own WebView2 remote-debugging endpoint** — never point Edge (or any browser) at
`http://localhost:1420`, and never rely on the `orca computer` screenshot path (it captures the
foreground desktop window, not the target WebView2, so it grabs whatever browser is on top).

## How to do it

1. Build the app. A **release** build embeds the frontend and loads `tauri://localhost`, so it needs
   no dev server. A **debug** build loads `devUrl` (`http://localhost:1420`), so `pnpm --filter
   dlssync-frontend dev` must be running or the WebView shows `ERR_CONNECTION_REFUSED`
   ("Hmmm… can't reach this page"). Prefer release for standalone validation.

2. Launch the app with WebView2 remote debugging enabled (use a port that does not clash with the
   automation Edge on 9222):

   ```powershell
   $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9333'
   Start-Process .\target\release\dlssync.exe   # or target\debug\dlssync.exe with vite running
   ```

3. Confirm CDP is live and find the page target:

   ```powershell
   Invoke-RestMethod http://127.0.0.1:9333/json/version   # Browser: Edg/<ver>
   Invoke-RestMethod http://127.0.0.1:9333/json/list      # type "page" -> webSocketDebuggerUrl
   ```

4. Drive it over CDP. `Page.captureScreenshot` renders the page even when the window is minimized or
   behind another window, so there is no foreground fight. Navigate the Svelte views by clicking the
   sidebar buttons via their `title` (`button[title="Drivers"]`, `button[title="Catalog"]`, …).

   A reusable Node client (Node 22 global `WebSocket`) lives at `%TEMP%\dlss-cdp.mjs`: it reads
   `/json/list`, opens the page WebSocket, sends `Page.enable` + `Runtime.enable`, optionally
   `Page.navigate`, then `Runtime.evaluate` to click a nav button and `Page.captureScreenshot` to a
   PNG.

## Why not the other ways

- **Edge at `localhost:1420`** — that is the dev server, not the app. It has no Tauri backend, so
  every `invoke()` fails and the data views stay empty; with vite stopped it just refuses the
  connection. This is the `Hmmm… can't reach this page` screenshot to avoid.
- **`orca computer` screenshot** — on Windows it captures the foreground/topmost window, not the
  specific WebView2, so a maximized browser wins the capture. Windows `SetForegroundWindow` from a
  background process is unreliable, so this cannot be made deterministic. Use CDP-to-WebView2.
- **Single instance** — the installed v1.x in the tray holds the single-instance lock and shadows a
  fresh dev/test build. Close the installed instance before launching the build under test.
