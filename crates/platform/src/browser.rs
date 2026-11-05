//! Read-only check for running browsers. Nothing here ever stops a process.

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

pub const SUPPORTED: [&str; 3] = ["chrome.exe", "msedge.exe", "brave.exe"];

fn supported_among<'a>(process_names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let names: Vec<&str> = process_names.collect();
    SUPPORTED
        .iter()
        .filter(|b| names.iter().any(|n| b.eq_ignore_ascii_case(n)))
        .map(|b| b.to_string())
        .collect()
}

/// The supported browsers that are running, by process name.
pub fn running_browsers() -> Vec<String> {
    // If the check can't run, assume they all are: the strict answer for F4.
    let Ok(mut system) = System::new() else {
        return SUPPORTED.iter().map(|b| b.to_string()).collect();
    };
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    supported_among(
        system
            .processes()
            .values()
            .filter_map(|p| p.name().to_str()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_each_supported_browser_in_any_case() {
        let names = ["explorer.exe", "CHROME.EXE", "chrome.exe", "brave.exe"];
        assert_eq!(
            supported_among(names.into_iter()),
            ["chrome.exe", "brave.exe"]
        );
        assert_eq!(supported_among(["msedge.exe"].into_iter()), ["msedge.exe"]);
    }

    #[test]
    fn ignores_everything_else() {
        // The app's own webview must not count as a browser, or Spotify and friends.
        let others = ["msedgewebview2.exe", "spotify.exe", "chrome_helper.exe"];
        assert!(supported_among(others.into_iter()).is_empty());
        assert!(supported_among(std::iter::empty()).is_empty());
    }

    #[test]
    fn reads_the_real_process_list() {
        // The result depends on the machine, so this only checks that the call works.
        for browser in running_browsers() {
            assert!(SUPPORTED.contains(&browser.as_str()));
        }
    }
}
