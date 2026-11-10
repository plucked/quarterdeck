use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

use crate::GitHubError;

/// The Keychain item holding the token saved from the app's settings: a
/// generic password under the app's bundle identifier.
const KEYCHAIN_SERVICE: &str = "de.plucked.quarterdeck";
const KEYCHAIN_ACCOUNT: &str = "github-token";
/// `errSecItemNotFound`: no such item in the Keychain.
const ITEM_NOT_FOUND: i32 = -25300;

/// The token saved in the login Keychain, if any. macOS may ask the user
/// to allow access when the app's signature differs from the one that
/// saved it (a development build).
pub(crate) fn stored_token() -> Result<Option<String>, GitHubError> {
    match get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(bytes) => {
            let token = String::from_utf8(bytes).map_err(|_| GitHubError::Keychain {
                message: "the saved token is not text".to_owned(),
            })?;
            Ok(Some(token.trim().to_owned()).filter(|token| !token.is_empty()))
        }
        Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
        Err(error) => Err(keychain_error(error)),
    }
}

/// Saves `token` in the login Keychain, replacing a saved one.
pub fn store_token(token: &str) -> Result<(), GitHubError> {
    set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, token.trim().as_bytes())
        .map_err(keychain_error)
}

/// Removes the saved token; nothing to do when none is saved.
pub fn delete_stored_token() -> Result<(), GitHubError> {
    match delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
        Ok(()) => Ok(()),
        Err(error) if error.code() == ITEM_NOT_FOUND => Ok(()),
        Err(error) => Err(keychain_error(error)),
    }
}

fn keychain_error(error: security_framework::base::Error) -> GitHubError {
    GitHubError::Keychain {
        message: error.to_string(),
    }
}
