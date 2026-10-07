//! Test de bout en bout sur une boîte fictive.

use std::process::Command;

fn run(extra: &[&str]) -> Vec<Vec<String>> {
    let output = std::env::temp_dir().join(format!("mail-contacts-{}-{}.csv", std::process::id(), extra.len()));
    let status = Command::new(env!("CARGO_BIN_EXE_mail-contacts"))
        .arg("tests/fixtures/sample.mbox")
        .args(["--own-email", "camille.martin@mon-entreprise.example"])
        .args(extra)
        .arg("--output")
        .arg(&output)
        .status()
        .expect("lancement du binaire");
    assert!(status.success());

    let content = std::fs::read_to_string(&output).expect("lecture du CSV");
    std::fs::remove_file(&output).ok();
    let mut reader = csv::Reader::from_reader(content.trim_start_matches('\u{feff}').as_bytes());
    reader
        .records()
        .map(|record| record.unwrap().iter().map(str::to_string).collect())
        .collect()
}

fn row<'a>(rows: &'a [Vec<String>], email: &str) -> &'a Vec<String> {
    rows.iter().find(|row| row[0] == email).unwrap_or_else(|| panic!("{email} absent"))
}

#[test]
fn dedoublonne_et_compte_les_echanges() {
    let rows = run(&[]);
    let emails: Vec<&str> = rows.iter().map(|row| row[0].as_str()).collect();

    // Le contact le plus actif vient en premier ; ni le propriétaire ni l'expéditeur automatique.
    assert_eq!(emails[0], "alice.durand@fournisseur-exemple.fr");
    assert!(!emails.contains(&"camille.martin@mon-entreprise.example"));
    assert!(!emails.contains(&"no-reply@outil-exemple.com"));
    assert_eq!(rows.len(), 4);

    // Deux messages reçus (dont un avec une casse différente) et un envoyé.
    let alice = row(&rows, "alice.durand@fournisseur-exemple.fr");
    assert_eq!(&alice[1..=8], ["Alice", "Durand", "Fournisseur Exemple", "personne", "3", "2", "1", "2026-03-05"]);
    // La signature retenue est celle du message le plus récent.
    assert_eq!(alice[9], "Alice Durand | Directrice commerciale");

    let role = row(&rows, "contact@fournisseur-exemple.fr");
    assert_eq!(role[4], "adresse partagée");
    assert_eq!(row(&rows, "bob.leroy@gmail.com")[3], "Personnel");
    assert_eq!(&row(&rows, "paul.petit@client-exemple.com")[1..=2], ["Paul", "Petit"]);
}

#[test]
fn filtre_par_date_et_nom_d_entreprise_impose() {
    let rows = run(&["--since", "2026-02-01", "--company", "fournisseur-exemple.fr=Fournisseur Exemple SAS"]);
    assert!(rows.iter().all(|row| row[0] != "paul.petit@client-exemple.com"));
    assert_eq!(row(&rows, "alice.durand@fournisseur-exemple.fr")[3], "Fournisseur Exemple SAS");
}
