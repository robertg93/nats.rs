use std::{fs, io, path::Path};

use nkeys::KeyPair;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::SecureString;

/// Loads the user JWT and nkey from a `.creds` file.
pub(crate) fn load_creds(path: &Path) -> io::Result<(SecureString, KeyPair)> {
    // Load the private nkey.
    let contents = SecureString::from(fs::read_to_string(path)?);

    let jwt = parse_decorated_jwt(&contents).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "cannot parse user JWT from the credentials file",
        )
    })?;

    let nkey = parse_decorated_nkey(&contents).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "cannot parse nkey from the credentials file",
        )
    })?;
    let kp =
        KeyPair::from_seed(&nkey).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    Ok((jwt, kp))
}

/// Loads the nkey from a `.nk` file.
pub(crate) fn load_nk(path: &Path) -> io::Result<KeyPair> {
    let contents = SecureString::from(fs::read_to_string(path)?);

    for line in contents.lines() {
        let line = line.trim();

        if line.starts_with("SO") || line.starts_with("SA") || line.starts_with("SU") {
            return KeyPair::from_seed(line)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err));
        }
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "no nkey seed found",
    ))
}

/// Signs nonce using a credentials file.
pub(crate) fn sign_nonce(nonce: &[u8], key_pair: &KeyPair) -> io::Result<SecureString> {
    // Use the nkey to sign the nonce.
    let sig = key_pair
        .sign(nonce)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    // Encode the signature to Base64URL.
    Ok(SecureString::from(base64_url::encode(&sig)))
}

// This regex parses a credentials file.
//
// The credentials file is typically `~/.nkeys/creds/synadia/<account/<account>.creds`
// and looks like this:
//
// ```
// -----BEGIN NATS USER JWT-----
// eyJ0eXAiOiJqd3QiLCJhbGciOiJlZDI1NTE5...
// ------END NATS USER JWT------
//
// ************************* IMPORTANT *************************
// NKEY Seed printed below can be used sign and prove identity.
// NKEYs are sensitive and should be treated as secrets.
//
// -----BEGIN USER NKEY SEED-----
// SUAIO3FHUX5PNV2LQIIP7TZ3N4L7TX3W53MQGEIVYFIGA635OZCKEYHFLM
// ------END USER NKEY SEED------
// ```
static USER_CONFIG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\s*(?:(?:[-]{3,}.*[-]{3,}\r?\n)([\w\-.=]+)(?:\r?\n[-]{3,}.*[-]{3,}\r?\n))")
        .unwrap()
});

/// Parses a credentials file and returns its user JWT.
fn parse_decorated_jwt(contents: &SecureString) -> Option<SecureString> {
    let capture = USER_CONFIG_RE.captures_iter(contents).next()?;
    Some(SecureString::from(capture[1].to_string()))
}

/// Parses a credentials file and returns its nkey.
fn parse_decorated_nkey(contents: &SecureString) -> Option<SecureString> {
    let capture = USER_CONFIG_RE.captures_iter(contents).nth(1)?;
    Some(SecureString::from(capture[1].to_string()))
}
