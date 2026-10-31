# RPCRad

[简体中文](README.md) | [English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Русский](README.ru.md)

Ein lokales gRPC-Debugging-Werkzeug für den Desktop, entwickelt mit Rust, GPUI und GPUI-KIT 0.8. Echte Anfragen werden über das mitgelieferte grpcurl 1.9.3 gesendet. Die Oberfläche ist nativ und verwendet keine WebView.

## Installation und Updates

Passende Pakete stehen unter [GitHub Releases](https://github.com/eyotang/RPCRad/releases/latest) bereit.

| System | Architektur | Paket |
| --- | --- | --- |
| macOS 15+ | Apple Silicon / Intel | Separate `.dmg`-Dateien |
| Windows 10/11 | x64 | `-setup.exe` oder portable `.zip` |
| Ubuntu 24.04+ | x64 | `.deb` oder portable `.tar.gz` |
| Omarchy / Arch Linux | x64 | `.pkg.tar.zst`, Wayland und X11 |

Alle Pakete enthalten grpcurl, Beispiel-Protos und die Lizenzen der Abhängigkeiten. Für die Nutzung werden weder Rust noch Go benötigt. Linux benötigt einen funktionierenden Vulkan-Treiber und ein Desktop-Portal. Für CJK-Schriften empfiehlt sich etwa `noto-fonts-cjk`. Windows-/Linux-ARM64-Pakete stehen noch nicht bereit.

Installation unter Ubuntu: `sudo apt install ./rpcrad_VERSION_amd64.deb`; unter Arch: `sudo pacman -U ./rpcrad-VERSION-1-x86_64.pkg.tar.zst`. VERSION durch die heruntergeladene Version ersetzen. Portable Pakete vollständig entpacken; Programm, grpcurl und resources müssen zusammenbleiben. Details stehen in der [Installationsanleitung (Chinesisch)](docs/INSTALL.md).

Unter macOS befindet sich **RPCRad → Über RPCRad / Nach Updates suchen** im Systemmenü. Unter Windows/Linux steht das **RPCRad**-Menü oben im Fenster bereit. Updates werden auch beim Start geprüft, über GitHub Releases, `latest.json` und SHA-256. Neue Pakete werden selbst heruntergeladen und installiert; der Arbeitsbereich bleibt erhalten. Es gibt keinen stillen Austausch im Hintergrund.

Die macOS-Pakete sind derzeit weder mit Developer ID signiert noch notarisiert. Windows-Installer besitzen keine Authenticode-Signatur. Falls die macOS-Quarantäne den Start verhindert, zuerst die offizielle Herkunft und die SHA-256-Prüfsumme bestätigen. Nur bei Bedarf ausführen:

```sh
xattr -dr com.apple.quarantine /Applications/RPCRad.app
open /Applications/RPCRad.app
```

Bei einem anderen Installationsort den Pfad anpassen. Dies dient der optionalen Fehlerbehebung und ist kein Update-Befehl. Die Anwendung führt ihn niemals automatisch aus.

## Oberflächensprachen

Unter **RPCRad → Einstellungen… → Sprache** bietet die Sprachauswahl **简体中文, English, 日本語, 한국어, Français, Deutsch und Русский**. Änderungen wirken sofort und bleiben nach einem Neustart erhalten. Neue und ältere Arbeitsbereiche ohne Sprachwahl verwenden vereinfachtes Chinesisch. Auch `--language=de` ist möglich; unterstützt werden `zh-CN`, `en`, `ja`, `ko`, `fr`, `de`, `ru`.

Die Einstellungen öffnen sich in einem eigenen Fenster mit **Systemeinstellung (Standard), Hell und Dunkel**. Änderungen gelten sofort und werden gespeichert; die Systemeinstellung folgt Änderungen des Betriebssystems. Auch Update-Prüfung und Programminformationen öffnen eigene Fenster. Erneutes Öffnen holt das vorhandene Fenster nach vorn, ohne die Anfrageansicht zu ersetzen. Einstellungen öffnen: **⌘ ,** unter macOS bzw. **Strg+,** unter Windows/Linux. Hilfsfenster schließen: **Esc** oder **⌘/Strg+W**.

Anwendungsmeldungen und Beschreibungen integrierter Plugins werden übersetzt. Eigene Namen, Variablenwerte, Anfrage-/Antwortdaten und bestehende Verlaufseinträge bleiben unverändert. Servermeldungen und Drittanbieter-Plugins ohne Übersetzungen werden nicht automatisch übersetzt.

## Funktionen und erste Anfrage

Proto importieren oder Server-Reflexion verwenden, Methode auswählen, Serveradresse eingeben und senden. **⌘ Enter** sendet unter macOS, **Ctrl+Enter** unter Windows/Linux. **⌘/Ctrl+K** sucht Methoden, **⌘/Ctrl+S** speichert. Seitenleiste und Editorspalten lassen sich in der Breite anpassen.

- Dienst- und Methodenbaum, Suche, Favoriten, Tabs und gespeicherte Anfrageentwürfe.
- Nativer JSON-Editor mit Syntaxhervorhebung, Formatierung und Proto-Beispielen.
- Unary- und Server-Streaming-Aufrufe, Abbruch, Zeitlimits, Status und Dauer.
- Metadaten, Bearer-Token und TLS mit den vertrauenswürdigen CAs des Systems.
- Umgebungen, globale Variablen und Plugins zur Header-Signierung.
- Antworttext, Headers, Trailers, Kopieren und bis zu 50 Aufrufe pro Sitzung.

Ein Verlaufseintrag zeigt den tatsächlichen Text, Verbindungseinstellungen, ursprüngliche Header und die endgültig an grpcurl übergebenen Metadaten. Normale Header sind sichtbar; vertrauliche Felder werden einzeln eingeblendet. Kopiert wird die aktuelle Anzeige. Der Papierkorb leert den Verlauf, ohne laufende Aufrufe zu unterbrechen. **Anfrage laden** stellt Adresse, Text und Verbindung wieder her; Header verwenden aktuelle Einstellungen, Plugins werden beim nächsten Senden erneut ausgeführt.

Fehler oder Abbrüche vor dem Senden werden als nicht gesendet markiert. Die Aufzeichnung enthält keine automatisch erzeugten HTTP/2-Header und beweist nicht, dass der Server die Daten erhalten hat. Client-/bidirektionales Streaming, eigene CAs/mTLS und vollständige Verwaltung von Proto-Importpfaden sind noch nicht verfügbar. Antworten werden bis 8 MiB angezeigt.

## Umgebungen und Variablen

Über die Umgebungsauswahl und **Variablen** lassen sich Umgebungen erstellen, umbenennen, löschen und Werte bearbeiten. Die aktive Umgebung hat Vorrang vor gleichnamigen globalen Variablen. Ohne ausgewählte Umgebung gelten nur globale Werte.

Beispiele: `{{endpoint}}` als Adresse, `{{appid}}` als Plugin-AppID, `{{secret}}` als Secret. Anfragetext:

```text
{
  "user_id": "{{user_id}}",
  "include_profile": {{include_profile}}
}
```

Verweise in Anführungszeichen werden als JSON-Zeichenfolgen maskiert. Ohne Anführungszeichen muss der Wert gültiges JSON sein: Zahl, boolescher Wert, Objekt oder Array. Metadatenwerte müssen Zeichenfolgen sein. Verschachtelte Verweise wie `{{host}}:{{port}}` sind möglich. Fehlende, doppelte oder zirkuläre Variablen verhindern das Senden. Namen beginnen mit einem Buchstaben oder Unterstrich und dürfen Ziffern, Punkte und Bindestriche enthalten.

Normale Werte werden gespeichert. Vertrauliche Variablenwerte, Secrets im Klartext, Authentifizierungsdaten, Metadaten und Verlauf gelten nur für die Sitzung. Ein vollständiger `{{secret}}`-Verweis wird gespeichert, der vertrauliche Wert muss nach einem Neustart neu eingegeben werden. Bewusst im Anfragetext oder Header verwendete vertrauliche Werte werden gesendet und erscheinen in der Momentaufnahme dieser Sitzung. Entwürfe auf der Festplatte behalten ihre Variablenvorlagen.

## Header-Plugins

**MD5 · AppID + Zeitstempel** auswählen, AppID und Secret eingeben und Plugin aktivieren. Standardmäßig wird bei jedem Senden die aktuelle Unix-Zeit in Millisekunden als `ts` verwendet. `sign` ist der kleingeschriebene Hexwert von `MD5(secret + appid + ts + secret)`. Gesendet werden `appid`, `ts` und `sign`; Zeitstempel und Signatur nutzen denselben Zeitpunkt. Headernamen, Zeiteinheit und Schreibweise sind konfigurierbar.

HMAC-SHA256- und AES-256-GCM-Beispiele sind enthalten. Pro Aufruf wird genau ein ausgewähltes Plugin ausgeführt; Verkettung und methodenspezifische Zuordnung sind nicht verfügbar. Beim Pluginwechsel wird Secret geleert; normale Einstellungen bleiben pro Plugin gespeichert. Vorschauen sind Momentaufnahmen und werden beim Senden neu berechnet. Tatsächlich gesendete Werte stehen im neuen Verlaufseintrag. Die [Plugin-Anleitung (Chinesisch)](plugins/README.md) beschreibt Rhai und Übersetzungsfelder.

## Entwicklung und Prüfung

Benötigt werden Rust stable, Go, Python 3.11+ und native Build-Werkzeuge. Die Go-Version ist in `.go-version` festgelegt.

```sh
python tools/setup.py
cargo run --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
python -m unittest discover -s tools/tests
```

`tools/bin/rpcrad-demo` startet den Beispielserver auf `127.0.0.1:50051`. GetUser, WatchUsers und `user_id = missing` für NotFound ausprobieren. Unter macOS startet `Launch RPCRad.command` Beispielserver und Anwendung. Kein Konto oder Cloud-Dienst erforderlich. Beim Start wird GitHub nach Updates gefragt, ohne Anfrageinhalte hochzuladen.

Releases werden auf beiden Mac-Architekturen, Windows, Ubuntu und Arch/Wayland geprüft. Jede Release-Beschreibung enthält chinesische und englische Abschnitte sowie die xattr-Anleitung. Siehe [Release-Ablauf (Chinesisch)](docs/RELEASING.md) und [AGENTS.md](AGENTS.md). Wichtige Quellen: `src/ui.rs`, `src/rpc.rs`, `locales/`, `examples/protos/`, `tools/demo/`. Gestaltungsvorbilder: BloomRPC, Kreya und Yaak. Für Abhängigkeiten gelten die Lizenzen ihrer jeweiligen Projekte.
