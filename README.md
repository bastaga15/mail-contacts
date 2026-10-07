# mail-contacts

Extrait un carnet de contacts dédoublonné d'une archive mail au format mbox : qui a écrit, combien de fois, quand pour la dernière fois, et avec quelle signature.

```
$ mail-contacts export/ --own-email camille.martin@mon-entreprise.example
1 fichier(s), 5 messages lus (0 illisibles, 0 antérieurs à la date demandée)
4 contacts écrits dans contacts.csv
```

| email | prenom | nom | entreprise | type | echanges | dernier_contact | signature |
|---|---|---|---|---|---|---|---|
| alice.durand@fournisseur-exemple.fr | Alice | Durand | Fournisseur Exemple | personne | 3 | 2026-03-05 | Alice Durand \| Directrice commerciale |
| bob.leroy@gmail.com | Bob | Leroy | Personnel | personne | 1 | 2026-03-03 | |
| contact@fournisseur-exemple.fr | | | Fournisseur Exemple | adresse partagée | 1 | 2026-03-03 | |

## Le problème

Une entreprise qui met en place un CRM part rarement de zéro : ses contacts existent déjà, dispersés dans des années de messagerie. Les ressaisir à la main prend des jours et en oublie la moitié.

L'accès direct à la boîte (IMAP) est souvent fermé par le service informatique. Un export d'archive reste en général possible, et c'est ce que lit cet outil.

## Fonctionnement

1. Parcourt un fichier mbox ou un dossier d'export, en ignorant corbeille, brouillons et indésirables.
2. Pour un message reçu, retient l'expéditeur. Pour un message envoyé, retient les destinataires et les personnes en copie.
3. Dédoublonne par adresse, sans tenir compte de la casse, et écarte les expéditeurs automatiques (`no-reply`, notifications).
4. Nettoie les noms : `DURAND Alice`, `Durand, Alice` et les identifiants Exchange illisibles sont ramenés à un prénom et un nom, à défaut déduits de la forme `prenom.nom` de l'adresse.
5. Déduit l'entreprise du domaine, signale les adresses partagées (`contact@`, `compta@`) et relève la signature du message le plus récent.
6. Écrit un CSV trié par nombre d'échanges, lisible directement dans Excel.

Tout se passe sur la machine : aucun message ne quitte le poste.

## Installation

Prérequis : [Rust](https://rustup.rs) 1.82 ou plus.

```bash
git clone https://github.com/bastaga15/mail-contacts.git
cd mail-contacts
cargo install --path .
```

## Utilisation

```bash
mail-contacts <fichier ou dossier> --own-email <adresse> [options]
```

| Option | Rôle |
|---|---|
| `--own-email` | Adresse du propriétaire de la boîte. À répéter s'il en a plusieurs. |
| `--since AAAA-MM-JJ` | Ignore les messages plus anciens. |
| `--company domaine=Nom` | Impose le nom d'une entreprise pour un domaine. |
| `-o, --output` | Fichier de sortie (`contacts.csv` par défaut). |

Pour une archive Outlook `.pst`, convertissez-la d'abord avec [readpst](https://www.five-ten-sg.com/libpst/) : `readpst -r -o export/ archive.pst`.

## Tests

```bash
cargo test
```

Les tests s'appuient sur une boîte entièrement fictive (`tests/fixtures/sample.mbox`).

## Limites connues

- La fonction de la personne n'est pas déduite : la signature est exportée telle quelle. C'est le point de départ naturel d'un enrichissement par un modèle de langage.
- Le format `.pst` n'est pas lu directement.
- L'entreprise vient du domaine : `groupe-exemple.fr` donne « Groupe Exemple », ce qui est une approximation. Les domaines à extension double (`.co.uk`) ne sont pas gérés.
- La détection de signature repose sur le délimiteur `-- ` et sur les formules de politesse courantes en français et en anglais.

## Origine

Cet outil reprend, en Rust et sans aucune donnée réelle, la démarche d'une mission décrite dans [cet article](https://lectech.fr/actualites/enrichir-contacts-mail).

## Licence

MIT, voir `LICENSE`.

Bastien Lechat, [LecTech](https://lectech.fr)
