//! A signed copy of the lock's end time in `HKLM\SOFTWARE\MonkMode`, so deleting the database
//! alone doesn't unlock.
//!
// ponytail: the key comes from MachineGuid plus a salt in this open-source code, so an admin who
// reads the code could forge a signature. That's within F6's goal of slow and annoying. Only a
// secret that admins can't read would close it, and Windows has no such place.

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use thiserror::Error;
use windows_registry::{Key, CURRENT_USER, LOCAL_MACHINE};

const KEY_PATH: &str = r"SOFTWARE\MonkMode";
const ENDS_AT: &str = "LockEndsAt";
const SIGNATURE: &str = "LockSignature";
const SALT: &str = "monk-mode-lock-v1";
const NOT_FOUND: i32 = 0x8007_0002_u32 as i32;

#[derive(Debug, Error)]
pub enum LockRegistryError {
    #[error("The stored lock copy failed its signature check.")]
    Tampered,
    #[error(transparent)]
    Windows(#[from] windows_result::Error),
}

fn absent_to_none<T>(result: windows_registry::Result<T>) -> windows_registry::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.code().0 == NOT_FOUND => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct LockRegistry {
    root: &'static Key,
    path: String,
    machine_key: String,
}

impl LockRegistry {
    /// The real location. Writing needs admin rights, which the service has.
    pub fn machine() -> Result<Self, LockRegistryError> {
        let machine_key = LOCAL_MACHINE
            .open(r"SOFTWARE\Microsoft\Cryptography")?
            .get_string("MachineGuid")?;
        Ok(Self {
            root: LOCAL_MACHINE,
            path: KEY_PATH.to_string(),
            machine_key,
        })
    }

    /// Signature for a lock's end time. The database stores the same value (`lock.signature`).
    pub fn sign(&self, ends_at: DateTime<Utc>) -> String {
        self.sign_text(&ends_at.to_rfc3339())
    }

    pub fn verify(&self, ends_at: DateTime<Utc>, signature: &str) -> bool {
        self.sign(ends_at) == signature
    }

    fn sign_text(&self, text: &str) -> String {
        let key = format!("{SALT}:{}", self.machine_key);
        let mut mac =
            Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("HMAC takes keys of any length");
        mac.update(text.as_bytes());
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    pub fn write(&self, ends_at: DateTime<Utc>) -> Result<(), LockRegistryError> {
        let key = self.root.create(&self.path)?;
        key.set_string(ENDS_AT, ends_at.to_rfc3339())?;
        key.set_string(SIGNATURE, self.sign(ends_at))?;
        Ok(())
    }

    /// The stored end time, or None if nothing is stored. Half a copy counts as tampering.
    pub fn read(&self) -> Result<Option<DateTime<Utc>>, LockRegistryError> {
        let Some(key) = absent_to_none(self.root.open(&self.path))? else {
            return Ok(None);
        };
        let ends_at = absent_to_none(key.get_string(ENDS_AT))?;
        let signature = absent_to_none(key.get_string(SIGNATURE))?;
        let (ends_at, signature) = match (ends_at, signature) {
            (None, None) => return Ok(None),
            (Some(e), Some(s)) => (e, s),
            _ => return Err(LockRegistryError::Tampered),
        };
        if self.sign_text(&ends_at) != signature {
            return Err(LockRegistryError::Tampered);
        }
        let parsed = DateTime::parse_from_rfc3339(&ends_at);
        Ok(Some(
            parsed.map_err(|_| LockRegistryError::Tampered)?.to_utc(),
        ))
    }

    /// Only for the uninstaller once the lock has ended (T6.3).
    pub fn clear(&self) -> Result<(), LockRegistryError> {
        absent_to_none(self.root.remove_tree(&self.path))?;
        Ok(())
    }

    /// A store under HKCU, so tests don't need admin rights and can't touch the real lock.
    #[doc(hidden)]
    pub fn for_tests(path: String, machine_key: &str) -> Self {
        Self {
            root: CURRENT_USER,
            path,
            machine_key: machine_key.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// A unique test key, removed when the test ends, pass or fail.
    struct TestStore(LockRegistry);

    impl TestStore {
        fn new(name: &str) -> Self {
            let path = format!(r"SOFTWARE\MonkModeTest-{}-{name}", std::process::id());
            Self(LockRegistry::for_tests(path, "machine-a"))
        }

        fn key(&self) -> Key {
            self.0.root.create(&self.0.path).unwrap()
        }
    }

    impl Drop for TestStore {
        fn drop(&mut self) {
            let _ = self.0.clear();
        }
    }

    fn end() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2027, 2, 3, 12, 0, 0).unwrap()
    }

    #[test]
    fn round_trips() {
        let store = TestStore::new("round");
        store.0.write(end()).unwrap();
        assert_eq!(store.0.read().unwrap(), Some(end()));
    }

    #[test]
    fn nothing_stored_reads_as_none() {
        let store = TestStore::new("none");
        assert_eq!(store.0.read().unwrap(), None);
        store.key();
        assert_eq!(store.0.read().unwrap(), None);
    }

    #[test]
    fn clear_removes_it_and_is_safe_to_repeat() {
        let store = TestStore::new("clear");
        store.0.write(end()).unwrap();
        store.0.clear().unwrap();
        store.0.clear().unwrap();
        assert_eq!(store.0.read().unwrap(), None);
    }

    #[test]
    fn an_edited_date_is_rejected() {
        let store = TestStore::new("edited");
        store.0.write(end()).unwrap();
        store
            .key()
            .set_string(ENDS_AT, "2026-01-01T00:00:00+00:00")
            .unwrap();
        assert!(matches!(store.0.read(), Err(LockRegistryError::Tampered)));
    }

    #[test]
    fn an_edited_signature_is_rejected() {
        let store = TestStore::new("sig");
        store.0.write(end()).unwrap();
        store.key().set_string(SIGNATURE, "00").unwrap();
        assert!(matches!(store.0.read(), Err(LockRegistryError::Tampered)));
    }

    #[test]
    fn half_a_copy_is_rejected() {
        let store = TestStore::new("half");
        store.0.write(end()).unwrap();
        store.key().remove_value(SIGNATURE).unwrap();
        assert!(matches!(store.0.read(), Err(LockRegistryError::Tampered)));
    }

    #[test]
    fn a_copy_from_another_machine_is_rejected() {
        let store = TestStore::new("machine");
        store.0.write(end()).unwrap();
        let other = LockRegistry::for_tests(store.0.path.clone(), "machine-b");
        assert!(matches!(other.read(), Err(LockRegistryError::Tampered)));
    }

    #[test]
    fn sign_and_verify_agree_for_the_database_copy() {
        let store = TestStore::new("verify");
        let signature = store.0.sign(end());
        assert!(store.0.verify(end(), &signature));
        assert!(!store
            .0
            .verify(end() - chrono::Duration::days(1), &signature));
        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn the_machine_key_can_be_read_without_admin_rights() {
        let real = LockRegistry::machine().unwrap();
        assert!(!real.machine_key.is_empty());
        assert_eq!(real.path, KEY_PATH);
    }
}
