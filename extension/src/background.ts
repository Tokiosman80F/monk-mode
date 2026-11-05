import { buildRules, isBlocked } from "./rules";
import { browserProcess, currentTab } from "./tracker";
import { SHORTS_KEY, STATE_KEY } from "./view";
import type { BlockedSet } from "./types/BlockedSet";
import type { Message } from "./types/Message";
import type { State } from "./types/State";

const SERVICE_URL = "ws://127.0.0.1:47613";
const HEARTBEAT_MS = 5000;
const RECONNECT_MS = 2000;
const IDLE_AFTER_SEC = 120;
const WAKE_ALARM = "wake";
const SHORTS_RULESET = "shorts";

let socket: WebSocket | null = null;
let heartbeat: ReturnType<typeof setInterval> | undefined;
let reconnect: ReturnType<typeof setTimeout> | undefined;
let connecting = false;
let pendingAttempts = 0;
let idleState: chrome.idle.IdleState = "active";
let patterns: string[] = [];
let nextId = 1;

function send(message: Message) {
  if (socket?.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify({ id: String(nextId++), ...message }));
  }
}

function sendAttempt() {
  send({ type: "ext_blocked_attempt", payload: { url: "" } });
}

async function reportTab() {
  if (socket?.readyState !== WebSocket.OPEN) return;
  send({ type: "ext_active_tab", payload: await currentTab(idleState) });
}

function moveToBlockPage(tabId: number) {
  void chrome.tabs.update(tabId, { url: chrome.runtime.getURL("block.html") });
}

async function applyPatterns(next: string[]) {
  if (JSON.stringify(next) === JSON.stringify(patterns)) return;
  patterns = next;

  const existing = await chrome.declarativeNetRequest.getDynamicRules();
  await chrome.declarativeNetRequest.updateDynamicRules({
    removeRuleIds: existing.map((rule) => rule.id),
    addRules: buildRules(next),
  });

  const tabs = await chrome.tabs.query({});
  for (const tab of tabs) {
    if (tab.id !== undefined && tab.url && isBlocked(tab.url, next))
      moveToBlockPage(tab.id);
  }
}

async function applyShortsSetting(enabled: boolean) {
  await chrome.storage.local.set({ [SHORTS_KEY]: enabled });
  await chrome.declarativeNetRequest.updateEnabledRulesets(
    enabled
      ? { enableRulesetIds: [SHORTS_RULESET] }
      : { disableRulesetIds: [SHORTS_RULESET] },
  );
}

async function applyService(data: { state: State; blocked: BlockedSet }) {
  await chrome.storage.session.set({ [STATE_KEY]: data.state });
  await applyShortsSetting(data.state.settings.shorts_blocker_enabled);
  await applyPatterns(data.blocked.url_patterns);
}

function onServiceMessage(text: string) {
  const message = JSON.parse(text);
  if (message.type === "state_changed") {
    void applyService(message.payload);
  } else if (message.ok && message.data?.blocked) {
    void applyService(message.data);
  } else if (message.ok === false) {
    console.warn("Monk Mode service refused a message:", message.error);
  }
}

async function connect() {
  clearTimeout(reconnect);
  if (connecting || (socket && socket.readyState !== WebSocket.CLOSED)) return;
  connecting = true;

  let token: string;
  try {
    const response = await fetch(chrome.runtime.getURL("token.json"));
    ({ token } = await response.json());
  } catch {
    console.warn(
      "Monk Mode: token.json is missing. Install the extension from the app's folder.",
    );
    connecting = false;
    return;
  }

  const ws = new WebSocket(SERVICE_URL);
  socket = ws;
  connecting = false;
  ws.onopen = () => {
    const browser = browserProcess(navigator as never);
    send({ type: "ext_hello", payload: { token, browser } });
    for (; pendingAttempts > 0; pendingAttempts--) sendAttempt();
    void reportTab();
    clearInterval(heartbeat);
    heartbeat = setInterval(() => {
      send({ type: "ext_heartbeat" });
      void reportTab();
    }, HEARTBEAT_MS);
  };
  ws.onmessage = (event) => onServiceMessage(event.data);
  ws.onclose = () => {
    clearInterval(heartbeat);
    reconnect = setTimeout(() => void connect(), RECONNECT_MS);
  };
}

chrome.runtime.onInstalled.addListener(() => {
  chrome.alarms.create(WAKE_ALARM, { periodInMinutes: 0.5 });
});
// The worker can be stopped while the service is down. This brings it back to try again.
chrome.alarms.onAlarm.addListener(() => void connect());
chrome.runtime.onStartup.addListener(() => void connect());

chrome.idle.setDetectionInterval(IDLE_AFTER_SEC);
chrome.idle.queryState(IDLE_AFTER_SEC, (state) => (idleState = state));
chrome.idle.onStateChanged.addListener((state) => {
  idleState = state;
  void reportTab();
});

chrome.tabs.onActivated.addListener(() => void reportTab());
chrome.windows.onFocusChanged.addListener(() => void reportTab());
chrome.tabs.onUpdated.addListener((tabId, change) => {
  if (change.url && isBlocked(change.url, patterns)) moveToBlockPage(tabId);
  if (change.url || change.audible !== undefined) void reportTab();
});

chrome.runtime.onMessage.addListener((message: { type: string }) => {
  if (message.type === "blocked_attempt") {
    if (socket?.readyState === WebSocket.OPEN) sendAttempt();
    else pendingAttempts++;
  } else if (message.type === "open_app") {
    send({ type: "ext_open_app" });
  }
});

void connect();
