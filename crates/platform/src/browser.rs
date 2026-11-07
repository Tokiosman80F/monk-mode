//! Read-only check for running browsers. Nothing here ever stops a process.

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub const SUPPORTED: [&str; 3] = ["chrome.exe", "msedge.exe", "brave.exe"];

/// Every open tab or window has a page renderer process. A browser without one has nothing to
/// watch, like Edge's startup boost, which keeps `msedge.exe` running in the background with no
/// window. A process whose arguments can't be read counts, the strict answer.
fn shows_a_page(args: &[&str]) -> bool {
    args.is_empty() || (args.contains(&"--type=renderer") && !args.contains(&"--extension-process"))
}

fn supported_among<'a>(processes: impl Iterator<Item = (&'a str, Vec<&'a str>)>) -> Vec<String> {
    let open: Vec<&str> = processes
        .filter(|(_, args)| shows_a_page(args))
        .map(|(name, _)| name)
        .collect();
    SUPPORTED
        .iter()
        .filter(|b| open.iter().any(|n| b.eq_ignore_ascii_case(n)))
        .map(|b| b.to_string())
        .collect()
}

/// The supported browsers with a page open, by process name.
pub fn running_browsers() -> Vec<String> {
    // If the check can't run, assume they all are: the strict answer for F4.
    let Ok(mut system) = System::new() else {
        return SUPPORTED.iter().map(|b| b.to_string()).collect();
    };
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    supported_among(system.processes().values().filter_map(|p| {
        let name = p.name().to_str()?;
        let args = p.cmd().iter().filter_map(|a| a.to_str()).collect();
        Some((name, args))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(name: &str) -> (&str, Vec<&str>) {
        (name, vec!["chrome.exe", "--type=renderer"])
    }

    #[test]
    fn finds_each_supported_browser_with_a_page_open_in_any_case() {
        let processes = [
            page("explorer.exe"),
            page("CHROME.EXE"),
            page("chrome.exe"),
            page("brave.exe"),
        ];
        assert_eq!(
            supported_among(processes.into_iter()),
            ["chrome.exe", "brave.exe"]
        );
        assert_eq!(
            supported_among([page("msedge.exe")].into_iter()),
            ["msedge.exe"]
        );
    }

    #[test]
    fn a_browser_running_in_the_background_with_no_page_does_not_count() {
        // Edge's startup boost, as seen on a real PC.
        let edge = [
            (
                "msedge.exe",
                vec!["msedge.exe", "--no-startup-window", "/prefetch:5"],
            ),
            ("msedge.exe", vec!["msedge.exe", "--type=crashpad-handler"]),
            ("msedge.exe", vec!["msedge.exe", "--type=gpu-process"]),
            ("msedge.exe", vec!["msedge.exe", "--type=utility"]),
            (
                "msedge.exe",
                vec!["msedge.exe", "--type=renderer", "--extension-process"],
            ),
        ];
        assert!(supported_among(edge.into_iter()).is_empty());
    }

    #[test]
    fn unreadable_arguments_count_as_open() {
        assert_eq!(
            supported_among([("chrome.exe", vec![])].into_iter()),
            ["chrome.exe"]
        );
    }

    #[test]
    fn ignores_everything_else() {
        // The app's own webview must not count as a browser, or Spotify and friends.
        let others = [
            page("msedgewebview2.exe"),
            page("spotify.exe"),
            page("chrome_helper.exe"),
        ];
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
