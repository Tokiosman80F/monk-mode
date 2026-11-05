const BLOCK_PATH = "/block.html";

/** Must match `SHORTS_PATTERNS` in `crates/core/src/engine.rs`. */
export const DEFAULT_SHORTS_PATTERNS = [
  "||youtube.com/shorts/",
  "||instagram.com/reels/",
  "||instagram.com/reel/",
  "||facebook.com/reel/",
  "||facebook.com/watch",
];

/** Dynamic rules for the service's `url_patterns`. */
export function buildRules(
  patterns: string[],
): chrome.declarativeNetRequest.Rule[] {
  return patterns.map((urlFilter, i) => ({
    id: i + 1,
    priority: 1,
    action: {
      type: "redirect" as chrome.declarativeNetRequest.RuleActionType,
      redirect: { extensionPath: BLOCK_PATH },
    },
    condition: {
      urlFilter,
      resourceTypes: [
        "main_frame" as chrome.declarativeNetRequest.ResourceType,
      ],
    },
  }));
}

function parseUrl(url: string): URL | null {
  try {
    return new URL(url);
  } catch {
    return null;
  }
}

/**
 * Does this URL match a `||domain/path` or `||domain^` filter? Rules only catch real page loads,
 * so this finds tabs that are already open and in-page navigation, like YouTube's own links.
 */
export function matchesPattern(url: string, pattern: string): boolean {
  const parsed = parseUrl(url);
  if (!parsed || !["http:", "https:"].includes(parsed.protocol)) return false;
  const body = pattern.replace(/^\|\|/, "").replace(/\^$/, "");
  const slash = body.indexOf("/");
  const domain = slash === -1 ? body : body.slice(0, slash);
  const path = slash === -1 ? "" : body.slice(slash);
  const host = parsed.hostname.toLowerCase();
  const hostMatches = host === domain || host.endsWith(`.${domain}`);
  return hostMatches && parsed.pathname.startsWith(path);
}

export function isBlocked(url: string, patterns: string[]): boolean {
  return patterns.some((pattern) => matchesPattern(url, pattern));
}
