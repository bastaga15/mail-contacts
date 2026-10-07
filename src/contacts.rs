//! Agrégation des messages en un carnet de contacts dédoublonné.

use std::collections::HashMap;

use mail_parser::{Address, Message};

use crate::{names, signature};

/// Date calendaire, comparable et affichable en `AAAA-MM-JJ`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Day {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl std::fmt::Display for Day {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl std::str::FromStr for Day {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = text.split('-').collect();
        let parse = || -> Option<Day> {
            let [year, month, day] = parts.as_slice() else { return None };
            Some(Day { year: year.parse().ok()?, month: month.parse().ok()?, day: day.parse().ok()? })
        };
        parse()
            .filter(|d| (1..=12).contains(&d.month) && (1..=31).contains(&d.day))
            .ok_or_else(|| format!("date invalide « {text} », format attendu : AAAA-MM-JJ"))
    }
}

#[derive(Debug, Default, Clone)]
pub struct Contact {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub company: String,
    /// Adresse partagée (contact@, compta@) plutôt qu'une personne.
    pub is_role: bool,
    /// Messages reçus de ce contact.
    pub received: u32,
    /// Messages qui lui ont été envoyés, en destinataire ou en copie.
    pub sent: u32,
    pub last_contact: Option<Day>,
    /// Signature du message le plus récent qui en contient une.
    pub signature: String,
    signature_day: Option<Day>,
}

impl Contact {
    pub fn exchanges(&self) -> u32 {
        self.received + self.sent
    }
}

#[derive(Debug, Default)]
pub struct Stats {
    pub messages: u32,
    /// Messages sans expéditeur lisible ou sans date exploitable.
    pub skipped: u32,
    /// Messages antérieurs à la date demandée.
    pub too_old: u32,
}

pub struct Options {
    /// Adresses du propriétaire de la boîte, en minuscules.
    pub own_emails: Vec<String>,
    pub since: Option<Day>,
    /// Nom d'entreprise imposé pour un domaine.
    pub company_overrides: HashMap<String, String>,
}

#[derive(Default)]
pub struct Directory {
    contacts: HashMap<String, Contact>,
    pub stats: Stats,
}

fn addresses<'a>(field: Option<&'a Address<'a>>) -> Vec<(Option<&'a str>, String)> {
    field
        .into_iter()
        .flat_map(|address| address.iter())
        .filter_map(|addr| {
            let email = addr.address.as_deref()?.trim().to_lowercase();
            email.contains('@').then_some((addr.name.as_deref(), email))
        })
        .collect()
}

impl Directory {
    pub fn add_message(&mut self, message: &Message, options: &Options) {
        self.stats.messages += 1;

        let senders = addresses(message.from());
        let (Some((sender_name, sender)), Some(date)) = (senders.first(), message.date()) else {
            self.stats.skipped += 1;
            return;
        };
        let day = Day { year: date.year, month: date.month, day: date.day };
        if options.since.is_some_and(|since| day < since) {
            self.stats.too_old += 1;
            return;
        }

        if options.own_emails.contains(sender) {
            // Message envoyé : chaque destinataire est un contact.
            let recipients = addresses(message.to()).into_iter().chain(addresses(message.cc()));
            for (name, email) in recipients {
                if let Some(contact) = self.touch(&email, name, day, options) {
                    contact.sent += 1;
                }
            }
        } else if let Some(contact) = self.touch(sender, *sender_name, day, options) {
            // Message reçu : seul l'expéditeur compte, les autres destinataires
            // n'ont pas forcément échangé avec le propriétaire de la boîte.
            contact.received += 1;
            let newer = contact.signature_day.is_none_or(|known| day >= known);
            if newer {
                if let Some(found) = message.body_text(0).and_then(|body| signature::extract(&body)) {
                    contact.signature = found;
                    contact.signature_day = Some(day);
                }
            }
        }
    }

    /// Crée ou met à jour le contact. `None` pour le propriétaire et les expéditeurs automatiques.
    fn touch(&mut self, email: &str, display: Option<&str>, day: Day, options: &Options) -> Option<&mut Contact> {
        if options.own_emails.iter().any(|own| own == email) || names::is_automated(email) {
            return None;
        }
        let contact = self.contacts.entry(email.to_string()).or_insert_with(|| {
            let domain = names::domain(email);
            Contact {
                email: email.to_string(),
                company: options
                    .company_overrides
                    .get(domain)
                    .cloned()
                    .unwrap_or_else(|| names::company_from_domain(domain)),
                is_role: names::is_role_address(email),
                ..Contact::default()
            }
        });
        // Un même contact apparaît sous plusieurs formes : on garde le premier nom exploitable.
        if contact.first_name.is_empty() && contact.last_name.is_empty() && !contact.is_role {
            let name = names::parse_name(display, email);
            contact.first_name = name.first;
            contact.last_name = name.last;
        }
        contact.last_contact = contact.last_contact.max(Some(day));
        Some(contact)
    }

    /// Contacts triés par nombre d'échanges décroissant, puis par adresse.
    pub fn into_sorted(self) -> (Vec<Contact>, Stats) {
        let mut contacts: Vec<Contact> = self.contacts.into_values().collect();
        contacts.sort_by(|a, b| b.exchanges().cmp(&a.exchanges()).then_with(|| a.email.cmp(&b.email)));
        (contacts, self.stats)
    }
}
