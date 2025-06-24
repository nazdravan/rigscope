// The main button offers the file that fits the visitor's system; without JavaScript it links to
// the downloads section. Phones, tablets and Chromebooks get nothing: rigscope is a desktop app,
// and Linux and Windows read real hardware (macOS shows a demo machine).

export function pickDownload(ua, platform, maxTouchPoints = 0) {
  if (/Android|iPhone|iPad|iPod|CrOS/i.test(ua)) return null;
  if (/Win/i.test(platform) || /Windows/i.test(ua)) return "windows";
  if (/Mac/i.test(platform)) return maxTouchPoints > 1 ? null : "mac";
  if (/Linux/i.test(platform) || /X11/.test(ua)) return "linux";
  return null;
}

export function withHash(href, hash) {
  return href.split("#")[0] + (hash || "");
}

function setupLanguages() {
  for (const a of document.querySelectorAll(".langs a")) {
    a.addEventListener("click", () => { a.href = withHash(a.getAttribute("href"), location.hash); });
  }
}

function setupDownload() {
  const button = document.getElementById("download-main");
  if (!button) return;
  const system = pickDownload(navigator.userAgent, navigator.platform || "", navigator.maxTouchPoints || 0);
  if (!system) return;
  const url = JSON.parse(button.dataset.kinds || "{}")[system];
  const label = button.getAttribute(`data-label-${system}`);
  if (!url || !label) return;
  button.href = url;
  button.textContent = label;
}

if (typeof document !== "undefined") {
  setupDownload();
  setupLanguages();
}
