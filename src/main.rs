//! Extrait un carnet de contacts dédoublonné d'une archive mail au format mbox.

mod contacts;
mod names;
mod signature;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Parser;
use mail_parser::mailbox::mbox::MessageIterator;
use mail_parser::MessageParser;
use walkdir::WalkDir;

use contacts::{Contact, Day, Directory, Options};

/// Dossiers ignorés quand le chemin fourni est un répertoire.
const SKIPPED_FOLDERS: &[&str] = &[
    "supprime", "corbeille", "trash", "deleted", "spam", "junk", "indesirable", "draft", "brouillon",
];

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Fichier mbox, ou dossier parcouru récursivement (export de readpst par exemple)
    path: PathBuf,

    /// Adresse du propriétaire de la boîte ; à répéter s'il en a plusieurs
    #[arg(long = "own-email", required = true)]
    own_emails: Vec<String>,

    /// Ignore les messages antérieurs à cette date (AAAA-MM-JJ)
    #[arg(long)]
    since: Option<Day>,

    /// Impose le nom d'une entreprise pour un domaine : `--company exemple.fr="Exemple SAS"`
    #[arg(long = "company", value_parser = parse_override)]
    companies: Vec<(String, String)>,

    /// Fichier CSV de sortie
    #[arg(long, short, default_value = "contacts.csv")]
    output: PathBuf,
}

fn parse_override(text: &str) -> Result<(String, String), String> {
    match text.split_once('=') {
        Some((domain, name)) if !domain.is_empty() && !name.is_empty() => {
            Ok((domain.trim().to_lowercase(), name.trim().to_string()))
        }
        _ => Err(format!("« {text} » : format attendu domaine=Nom")),
    }
}

fn is_skipped(path: &Path) -> bool {
    path.components().any(|component| {
        let folded = names::fold(&component.as_os_str().to_string_lossy());
        SKIPPED_FOLDERS.iter().any(|skipped| folded.contains(skipped))
    })
}

/// Un fichier seul est lu tel quel. Dans un dossier, on retient les fichiers nommés
/// `mbox` (convention de readpst) ou portant l'extension `.mbox`.
fn find_mailboxes(path: &Path) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if !path.is_dir() {
        bail!("{} n'existe pas", path.display());
    }
    let mut found = Vec::new();
    for entry in WalkDir::new(path) {
        let entry = entry.with_context(|| format!("lecture de {}", path.display()))?;
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let relative = entry.path().strip_prefix(path).unwrap_or(entry.path());
        if entry.file_type().is_file() && (name == "mbox" || name.ends_with(".mbox")) && !is_skipped(relative) {
            found.push(entry.into_path());
        }
    }
    found.sort();
    Ok(found)
}

fn read_mailbox(path: &Path, directory: &mut Directory, options: &Options) -> Result<()> {
    let file = File::open(path).with_context(|| format!("ouverture de {}", path.display()))?;
    let parser = MessageParser::default();
    for raw in MessageIterator::new(BufReader::new(file)) {
        // Un message illisible ne doit pas faire perdre le reste de l'archive.
        let Ok(raw) = raw else {
            directory.stats.messages += 1;
            directory.stats.skipped += 1;
            continue;
        };
        match parser.parse(raw.contents()) {
            Some(message) => directory.add_message(&message, options),
            None => {
                directory.stats.messages += 1;
                directory.stats.skipped += 1;
            }
        }
    }
    Ok(())
}

fn write_csv(contacts: &[Contact], output: &Path) -> Result<()> {
    let mut file = File::create(output).with_context(|| format!("création de {}", output.display()))?;
    // Marque d'ordre d'octets : sans elle, Excel lit les accents de travers.
    file.write_all("\u{feff}".as_bytes())?;
    let mut writer = csv::Writer::from_writer(file);
    writer.write_record([
        "email", "prenom", "nom", "entreprise", "type", "echanges", "recus", "envoyes",
        "dernier_contact", "signature",
    ])?;
    for contact in contacts {
        writer.write_record([
            contact.email.as_str(),
            contact.first_name.as_str(),
            contact.last_name.as_str(),
            contact.company.as_str(),
            if contact.is_role { "adresse partagée" } else { "personne" },
            &contact.exchanges().to_string(),
            &contact.received.to_string(),
            &contact.sent.to_string(),
            &contact.last_contact.map(|day| day.to_string()).unwrap_or_default(),
            contact.signature.as_str(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let options = Options {
        own_emails: cli.own_emails.iter().map(|email| email.trim().to_lowercase()).collect(),
        since: cli.since,
        company_overrides: cli.companies.into_iter().collect::<HashMap<_, _>>(),
    };

    let mailboxes = find_mailboxes(&cli.path)?;
    if mailboxes.is_empty() {
        bail!("aucun fichier mbox trouvé dans {}", cli.path.display());
    }

    let mut directory = Directory::default();
    for mailbox in &mailboxes {
        read_mailbox(mailbox, &mut directory, &options)?;
    }

    let (contacts, stats) = directory.into_sorted();
    write_csv(&contacts, &cli.output)?;

    eprintln!(
        "{} fichier(s), {} messages lus ({} illisibles, {} antérieurs à la date demandée)",
        mailboxes.len(),
        stats.messages,
        stats.skipped,
        stats.too_old
    );
    eprintln!("{} contacts écrits dans {}", contacts.len(), cli.output.display());
    Ok(())
}
