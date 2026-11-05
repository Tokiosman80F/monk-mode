//! The managed block in the Windows hosts file. Everything outside the block belongs to the user
//! and is kept byte for byte, including any non-UTF-8 comments, so all of this works on bytes.

use std::fs;
use std::io::{self, ErrorKind, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const START: &[u8] = b"# >>> MONK MODE BLOCKER START";
const END: &[u8] = b"# <<< MONK MODE BLOCKER END";
const CRLF: &[u8] = b"\r\n";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn system32() -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    PathBuf::from(root).join("System32")
}

/// The real hosts file, or `MONK_HOSTS_PATH` if set.
pub fn hosts_path() -> PathBuf {
    std::env::var_os("MONK_HOSTS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| system32().join(r"drivers\etc\hosts"))
}

fn without_block(existing: &[u8]) -> Vec<u8> {
    let lines: Vec<&[u8]> = existing.split_inclusive(|&b| b == b'\n').collect();
    let mut kept = Vec::with_capacity(existing.len());
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim_ascii();
        if line == START {
            // With no end marker, drop only this line. Never guess and delete the user's content.
            let end = lines[i..].iter().position(|l| l.trim_ascii() == END);
            i += end.unwrap_or(0) + 1;
        } else {
            if line != END {
                kept.extend_from_slice(lines[i]);
            }
            i += 1;
        }
    }
    kept
}

/// The hosts file contents for this list of blocked domains. An empty list removes the block.
fn render(existing: &[u8], domains: &[String]) -> Vec<u8> {
    let mut out = without_block(existing);
    if domains.is_empty() {
        return out;
    }
    if !out.is_empty() && !out.ends_with(b"\n") {
        out.extend_from_slice(CRLF);
    }
    out.extend_from_slice(START);
    out.extend_from_slice(CRLF);
    for domain in domains {
        for ip in ["0.0.0.0", "::"] {
            out.extend_from_slice(format!("{ip} {domain}\r\n").as_bytes());
        }
    }
    out.extend_from_slice(END);
    out.extend_from_slice(CRLF);
    out
}

fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".monk-tmp");
    let tmp = PathBuf::from(tmp);

    let result = (|| {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        // Marking the file read-only must not be a way to switch enforcement off.
        if let Ok(metadata) = fs::metadata(path) {
            let mut permissions = metadata.permissions();
            if permissions.readonly() {
                #[allow(clippy::permissions_set_readonly_false)] // Windows only: no Unix modes.
                permissions.set_readonly(false);
                fs::set_permissions(path, permissions)?;
            }
        }
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Makes the file match `domains`. Returns true if it had to change, and doesn't touch it if not.
pub fn apply(path: &Path, domains: &[String]) -> io::Result<bool> {
    let existing = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e),
    };
    let wanted = render(&existing, domains);
    if wanted == existing {
        return Ok(false);
    }
    write_atomic(path, &wanted)?;
    Ok(true)
}

pub fn flush_dns() -> io::Result<()> {
    let output = Command::new(system32().join("ipconfig.exe"))
        .arg("/flushdns")
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "ipconfig /flushdns failed: {}",
            output.status
        )))
    }
}

/// The service's entry point: applies the list to the real hosts file, flushing DNS on a change.
pub fn enforce(domains: &[String]) -> io::Result<bool> {
    let changed = apply(&hosts_path(), domains)?;
    if changed {
        flush_dns()?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOREIGN: &[u8] = b"# My hosts\r\n127.0.0.1 dev.local\r\n";

    fn domains(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn block(items: &[&str]) -> Vec<u8> {
        render(b"", &domains(items))
    }

    fn temp_hosts(contents: &[u8]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hosts");
        fs::write(&path, contents).unwrap();
        (dir, path)
    }

    #[test]
    fn adds_a_block_with_ipv4_and_ipv6_lines_and_crlf() {
        let (_dir, path) = temp_hosts(FOREIGN);
        assert!(apply(&path, &domains(&["tiktok.com"])).unwrap());
        let expected = concat!(
            "# My hosts\r\n127.0.0.1 dev.local\r\n",
            "# >>> MONK MODE BLOCKER START\r\n",
            "0.0.0.0 tiktok.com\r\n",
            ":: tiktok.com\r\n",
            "# <<< MONK MODE BLOCKER END\r\n",
        );
        assert_eq!(fs::read(&path).unwrap(), expected.as_bytes());
    }

    #[test]
    fn update_replaces_the_block_and_keeps_foreign_content() {
        let (_dir, path) = temp_hosts(FOREIGN);
        apply(&path, &domains(&["a.com", "b.com"])).unwrap();
        assert!(apply(&path, &domains(&["c.com"])).unwrap());
        let mut expected = FOREIGN.to_vec();
        expected.extend(block(&["c.com"]));
        assert_eq!(fs::read(&path).unwrap(), expected);
    }

    #[test]
    fn empty_list_removes_the_block_and_restores_the_file() {
        let (_dir, path) = temp_hosts(FOREIGN);
        apply(&path, &domains(&["a.com"])).unwrap();
        assert!(apply(&path, &[]).unwrap());
        assert_eq!(fs::read(&path).unwrap(), FOREIGN);
    }

    #[test]
    fn applying_twice_does_not_write_again() {
        let (_dir, path) = temp_hosts(FOREIGN);
        assert!(apply(&path, &domains(&["a.com"])).unwrap());
        let first = fs::read(&path).unwrap();
        assert!(!apply(&path, &domains(&["a.com"])).unwrap());
        assert_eq!(fs::read(&path).unwrap(), first);
    }

    #[test]
    fn removing_entries_by_hand_is_reverted() {
        let (_dir, path) = temp_hosts(FOREIGN);
        apply(&path, &domains(&["a.com"])).unwrap();
        fs::write(&path, FOREIGN).unwrap();
        assert!(apply(&path, &domains(&["a.com"])).unwrap());
        let mut expected = FOREIGN.to_vec();
        expected.extend(block(&["a.com"]));
        assert_eq!(fs::read(&path).unwrap(), expected);
    }

    #[test]
    fn foreign_content_around_a_block_is_kept() {
        let mut original = b"before\r\n".to_vec();
        original.extend(block(&["a.com"]));
        original.extend_from_slice(b"after\r\n");
        let (_dir, path) = temp_hosts(&original);
        apply(&path, &domains(&["b.com"])).unwrap();
        let mut expected = b"before\r\nafter\r\n".to_vec();
        expected.extend(block(&["b.com"]));
        assert_eq!(fs::read(&path).unwrap(), expected);
    }

    #[test]
    fn non_utf8_and_lf_only_foreign_content_is_kept_byte_for_byte() {
        let foreign: &[u8] = b"# caf\xE9\n127.0.0.1 x.local\n";
        let (_dir, path) = temp_hosts(foreign);
        apply(&path, &domains(&["a.com"])).unwrap();
        assert!(fs::read(&path).unwrap().starts_with(foreign));
        apply(&path, &[]).unwrap();
        assert_eq!(fs::read(&path).unwrap(), foreign);
    }

    #[test]
    fn a_missing_final_newline_does_not_join_lines() {
        let (_dir, path) = temp_hosts(b"127.0.0.1 x.local");
        apply(&path, &domains(&["a.com"])).unwrap();
        let mut expected = b"127.0.0.1 x.local\r\n".to_vec();
        expected.extend(block(&["a.com"]));
        assert_eq!(fs::read(&path).unwrap(), expected);
    }

    #[test]
    fn a_deleted_end_marker_never_loses_user_content() {
        let (_dir, path) = temp_hosts(b"# >>> MONK MODE BLOCKER START\r\n127.0.0.1 x.local\r\n");
        apply(&path, &domains(&["a.com"])).unwrap();
        let contents = fs::read(&path).unwrap();
        assert!(contents.starts_with(b"127.0.0.1 x.local\r\n"));
        // Stable from here on, even though the file was damaged.
        assert!(!apply(&path, &domains(&["a.com"])).unwrap());
    }

    #[test]
    fn a_stray_end_marker_is_removed() {
        let (_dir, path) = temp_hosts(b"x\r\n# <<< MONK MODE BLOCKER END\r\ny\r\n");
        apply(&path, &[]).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"x\r\ny\r\n");
    }

    #[test]
    fn a_missing_file_is_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hosts");
        assert!(apply(&path, &domains(&["a.com"])).unwrap());
        assert_eq!(fs::read(&path).unwrap(), block(&["a.com"]));
    }

    #[test]
    fn a_read_only_file_is_still_enforced() {
        let (_dir, path) = temp_hosts(FOREIGN);
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();
        assert!(apply(&path, &domains(&["a.com"])).unwrap());
        assert!(fs::read(&path).unwrap().ends_with(&block(&["a.com"])));
    }

    #[test]
    fn leaves_no_temp_file_behind() {
        let (dir, path) = temp_hosts(FOREIGN);
        apply(&path, &domains(&["a.com"])).unwrap();
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn path_can_be_overridden_by_env() {
        // The only test that reads or sets this variable.
        assert!(hosts_path().ends_with(r"drivers\etc\hosts"));
        std::env::set_var("MONK_HOSTS_PATH", r"C:\temp\hosts");
        assert_eq!(hosts_path(), PathBuf::from(r"C:\temp\hosts"));
        std::env::remove_var("MONK_HOSTS_PATH");
    }

    #[test]
    #[ignore = "needs admin rights on current Windows builds; the service has them"]
    fn dns_flush_runs() {
        flush_dns().unwrap();
    }
}
