# RPCRad

[简体中文](README.md) | [English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

Un outil de débogage gRPC local pour ordinateur, développé en Rust avec GPUI et GPUI-KIT 0.8. Les appels réels utilisent grpcurl 1.9.3, fourni avec l’application. L’interface est native, sans WebView.

## Installation et mises à jour

Téléchargez le paquet adapté à votre système sur [GitHub Releases](https://github.com/eyotang/RPCRad/releases/latest).

| Système | Architecture | Paquet |
| --- | --- | --- |
| macOS 15+ | Apple Silicon / Intel | Fichiers `.dmg` distincts |
| Windows 10/11 | x64 | Installateur `-setup.exe` ou `.zip` portable |
| Ubuntu 24.04+ | x64 | `.deb` ou `.tar.gz` portable |
| Omarchy / Arch Linux | x64 | `.pkg.tar.zst`, Wayland et X11 |

Les paquets contiennent grpcurl, des exemples Proto et les licences des dépendances. Rust et Go ne sont pas nécessaires pour utiliser l’application. Linux exige un pilote Vulkan fonctionnel et un portail de bureau ; les langues CJK nécessitent des polices adaptées, comme `noto-fonts-cjk`. Les paquets Windows/Linux ARM64 ne sont pas disponibles.

Installez avec `sudo apt install ./rpcrad_VERSION_amd64.deb` ou `sudo pacman -U ./rpcrad-VERSION-1-x86_64.pkg.tar.zst`, en remplaçant VERSION. Décompressez intégralement les paquets portables : l’exécutable, grpcurl et resources doivent rester ensemble. Consultez le [guide d’installation (chinois)](docs/INSTALL.md).

Sur macOS, utilisez le menu système **RPCRad → À propos de RPCRad / Rechercher des mises à jour**. Sous Windows/Linux, utilisez le menu **RPCRad** de la fenêtre. Une vérification a aussi lieu au démarrage, via GitHub Releases, `latest.json` et SHA-256. Téléchargez et installez vous-même le nouveau paquet ; votre espace de travail est conservé. Aucun remplacement silencieux en arrière-plan n’est effectué.

Les paquets macOS ne sont pas encore signés Developer ID ni notariés ; les installateurs Windows ne sont pas signés Authenticode. Si la quarantaine macOS empêche le lancement, vérifiez d’abord la provenance officielle et la somme SHA-256, puis exécutez uniquement si nécessaire :

```sh
xattr -dr com.apple.quarantine /Applications/RPCRad.app
open /Applications/RPCRad.app
```

Adaptez le chemin si l’application est installée ailleurs. Il s’agit d’un dépannage facultatif, pas d’une commande de mise à jour. L’application ne l’exécute jamais automatiquement.

## Langues de l’interface

Dans **RPCRad → Paramètres… → Langue**, le menu propose **简体中文, English, 日本語, 한국어, Français, Deutsch et Русский**. Le changement est immédiat et mémorisé après redémarrage. **Automatique (système)** est le choix par défaut des nouveaux espaces et de ceux sans langue enregistrée. Au démarrage ou lors de sa sélection, la première langue système prise en charge est choisie selon l’ordre de préférence. Les variantes régionales sont associées à leur langue ; les variantes chinoises utilisent le chinois simplifié. Sans correspondance ou en cas d’échec, l’anglais est utilisé. Les choix manuels existants sont conservés. Le mode automatique affiche la langue détectée et peut être activé avec `--language=auto`. L’argument `--language=fr` permet aussi de choisir la langue ; codes acceptés : `zh-CN`, `en`, `ja`, `ko`, `fr`, `de`, `ru`.

Les paramètres s’ouvrent dans une fenêtre distincte et proposent **Suivre le système (par défaut), Clair et Sombre**. Les changements sont immédiats et enregistrés ; le mode système suit les changements d’apparence du système. La recherche de mises à jour et À propos utilisent aussi des fenêtres distinctes. Une nouvelle ouverture ramène la fenêtre existante au premier plan sans remplacer la requête. Ouvrez les paramètres avec **⌘ ,** sur macOS ou **Ctrl+,** sous Windows/Linux ; **Échap** ou **⌘/Ctrl+W** ferme la fenêtre auxiliaire.

Les messages de l’application et les descriptions des plugins intégrés sont traduits. Les noms personnalisés, valeurs des variables, données des requêtes/réponses et instantanés déjà enregistrés restent inchangés. Les messages serveur et plugins tiers sans traduction ne sont pas traduits automatiquement.

## Fonctionnalités et première requête

Importez un Proto ou utilisez la réflexion du serveur, sélectionnez une méthode, saisissez l’adresse et envoyez. **⌘ Entrée** envoie sous macOS, **Ctrl+Entrée** sous Windows/Linux. **⌘/Ctrl+K** recherche une méthode et **⌘/Ctrl+S** enregistre. Les panneaux et colonnes de l’éditeur sont redimensionnables.

- Arborescence des services et méthodes, recherche, favoris, onglets et brouillons enregistrés.
- Éditeur JSON natif avec coloration, mise en forme et exemples issus des Proto.
- Appels unary et streaming serveur, annulation, délais, état et durée.
- Métadonnées, jetons Bearer et TLS avec les CA approuvées par le système.
- Environnements, variables globales et plugins de signature d’en-têtes.
- Corps de réponse, Headers, Trailers, copie et historique de 50 appels par session.

Ouvrez un appel de l’historique pour examiner son corps réel, la connexion, les en-têtes d’origine et les métadonnées finales transmises à grpcurl. Les en-têtes ordinaires sont visibles ; chaque champ sensible possède son propre bouton d’affichage. La copie correspond à ce qui est affiché. La corbeille efface l’historique sans interrompre un appel actif. **Charger la requête** restaure adresse, corps et connexion ; les en-têtes utilisent les paramètres actuels et les plugins sont réexécutés lors de l’envoi.

Un échec ou une annulation avant l’envoi est explicitement indiqué. Les métadonnées enregistrées excluent les en-têtes HTTP/2 automatiques et ne prouvent pas la réception par le serveur. Le streaming client/bidirectionnel, les CA personnalisées/mTLS et la gestion complète des racines d’import Proto ne sont pas encore disponibles. L’affichage des réponses est limité à 8 MiB.

## Environnements et variables

Le sélecteur d’environnement et **Variables** permettent de créer, renommer, supprimer et modifier les valeurs. L’environnement actif remplace les variables globales de même nom. Sans environnement, seules les globales s’appliquent.

Utilisez `{{endpoint}}` pour l’adresse, `{{appid}}` pour l’AppID du plugin et `{{secret}}` pour son Secret. Exemple de corps :

```text
{
  "user_id": "{{user_id}}",
  "include_profile": {{include_profile}}
}
```

Les références entre guillemets sont échappées comme chaînes JSON ; sans guillemets, elles doivent produire du JSON valide : nombre, booléen, objet ou tableau. Les métadonnées exigent des chaînes. Les références imbriquées, comme `{{host}}:{{port}}`, sont acceptées. Une variable absente, un nom en double ou une référence circulaire bloque l’envoi. Les noms commencent par une lettre ou un trait de soulignement et peuvent contenir chiffres, points et tirets.

Les valeurs ordinaires sont enregistrées. Les valeurs sensibles, secrets bruts, données d’authentification, métadonnées et historique restent dans la session. Une référence complète `{{secret}}` est mémorisée, mais sa valeur sensible doit être ressaisie après redémarrage. Une valeur sensible utilisée dans le corps ou un en-tête est envoyée et apparaît dans l’instantané de cette session. Les brouillons sur disque conservent les modèles non résolus.

## Plugins d’en-têtes

Choisissez **MD5 · AppID + Horodatage**, saisissez AppID et Secret, puis activez le plugin. À chaque envoi, l’horodatage Unix courant en millisecondes devient `ts`. La signature `sign` est le résultat hexadécimal minuscule de `MD5(secret + appid + ts + secret)`. Les en-têtes `appid`, `ts` et `sign` sont envoyés ensemble et utilisent le même instant. Les noms d’en-tête, l’unité et la casse sont configurables.

Des exemples HMAC-SHA256 et AES-256-GCM sont inclus. Un seul plugin sélectionné s’exécute par appel ; le chaînage et l’affectation par méthode ne sont pas disponibles. Changer de plugin efface Secret mais conserve les paramètres ordinaires de chaque plugin. L’aperçu n’est qu’un instantané ; l’envoi recalcule toujours les résultats. Vérifiez un nouvel appel dans l’historique pour voir les données réellement transmises. Voir le [guide des plugins (chinois)](plugins/README.md) pour Rhai et les traductions.

## Développement et vérification

Prérequis : Rust stable, Go, Python 3.11+ et les outils de compilation natifs du système. La version Go est fixée dans `.go-version`.

```sh
python tools/setup.py
cargo run --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
python -m unittest discover -s tools/tests
```

`tools/bin/rpcrad-demo` lance l’exemple sur `127.0.0.1:50051`. Essayez GetUser, WatchUsers ou `user_id = missing` pour une erreur NotFound. Sur macOS, `Launch RPCRad.command` démarre l’exemple et ouvre l’application. Aucun compte ni service cloud n’est requis. Le contrôle de version contacte GitHub sans envoyer le contenu de vos requêtes.

Chaque publication est vérifiée sur les deux architectures Mac, Windows, Ubuntu et Arch/Wayland. Sa description contient toujours des sections chinoise et anglaise, ainsi que les instructions xattr. Voir la [procédure de publication (chinois)](docs/RELEASING.md) et [AGENTS.md](AGENTS.md). Sources principales : `src/ui.rs`, `src/rpc.rs`, `locales/`, `examples/protos/` et `tools/demo/`. Références de conception : BloomRPC, Kreya, Yaak. Les licences des dépendances sont celles de leurs projets respectifs.
