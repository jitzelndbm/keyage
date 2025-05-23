use crate::error::{Error, Result};

use std::{
    env,
    fs::{self},
    io::{Read, Write},
    iter,
    path::PathBuf,
    str::FromStr,
};

use age::{
    cli_common::{read_identities, StdinGuard},
    Encryptor, Recipient,
};
use log::{debug, info, trace};
use serde_derive::{Deserialize, Serialize};

pub mod error;

////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub identifier: Option<String>,
    pub recipient: Option<String>,
}

impl ::std::default::Default for Configuration {
    fn default() -> Self {
        Self {
            identifier: None,
            recipient: None,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Store {
    pub root_path: PathBuf,
    pub identity_file_path: String,
    pub recipient_file_path: String,
    // TODO: Git integration
    // TODO: repository: Repository,
}

impl Store {
    pub const CONFIG_FILE_NAME: &'static str = "config.toml";
    pub const STORE_DIR_VAR_NAME: &'static str = "KEYAGE_STORE";
    pub const DEFAULT_STORE_DIR_NAME: &'static str = "keyage-store";

    fn get_recipient(&self) -> Result<Box<dyn Recipient + Send>> {
        if let Ok(ssh_recipient) = age::ssh::Recipient::from_str(&self.recipient_file_path) {
            Ok(Box::new(ssh_recipient))
        } else if let Ok(x25519_recipient) =
            age::x25519::Recipient::from_str(&self.recipient_file_path)
        {
            Ok(Box::new(x25519_recipient))
        } else {
            Err(Error::InvalidRecipientFormat)
        }
    }

    /// Get the default store, with a provided configuration file in the root of the store
    pub fn get() -> Result<Self> {
        let root_path = match env::var(Self::STORE_DIR_VAR_NAME).ok() {
            Some(d) => PathBuf::from(d),
            None => dirs::data_local_dir()
                .ok_or(Error::StoreNotFound(
                    "The path to the local data dir could not be found".to_string(),
                ))?
                .join(Self::DEFAULT_STORE_DIR_NAME),
        };
        debug!("Found store path: {0}", root_path.display());

        let configuration_path = root_path.join(Self::CONFIG_FILE_NAME);
        let configuration: Configuration =
            confy::load_path(configuration_path).map_err(|e| Error::ConfigLoad(e.to_string()))?;

        let recipient_file_path = configuration
            .recipient
            .ok_or(Error::ConfigLoad("Invalid recipient field".to_string()))?;
        let identity_file_path = configuration
            .identifier
            .ok_or(Error::ConfigLoad("Invalid identity field".to_string()))?;

        Ok(Self {
            root_path,
            identity_file_path,
            recipient_file_path,
        })
    }

    /// Encrypt a given string with the identity provided in the config
    pub fn encrypt(&self, text: impl Into<String>) -> Result<Vec<u8>> {
        let encryptor =
            Encryptor::with_recipients(vec![self.get_recipient()?]).ok_or(Error::AgeEncryption)?;

        let mut encrypted = Vec::new();
        let mut writer = encryptor
            .wrap_output(&mut encrypted)
            .map_err(|_| Error::AgeEncryption)?;
        writer
            .write_all(text.into().as_bytes())
            .map_err(|_| Error::AgeEncryption)?;
        writer.finish().map_err(|_| Error::AgeEncryption)?;

        Ok(encrypted)
    }

    /// Decrypt encrypted bytes with the identity provided in the config
    pub fn decrypt(&self, encrypted: Vec<u8>) -> Result<Vec<u8>> {
        let mut encrypted = encrypted.as_slice();
        let decryptor =
            match age::Decryptor::new(&mut encrypted).map_err(|_| Error::AgeDecryption)? {
                age::Decryptor::Recipients(d) => d,
                _ => unreachable!(),
            };

        // Get the identity
        let identities = read_identities(
            vec![self.identity_file_path.clone()],
            None,
            &mut StdinGuard::new(false),
        )
        .map_err(|_| Error::AgeDecryption)?;

        let identity = identities.first().ok_or(Error::AgeDecryption)?;

        let mut decrypted = vec![];
        let mut reader = decryptor
            .decrypt(iter::once(identity.as_ref()))
            .map_err(|_| Error::AgeDecryption)?;
        reader
            .read_to_end(&mut decrypted)
            .map_err(|_| Error::AgeDecryption)?;

        Ok(decrypted)
    }

    /// Get the encrypted contents from a path in the store.
    ///
    /// # Parameters
    /// - path: relative path to the store root, so "example/test", not "/home/..." etc.
    ///
    /// # Errors
    /// - If the password does not exist
    /// - If reading the file fails
    pub fn get_store_contents(&self, path: PathBuf) -> Result<Vec<u8>> {
        let pw_path = self.prepare_path(path);

        if !pw_path.exists() {
            return Err(Error::StoreNotFound(
                pw_path
                    .to_str()
                    .expect("String conversion error")
                    .to_string(),
            ));
        }

        Ok(fs::read(pw_path).map_err(|e| Error::StoreRead(e.to_string()))?)
    }

    /// This method forcefully updates the store content under path
    ///
    /// # Parameters
    /// - path: relative path to the store root, so "example/test", not "/home/..." etc.
    /// - text: the text represented as UTF-8 bytes that should be put into the store at this place
    ///
    /// # Behaviour
    /// - When the file under path does not exist yet, it is automatically created.
    /// - This is also true for the underlying directories. It's basically `mkdir -p`
    pub fn update_content(&self, path: PathBuf, encrypted: impl Into<Vec<u8>>) -> Result<()> {
        let pw_path = self.prepare_path(path);

        fs::create_dir_all(
            pw_path
                .parent()
                .ok_or(Error::StoreNotFound("/".to_string()))?,
        )
        .map_err(|e| Error::StoreWrite(e.to_string()))?;

        fs::write(pw_path, encrypted.into()).map_err(|e| Error::StoreWrite(e.to_string()))?;

        Ok(())
    }

    /// This method forcefully removes a password from the store
    ///
    /// # Parameters
    /// - path: relative path to the store root, so "example/test", not "/home/..." etc.
    pub fn remove_from_store(&self, path: PathBuf) -> Result<()> {
        let full_path = self.prepare_path(path);

        if full_path.is_dir() {
            fs::remove_dir_all(full_path)
        } else {
            fs::remove_file(full_path)
        }
        .map_err(|e| Error::StoreWrite(e.to_string()))?;

        Ok(())
    }

    /// This function accepts a relative path, and returns a full path
    pub fn prepare_path(&self, path: PathBuf) -> PathBuf {
        let mut path = self.root_path.join(path.clone());
        if path.is_dir() {
            return path;
        }
        if let Some(ex) = path.extension() {
            if ex != "age" {
                path.set_extension(ex.to_str().unwrap().to_owned() + ".age");
            }
        } else {
            path.set_extension("age");
        };
        trace!("prepare path result: {0}", path.display());
        path
    }

    /// Function accepts a relative path to the store, and returns whether or not the path is
    /// valid (dir or age file), and in the store
    pub fn valid_path_in_store(&self, path: PathBuf) -> Result<bool> {
        Ok(self.prepare_path(path).starts_with(
            self.root_path
                .canonicalize()
                .map_err(|e| Error::InvalidPath(e.to_string()))?,
        ))
    }

    /// Function accepts a relative path, and returns whether the path is a valid age file in the
    /// store
    pub fn is_password_in_store(&self, path: PathBuf) -> Result<bool> {
        let full_path = self.prepare_path(path);
        Ok(self.valid_path_in_store(full_path.clone())? && full_path.is_file())
    }
}
