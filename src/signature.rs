//! Repérage du bloc de signature dans le corps d'un message.

use crate::names::fold;

const MAX_LINES: usize = 6;

/// Formules de politesse qui précèdent en général la signature.
const CLOSINGS: &[&str] = &[
    "cordialement", "bien cordialement", "bien a vous", "salutations", "sinceres salutations",
    "bonne journee", "bonne reception", "merci d'avance", "best regards", "kind regards", "regards",
];

/// Marques du début d'un message cité : la signature se trouve avant.
fn starts_quote(line: &str) -> bool {
    let folded = fold(line);
    line.starts_with('>')
        || folded.starts_with("-----original message")
        || folded.starts_with("-----message d'origine")
        || folded.starts_with("de :")
        || folded.starts_with("from:")
        || (folded.starts_with("le ") && folded.ends_with("a ecrit :"))
        || (folded.starts_with("on ") && folded.ends_with("wrote:"))
}

/// Renvoie les lignes qui suivent le délimiteur `-- ` ou la dernière formule de
/// politesse, sans le message cité. `None` si aucun repère n'est trouvé.
pub fn extract(body: &str) -> Option<String> {
    let lines: Vec<&str> = body
        .lines()
        .map(str::trim_end)
        .take_while(|line| !starts_quote(line.trim_start()))
        .collect();

    let delimiter = lines.iter().rposition(|line| *line == "--" || *line == "-- ");
    let closing = lines.iter().rposition(|line| {
        let folded = fold(line.trim().trim_end_matches([',', '.', '!']));
        CLOSINGS.contains(&folded.as_str())
    });
    let start = delimiter.or(closing)? + 1;

    let signature: Vec<&str> = lines[start..]
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .take(MAX_LINES)
        .collect();

    (!signature.is_empty()).then(|| signature.join(" | "))
}

#[cfg(test)]
mod tests {
    use super::extract;

    #[test]
    fn apres_une_formule_de_politesse() {
        let body = "Bonjour,\n\nVoici le devis.\n\nCordialement,\nAlice Durand\nResponsable commerciale\n";
        assert_eq!(extract(body).as_deref(), Some("Alice Durand | Responsable commerciale"));
    }

    #[test]
    fn apres_le_delimiteur_standard() {
        let body = "Texte.\n\n-- \nAlice Durand\nDirectrice commerciale";
        assert_eq!(extract(body).as_deref(), Some("Alice Durand | Directrice commerciale"));
    }

    #[test]
    fn le_message_cite_est_exclu() {
        let body = "Merci.\n\nCordialement,\nAlice\n\nLe 2 mars 2026, Camille a écrit :\n> Bonjour\n> Cordialement,\n> Camille Martin";
        assert_eq!(extract(body).as_deref(), Some("Alice"));
    }

    #[test]
    fn aucun_repere() {
        assert_eq!(extract("Ok pour moi."), None);
    }
}
