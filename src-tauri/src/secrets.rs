//! API keys, kept in the operating system's credential store.
//!
//! They used to live in `settings_store.json` in plain text. That was defensible
//! on a single-user machine — whoever reaches that directory also reaches the
//! dictation history — but it is not a protection anyone should rely on, and
//! Murmel_Northstar.md §5.1 said "encrypted" when it was not.
//!
//! Rolling our own encryption would only move the problem: the key to decrypt
//! with has to live somewhere too. The system store is where credentials
//! belong, with whatever protection the platform provides (DPAPI on Windows,
//! the Secret Service on Linux).
//!
//! When the store is unavailable — a Linux session without a running keyring
//! daemon, say — Murmel reports it rather than falling back to a file. Silently
//! writing a key in clear text after promising otherwise would be worse than
//! refusing.

use keyring::Entry;
use log::{debug, warn};

/// Service name under which the entries appear in the system store. Chosen to
/// be recognisable in Windows' Credential Manager and GNOME's Seahorse.
const SERVICE: &str = "Murmel";

fn entry(provider_id: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, provider_id)
        .map_err(|err| format!("The system credential store is unavailable: {err}"))
}

/// Store (or replace) the key for a provider.
pub fn set_api_key(provider_id: &str, key: &str) -> Result<(), String> {
    let entry = entry(provider_id)?;

    if key.trim().is_empty() {
        return delete_api_key(provider_id);
    }

    entry
        .set_password(key)
        .map_err(|err| format!("Could not save the key: {err}"))?;

    debug!("Stored API key for provider '{provider_id}'");
    Ok(())
}

/// Read a provider's key. `None` when none is stored.
///
/// A store that cannot be reached is reported as "no key" rather than an error:
/// the caller's next step is a request that would fail anyway, and the settings
/// screen surfaces the real problem when saving.
pub fn get_api_key(provider_id: &str) -> Option<String> {
    match entry(provider_id) {
        Ok(entry) => match entry.get_password() {
            Ok(key) => Some(key),
            Err(keyring::Error::NoEntry) => None,
            Err(err) => {
                warn!("Could not read API key for '{provider_id}': {err}");
                None
            }
        },
        Err(err) => {
            warn!("{err}");
            None
        }
    }
}

/// Whether a key is stored — without reading it.
///
/// This is what the UI asks: it shows "a key is stored", never the key itself.
pub fn has_api_key(provider_id: &str) -> bool {
    get_api_key(provider_id).is_some_and(|key| !key.trim().is_empty())
}

/// Remove a provider's key. Removing one that is not there is not an error.
pub fn delete_api_key(provider_id: &str) -> Result<(), String> {
    let entry = entry(provider_id)?;

    match entry.delete_credential() {
        Ok(()) => {
            debug!("Deleted API key for provider '{provider_id}'");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(format!("Could not delete the key: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip against the real store. Uses a provider id no real provider
    /// has, so a developer's actual keys are never touched.
    #[test]
    fn stores_reads_and_deletes_a_key() {
        let provider = "murmel-test-provider";

        // A leftover from a failed run would make the assertions meaningless.
        let _ = delete_api_key(provider);

        if set_api_key(provider, "sk-test-value").is_err() {
            // No credential store in this environment (headless CI, for
            // example). The code path cannot be exercised, and failing here
            // would report a broken build instead of a missing daemon.
            return;
        }

        assert!(has_api_key(provider));
        assert_eq!(get_api_key(provider).as_deref(), Some("sk-test-value"));

        delete_api_key(provider).expect("delete");
        assert!(!has_api_key(provider));
        assert_eq!(get_api_key(provider), None);
    }

    /// Saving an empty value means "remove it", so the UI does not need a
    /// separate call for a cleared field.
    #[test]
    fn an_empty_key_removes_the_entry() {
        let provider = "murmel-test-provider-empty";
        let _ = delete_api_key(provider);

        if set_api_key(provider, "sk-test").is_err() {
            return;
        }

        set_api_key(provider, "   ").expect("empty clears");
        assert!(!has_api_key(provider));
    }
}
