use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use inquire::{Confirm, Password};
use qrcode::{render::unicode, QrCode};
use termtree::Tree;
use totp_rs::TOTP;
use tracing_log::AsTrace;
use tracing_subscriber::util::SubscriberInitExt;

use keyage::{Error, Result, Store};

////////////////////////////////////////////////////////////////////////////////
///////////////////////////////////// CLAP /////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
#[derive(Parser)]
#[command(
    version,
    about = include_str!("../docs/about.txt").trim(),
    long_about = include_str!("../docs/long_about.txt").trim()
)]
struct Cli {
    #[command(flatten)]
    verbose: Verbosity,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the keyage store directory (uses KEYAGE_STORE variable)
    Initialize { path_to_secret_key: PathBuf },

    /// List all the passwords in the store
    List,

    /// Add a password to the password store
    Insert {
        /// The path of the password relative to the root of the store
        path: PathBuf,

        /// Forcefully insert the password into the store.
        #[arg(short, long, action)]
        force: bool,
    },

    /// Does the same as insert, but it generates the password for you
    Generate {
        /// The path of the password relative to the root of the store
        path: PathBuf,

        /// The length of the password to generate (min. 8)
        length: usize,

        /// Ignore symbols, so only use numbers and letters
        #[arg(short, long, action)]
        no_symbols: bool,

        /// This setting will yet you overwrite it when the password already exists
        #[arg(short, long, action)]
        force: bool,
    },

    /// Remove a password from the store, always works recursively, but not forcefully
    Remove {
        /// The path of the password relative to the root of the store
        path: PathBuf,

        #[arg(short, long, action)]
        force: bool,
    },

    /// Show a password to the user
    Show {
        path: PathBuf,

        /// Show the password represented as a qr code
        #[arg(long, action)]
        qr: bool,

        /// Enable one time password mode
        #[arg(long, action)]
        otp: bool,
    },
}

////////////////////////////////////////////////////////////////////////////////
///////////////////////////////////// MAIN /////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
fn main() -> Result<()> {
    let cli = Cli::parse();

    let subscriber = tracing_subscriber::fmt()
        .with_line_number(false)
        .with_target(false)
        .without_time()
        .with_max_level(cli.verbose.log_level_filter().as_trace())
        .finish();

    subscriber.init();

    match cli.command {
        Commands::List {} => list()?,
        Commands::Remove { path, force } => remove(path, force)?,
        Commands::Initialize { path_to_secret_key } => initialize(path_to_secret_key)?,
        Commands::Insert { path, force } => insert(path, force)?,
        Commands::Generate {
            path,
            length,
            no_symbols,
            force,
        } => generate(path, length, no_symbols, force)?,
        Commands::Show { qr, otp, path } => show(path, qr, otp)?,
    };

    Ok(())
}

////////////////////////////////////////////////////////////////////////////////
/////////////////////////////////// HELPERS ////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
fn label<P: AsRef<Path>>(p: P) -> String {
    p.as_ref().file_name().unwrap().to_str().unwrap().to_owned()
}
fn tree<P: AsRef<Path>>(p: P) -> Result<Tree<String>> {
    let result = fs::read_dir(&p)?.filter_map(|e| e.ok()).fold(
        Tree::new(label(p.as_ref().canonicalize()?)),
        |mut root, entry| {
            let dir = entry.metadata().unwrap();
            if dir.is_dir() {
                root.push(tree(entry.path()).unwrap());
            } else {
                root.push(Tree::new(label(entry.path())));
            }
            root
        },
    );

    Ok(result)
}

////////////////////////////////////////////////////////////////////////////////
/////////////////////////////////// COMMANDS ///////////////////////////////////
////////////////////////////////////////////////////////////////////////////////
fn list() -> Result<()> {
    let store = Store::get()?;
    print!("{}", tree(store.root_path)?);
    Ok(())
}

fn show(path: PathBuf, qr: bool, otp: bool) -> Result<()> {
    let store = Store::get()?;

    if !store.is_password_in_store(path.clone())? {
        return Err(Error::Todo.into());
    }

    let encrypted = store.get_store_contents(path)?;
    let mut decrypted = store.decrypt(encrypted)?;

    if otp {
        let totp = TOTP::from_url_unchecked(String::from_utf8(decrypted)?)?;
        decrypted = totp.generate_current()?.into();
    }

    if qr {
        let code = QrCode::new(decrypted)?;
        let s = code
            .render::<unicode::Dense1x2>()
            .light_color(unicode::Dense1x2::Light)
            .dark_color(unicode::Dense1x2::Dark)
            .build();

        print!("{s}");
    } else {
        let password = String::from_utf8(decrypted)?;
        println!("{password}");
    };

    Ok(())
}

fn generate(path: PathBuf, length: usize, no_symbols: bool, force: bool) -> Result<()> {
    if length < 8 {
        return Err(Error::Todo.into());
    }

    let store = Store::get()?;

    let full_path = store.prepare_path(path.clone());

    if full_path.exists() && !force {
        return Err(Error::Todo.into());
    }

    let pg = passwords::PasswordGenerator {
        length,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: !no_symbols,
        exclude_similar_characters: true,
        strict: true,
        spaces: false,
    };

    let password = pg.generate_one()?;
    let encrypted = store.encrypt(password.clone())?;
    store.update_content(path, encrypted)?;

    println!("{password}");

    Ok(())
}

fn insert(path: PathBuf, force: bool) -> Result<()> {
    let store = Store::get()?;

    if store.is_password_in_store(path.clone())? && !force {
        return Err(Error::Todo.into());
    }

    let password = Password::new("Enter a password:")
        .with_display_toggle_enabled()
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()?;

    let encrypted = store.encrypt(password)?;
    store.update_content(path, encrypted)?;

    Ok(())
}

fn initialize(_path_to_secret_key: PathBuf) -> Result<()> {
    todo!();
}

fn remove(path: PathBuf, _force: bool) -> keyage::Result<()> {
    let store = Store::get()?;

    if !store.valid_path_in_store(path.clone())? {
        return Err(Error::Todo.into());
    }

    // Now get the confirmation from the user, then remove the password
    if Confirm::new(
        format!(
            "Are you sure you want to remove this password ({:?})?",
            path
        )
        .as_str(),
    )
    .with_default(false)
    .prompt()?
    {
        store.remove_from_store(path)?;
    }

    Ok(())
}
