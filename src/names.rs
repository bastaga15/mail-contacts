//! Nettoyage des noms affichés et déduction de l'entreprise à partir du domaine.

use unicode_normalization::UnicodeNormalization;

/// Domaines de messagerie personnelle : l'entreprise ne peut pas en être déduite.
const PERSONAL_DOMAINS: &[&str] = &[
    "gmail.com", "googlemail.com", "outlook.com", "outlook.fr", "hotmail.com", "hotmail.fr",
    "live.com", "live.fr", "yahoo.com", "yahoo.fr", "icloud.com", "me.com", "orange.fr",
    "wanadoo.fr", "free.fr", "sfr.fr", "laposte.net", "bbox.fr", "proton.me", "protonmail.com",
];

/// Partie locale typique d'une adresse partagée (contact@, compta@) plutôt que d'une personne.
const ROLE_KEYWORDS: &[&str] = &[
    "contact", "info", "infos", "hello", "bonjour", "support", "admin", "administration",
    "marketing", "ventes", "sales", "rh", "hr", "direction", "accueil", "service", "commercial",
    "compta", "comptabilite", "finance", "facturation", "billing", "achats", "logistique",
    "juridique", "communication", "presse", "sav", "secretariat", "reservation", "booking",
];

/// Expéditeurs automatiques : jamais des contacts.
const AUTOMATED_KEYWORDS: &[&str] = &[
    "noreply", "no-reply", "donotreply", "do-not-reply", "mailer-daemon", "postmaster",
    "notification", "notifications", "newsletter", "bounce",
];

pub struct PersonName {
    pub first: String,
    pub last: String,
}

fn local_part(email: &str) -> &str {
    email.split('@').next().unwrap_or(email)
}

pub fn domain(email: &str) -> &str {
    email.rsplit('@').next().unwrap_or("")
}

fn local_tokens(email: &str) -> Vec<&str> {
    local_part(email)
        .split(['.', '-', '_', '+'])
        .filter(|token| !token.is_empty())
        .collect()
}

pub fn is_automated(email: &str) -> bool {
    let local = local_part(email);
    AUTOMATED_KEYWORDS.iter().any(|keyword| local.contains(keyword))
}

/// Vrai si toute la partie locale est faite de mots de rôle (`contact`, `service-compta`).
pub fn is_role_address(email: &str) -> bool {
    let tokens = local_tokens(email);
    !tokens.is_empty() && tokens.iter().all(|token| ROLE_KEYWORDS.contains(token))
}

fn capitalize(word: &str) -> String {
    let lower = word.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Met une majuscule à chaque composante, y compris après un trait d'union (Jean-Paul).
fn title_case(text: &str) -> String {
    text.split(' ')
        .map(|word| word.split('-').map(capitalize).collect::<Vec<_>>().join("-"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_all_caps(word: &str) -> bool {
    word.chars().any(char::is_alphabetic) && !word.chars().any(char::is_lowercase)
}

/// Un nom affiché est inutilisable s'il s'agit d'un identifiant Exchange, d'un GUID
/// ou simplement de l'adresse elle-même.
fn is_unusable(display: &str) -> bool {
    let lower = display.to_lowercase();
    if lower.contains("/o=") || lower.contains("/cn=") || lower.contains("/ou=") {
        return true;
    }
    if display.contains('@') {
        return true;
    }
    let hex_run = display
        .split(|c: char| !c.is_ascii_hexdigit())
        .map(str::len)
        .max()
        .unwrap_or(0);
    hex_run >= 20
}

/// Déduit prénom et nom du nom affiché, sinon de la forme `prenom.nom` de l'adresse.
pub fn parse_name(display: Option<&str>, email: &str) -> PersonName {
    let cleaned = display
        .map(|name| name.trim().trim_matches(['"', '\'']).trim())
        .filter(|name| !name.is_empty() && !is_unusable(name));

    if let Some(name) = cleaned {
        // « Nom, Prénom » : convention fréquente dans les annuaires d'entreprise.
        if let Some((last, first)) = name.split_once(',') {
            return PersonName { first: title_case(first.trim()), last: title_case(last.trim()) };
        }
        let words: Vec<&str> = name.split_whitespace().collect();
        if words.len() >= 2 {
            // « DURAND Alice » : les mots en capitales en tête forment le nom.
            let caps = words.iter().take_while(|word| is_all_caps(word)).count();
            let (first, last) = if caps > 0 && caps < words.len() {
                (words[caps..].join(" "), words[..caps].join(" "))
            } else {
                (words[0].to_string(), words[1..].join(" "))
            };
            return PersonName { first: title_case(&first), last: title_case(&last) };
        }
    }

    let tokens = local_tokens(email);
    let alphabetic = |token: &&str| token.chars().all(char::is_alphabetic) && token.len() > 1;
    if tokens.len() == 2 && tokens.iter().all(alphabetic) && !is_role_address(email) {
        return PersonName { first: capitalize(tokens[0]), last: capitalize(tokens[1]) };
    }
    PersonName { first: String::new(), last: String::new() }
}

/// Entreprise déduite du domaine : `fournisseur-exemple.fr` donne « Fournisseur Exemple ».
pub fn company_from_domain(domain: &str) -> String {
    if PERSONAL_DOMAINS.contains(&domain) {
        return "Personnel".to_string();
    }
    let labels: Vec<&str> = domain.split('.').collect();
    // Le libellé qui précède l'extension ; `co.uk` et consorts ne sont pas gérés.
    let label = match labels.len() {
        0 => return String::new(),
        1 => labels[0],
        n => labels[n - 2],
    };
    title_case(&label.replace(['-', '_'], " "))
}

/// Minuscules sans accents, pour comparer des libellés de dossiers ou de formules.
pub fn fold(text: &str) -> String {
    text.nfd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c))
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(display: Option<&str>, email: &str) -> (String, String) {
        let parsed = parse_name(display, email);
        (parsed.first, parsed.last)
    }

    #[test]
    fn nom_en_capitales_en_tete() {
        assert_eq!(name(Some("DURAND Alice"), "a@x.fr"), ("Alice".into(), "Durand".into()));
        assert_eq!(name(Some("DE LA TOUR Jean-paul"), "a@x.fr"), ("Jean-Paul".into(), "De La Tour".into()));
    }

    #[test]
    fn nom_virgule_prenom() {
        assert_eq!(name(Some("Durand, Alice"), "a@x.fr"), ("Alice".into(), "Durand".into()));
    }

    #[test]
    fn identifiant_exchange_ignore_au_profit_de_l_adresse() {
        let display = "/O=EXEMPLE/OU=EXCHANGE/CN=RECIPIENTS/CN=4F2A9C1B7D3E4A5B8C6D7E8F9A0B1C2D-PAUL";
        assert_eq!(name(Some(display), "paul.petit@client.com"), ("Paul".into(), "Petit".into()));
    }

    #[test]
    fn adresse_sans_nom_exploitable() {
        assert_eq!(name(None, "jdoe@yahoo.fr"), (String::new(), String::new()));
        assert_eq!(name(None, "contact@x.fr"), (String::new(), String::new()));
    }

    #[test]
    fn adresses_de_role_et_automatiques() {
        assert!(is_role_address("service-compta@x.fr"));
        assert!(!is_role_address("alice.durand@x.fr"));
        assert!(is_automated("no-reply@outil.com"));
        assert!(!is_automated("alice@outil.com"));
    }

    #[test]
    fn entreprise_depuis_le_domaine() {
        assert_eq!(company_from_domain("fournisseur-exemple.fr"), "Fournisseur Exemple");
        assert_eq!(company_from_domain("mail.groupe.com"), "Groupe");
        assert_eq!(company_from_domain("gmail.com"), "Personnel");
    }
}
