export const SHORTS_ENABLED_KEY = "shortsBlockerEnabled";

const SHORT_LINK = 'a[href^="/shorts/"]';

/** Elements that are only ever Shorts. A regular video never matches any of these. */
export const SHORTS_SELECTORS = [
  "ytd-reel-shelf-renderer",
  "ytm-reel-shelf-renderer",
  "ytm-shorts-lockup-view-model",
  "ytd-rich-shelf-renderer[is-shorts]",
  'ytd-guide-entry-renderer:has(a[title="Shorts"])',
  'ytd-mini-guide-entry-renderer:has(a[title="Shorts"])',
  'yt-tab-shape[tab-title="Shorts"]',
  `grid-shelf-view-model:has(${SHORT_LINK})`,
  `ytd-rich-item-renderer:has(${SHORT_LINK})`,
  `ytd-video-renderer:has(${SHORT_LINK})`,
  `ytd-grid-video-renderer:has(${SHORT_LINK})`,
  `ytd-compact-video-renderer:has(${SHORT_LINK})`,
];

export const SHORTS_CSS = `${SHORTS_SELECTORS.join(",\n")} { display: none !important; }`;
