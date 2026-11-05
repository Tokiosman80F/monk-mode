import { SHORTS_CSS, SHORTS_ENABLED_KEY } from "./shorts";

const style = document.createElement("style");
style.textContent = SHORTS_CSS;

function apply(enabled: boolean) {
  if (enabled) document.documentElement.append(style);
  else style.remove();
}

// On until told otherwise, so nothing flashes while the setting loads.
apply(true);
chrome.storage.local.get(SHORTS_ENABLED_KEY).then((stored) => {
  apply(stored[SHORTS_ENABLED_KEY] !== false);
});
chrome.storage.local.onChanged.addListener((changes) => {
  if (changes[SHORTS_ENABLED_KEY])
    apply(changes[SHORTS_ENABLED_KEY].newValue !== false);
});
