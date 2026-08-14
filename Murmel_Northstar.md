# Murmel — Northstar & Projekt-Dokumentation

> **Ein persönlicher WisprFlow-Clone.**
> Kein SaaS. Kein Cloud-Zwang. Kein Bloat.
> Nur du, deine Stimme und dein Text.

**Stand:** August 2026 · Fork-Basis: [cjpais/Handy](https://github.com/cjpais/Handy) `b50b52a` (MIT)

---

## 1. Northstar

### Was Murmel sein soll

Murmel ist ein **privater, lokaler Speech-to-Text-Diktierassistent**. Primärziel ist
**Windows** (schnell und performant im Alltag), gleichberechtigt daneben **Ubuntu
24.04**. Langfristig kommt **Android** als eigenständiger Client dazu. Murmel läuft
lokal, sammelt keine Telemetrie und gehört allein dem Nutzer.

Die zentrale UX-Philosophie ist **Unsichtbarkeit mit Kontrolle**:

- **Unsichtbar:** Murmel ist da, wenn du ihn brauchst, und verschwindet, wenn du ihn nicht brauchst. Kein ständig sichtbares Overlay, kein Bloat.
- **Kontrolle:** Mit einem globalen Hotkey (z. B. `Ctrl + Win + M`) startest du die Diktat-Session. Mit einem zweiten Hotkey (`Ctrl + Shift + H`) öffnest du die Historie. Alles andere passiert im Hintergrund.
- **Sofortigkeit:** Gesprochener Text landet sofort am Cursor — egal in welcher Anwendung. Kein Copy-Paste-Zirkus.

### Die vier Säulen

| Säule                                | Bedeutung                                                                                                    |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| **Schnell auf Windows**              | Kalt- und Warmstart unter einer Sekunde bis zur Aufnahmebereitschaft; Transkription schneller als Echtzeit   |
| **Elegante, kleine UI**              | Ein Fenster, klare Typografie, kein Framework-Ballast im Sichtfeld                                           |
| **Text verstehen, nicht nur tippen** | Diktiertes wird auf Wunsch formatiert, aufgeräumt, umgeschrieben — wie bei WisprFlow, aber lokal (siehe §5)  |
| **Deine Daten gehören dir**          | Historie und Nutzungsstatistiken liegen lokal in SQLite; hübsch aufbereitet, aber nie hochgeladen (siehe §6) |

### Was Murmel _nicht_ sein soll

- Kein Meeting-Transkriptions-Tool
- Kein AI-Agent mit Chat-Interface
- Kein Cloud-Sync- oder Team-Collaboration-Tool
- Kein Electron-Monster mit 300 MB RAM-Verbrauch
- Kein Bloat: jede Dependency und jedes Feature muss sich rechtfertigen
- Kein Hack (kein DE-Shortcut-Workaround auf Wayland)

---

## 2. Warum ein eigener Clone?

### Das Problem mit WisprFlow

WisprFlow ist ein exzellentes Produkt, aber:

- **Closed Source** — man weiß nicht, was mit den Audiodaten passiert
- **Cloud-Abhängigkeit** — auch "lokale" Modelle laufen oft über deren Infrastruktur
- **Schwere App** — das UI fühlt sich träge an, das Overlay ist nicht verschiebbar
- **Keine echte Kontrolle** — man ist an deren Update-Zyklus und Geschäftsmodell gebunden

### Warum keiner der freien Nachbauten

Auf GitHub gibt es einige freie WisprFlow-Alternativen. Sie sind meist in der
Freizeit entstanden und verdienen Respekt dafür — für den täglichen Einsatz über
Jahre hinweg passen sie mir trotzdem nicht:

- **Pflegestand** — viele ruhen seit Monaten oder Jahren. Ein Werkzeug, das ich
  täglich benutze, muss OS-Updates überleben.
- **Plattformbindung** — häufig nur Linux oder nur Windows; ich brauche beides.
- **Tragfähigkeit** — Skripte ohne Tests und CI sind schnell gebaut, aber schwer
  über Jahre zu pflegen. Das ist eine Anforderung an mich selbst, keine Wertung
  fremder Arbeit.

### Die Lösung: Murmel

Ein **solider, wartbarer, privater Fork** auf Basis bewährter Open-Source-Technologie, der exakt die Features implementiert, die _du_ brauchst — und nichts davon wird je an einen Server gesendet.

---

## 3. Basis-Projekt: Handy

### Warum Handy?

| Kriterium               | Handy                             | OpenWhispr                  | nerd-dictation      |
| ----------------------- | --------------------------------- | --------------------------- | ------------------- |
| **Lizenz**              | MIT (permissiv, privat nutzbar)   | MIT                         | GPL v3.0 (Copyleft) |
| **Stack**               | Rust (Tauri) + React              | Electron + React            | Python              |
| **Cross-Platform**      | ✅ macOS, Windows, Linux          | ✅ macOS, Windows, Linux    | ❌ nur Linux        |
| **Stars**               | 22.435                            | 3.394                       | ~500                |
| **Aktivität**           | Sehr aktiv (wöchentliche Commits) | Aktiv                       | Verlangsamt         |
| **Fork-Freundlichkeit** | 🟢 Modularer Rust-Core            | 🟡 Komplexer Electron-Stack | 🟢 Einfach          |
| **Ressourcen**          | 🟢 Gering (Tauri/Rust)            | 🟡 Mittel (Electron)        | 🟢 Gering           |

Handy ist die **einzige Codebase**, die einen soliden, modularen Rust-Core für STT mitbringt, der unter MIT steht und aktiv gepflegt wird.

### Was von Handy übernommen wird

- **`transcribe-cpp`** — Lokale Spracherkennung mit Whisper-Familie (GGML/GGUF)
- **`transcribe-rs`** — CPU-optimierte Erkennung mit Parakeet V3
- **`cpal`** — Cross-Platform Audio I/O
- **`vad-rs`** — Voice Activity Detection (Silero)
- **Text-Injection-Logik** — `xdotool` (X11), `wtype`/`ydotool` (Wayland), native Windows APIs
- **SQLite-History** — Datenbank für Transkriptions-Verlauf
- **Tauri-Grundgerüst** — Cross-Platform Desktop-App Framework
- **LLM-Nachbearbeitung** — bereits vorhanden, siehe §5. Das war die angenehme
  Überraschung beim Fork: die WisprFlow-artige Textveredelung muss nicht von null
  gebaut werden.

### Was von Handy entfernt wurde (Stand: Rebranding-Commit)

- **Upstream-Branding** — Name, Wortmarke, App-Icon und Tray-Icons sind neu
  (Upstream behält sich seine Markenassets ausdrücklich vor, siehe [NOTICE.md](NOTICE.md))
- **Sponsoren, Discord, FUNDING** — gehören dem Upstream-Projekt
- **Community-Governance** — Feature-Freeze, RFC-Prozess und Discussions-Pflicht
  ergeben für ein Ein-Personen-Projekt keinen Sinn
- **Raycast-Extension, Homebrew/winget** — irrelevant; Murmel wird selbst gebaut

### Was noch offen ist

- **React-Frontend** — steht noch (siehe §4.2 für die ehrliche Einordnung)
- **macOS** — läuft im Code weiter mit (die plattformspezifischen Pfade sind
  nicht entfernt), wird aber **nicht mehr released**. Der Grund ist technisch
  und nicht zu umgehen, ohne ein Apple-Zertifikat zu kaufen: `tauri build`
  signiert bedingungslos, sobald `APPLE_CERTIFICATE` in der Umgebung steht, und
  der geerbte Workflow setzt die Variable auf einen leeren String statt sie
  wegzulassen. Ein Release, der bei jedem Lauf rot meldet, verdeckt die
  Fehlschläge, auf die es ankommt — deshalb sind die beiden macOS-Ziele seit
  0.12.0 aus der Matrix. Zurückholen: zwei Einträge in `release.yml` plus ein
  Zertifikat.

### Erledigt

- **Updater-Signaturschlüssel** — eigenes Paar seit 0.10.0; der private Teil liegt
  ausserhalb des Repos und im GitHub-Secret `TAURI_SIGNING_PRIVATE_KEY`
- **Code-Signing-Zertifikat** — bewusst nicht angeschafft. Der SmartScreen-Hinweis
  beim ersten Start ist der akzeptierte Preis; die Update-Integrität hängt am
  Signaturschlüssel, nicht am Zertifikat.

---

## 4. Architektur-Entscheidungen

### 4.1 Backend: Rust (Tauri)

- **Warum Rust?** Memory-Safety, native Performance, keine Garbage-Collection-Pausen
- **Warum Tauri?** Geringer Footprint (~600 KB Runtime vs. ~150 MB Electron), native System-Integration, Cross-Platform-Abstraktion für Fenster und System-Events

### 4.2 Frontend: klein halten — aber mit offenen Augen

Das Ziel bleibt eine **kleine, elegante Custom-UI**. Der Weg dorthin verdient
allerdings Ehrlichkeit, denn der Fork bringt deutlich mehr Frontend mit als
ursprünglich angenommen:

**Ist-Zustand:** React + Tailwind + Zustand, dazu Settings, Onboarding,
Modell-Auswahl, Update-Checker, Debug-Panels und **i18n in 24 Sprachen**. Das ist
kein „Bloat um des Bloats willen" — an dem UI hängt echte Funktionalität.

**Die Spannung:** Ein kompletter Rauswurf von React bedeutet, all das
nachzubauen — realistisch mehrere Wochen, mit dem Risiko, funktionierende Features
zu verlieren. Das kollidiert mit dem Ziel, schnell zu einem täglich nutzbaren
Werkzeug zu kommen.

**Der gewählte Pfad — Reduktion statt Rewrite:**

1. **Zuerst wegschneiden, was nicht gebraucht wird:** ~~i18n auf
   Deutsch/Englisch reduzieren~~ (erledigt, 0.12.0), macOS-spezifische UI-Pfade
   entfernen, Onboarding auf das Nötigste eindampfen. Das bringt den größten
   Effekt bei kleinstem Risiko.

   > **Warum die i18n-Reduktion zuerst kam:** Der CI-Prüfschritt vergleicht
   > jede Sprache gegen die englische Referenz. Jeder neue UI-Text kostete
   > damit 24 Einträge — 22 davon maschinell oder mit Englisch gefüllt, also
   > Einträge, die niemand gegenlesen kann. Vor einer Reihe von UI-Arbeiten
   > vervielfacht sich dieser Aufwand; danach kostet ein Text zwei Einträge.

2. **Dann das Hauptfenster neu gestalten:** Historie als zentrale Ansicht (§7),
   Einstellungen dahinter aufgeräumt.
3. **Rewrite nur, wenn er sich dann noch lohnt.** Wenn nach Schritt 1 und 2 ein
   schlankes, schnelles UI dasteht, ist die Framework-Frage zweitrangig — der
   Nutzer sieht das Ergebnis, nicht die Dependency-Liste.

**Falls doch ein Rewrite:** Vanilla HTML/JS/CSS über die Tauri-API. Leptos
(Rust→WASM) wäre die puristische Variante, kauft aber eine zweite Lernkurve ein.

### 4.3 STT-Engine: zwei Pfade, Modellwahl noch offen

Murmel bringt zwei unabhängige Inferenz-Pfade mit. Das ist eine Architektur-, keine
Modellentscheidung:

| Pfad                | Bibliothek       | Format    | GPU                         |
| ------------------- | ---------------- | --------- | --------------------------- |
| **Whisper-Familie** | `transcribe-cpp` | GGML/GGUF | ja (Vulkan/Metal), optional |
| **ONNX-Modelle**    | `transcribe-rs`  | ONNX      | nein, CPU                   |

Der Modellkatalog (`src-tauri/src/catalog/catalog.json`) enthält aktuell **67 Modelle**
— Whisper in mehreren Größen, Parakeet, Canary, Moonshine, SenseVoice, GigaAM und
andere. Sie werden **zur Laufzeit in der App** heruntergeladen und umgeschaltet;
nichts davon ist im Code festverdrahtet.

> **Offene Entscheidung:** Welches Modell der Default wird, ist noch nicht
> festgelegt und lässt sich sinnvoll erst nach eigenem Ausprobieren beantworten —
> die Kriterien (deutsche Erkennungsqualität, Latenz auf der eigenen Hardware,
> Speicherbedarf) sind maschinenabhängig. Bis dahin gilt: **selbst vergleichen,
> dann entscheiden.**

### 4.4 Audio-Pipeline

```
Mikrofon → cpal → VAD (Silero) → STT (Whisper/Parakeet) → [LLM-Nachbearbeitung] → Text-Injection
                                                            └─ optional, §5
```

Alles läuft lokal. Kein Byte verlässt den Rechner — vorausgesetzt, die
Nachbearbeitung nutzt ein lokales Modell (siehe §5.2).

---

## 5. Textveredelung — der WisprFlow-Teil

Das, was WisprFlow von einem reinen Diktiergerät unterscheidet: Gesprochenes wird
nicht bloß transkribiert, sondern **aufgeräumt**. Füllwörter raus, Interpunktion
rein, „ähm" weg, auf Wunsch in eine andere Tonalität umgeschrieben.

### 5.1 Was bereits da ist

Der Fork bringt eine vollständige Post-Processing-Pipeline mit — das war der
größte Gewinn der Fork-Entscheidung:

| Baustein                | Wo                            | Was es kann                                                                                          |
| ----------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------------- |
| **LLM-Client**          | `src-tauri/src/llm_client.rs` | OpenAI-kompatible Chat-Completions, Structured Outputs, Reasoning-Abschaltung mit Provider-Fallbacks |
| **Provider-Verwaltung** | `src-tauri/src/settings.rs`   | Mehrere Endpunkte parallel (Ollama, OpenAI, OpenRouter, Anthropic, Groq, …), je eigener Schlüssel    |
| **Prompt-System**       | `post_process_prompts`        | Vier Presets plus frei definierbare Prompts, per Auswahl umschaltbar                                 |
| **Zweiter Hotkey**      | `--toggle-post-process`       | Diktat _mit_ Nachbearbeitung, getrennt vom normalen Diktat                                           |
| **Historie**            | `post_process_runs`           | Ein Eintrag je Veredelungslauf, auch für fehlgeschlagene (§9)                                        |

> **API-Schlüssel liegen im Schlüsselbund des Betriebssystems** (Windows
> Credential Manager, Secret Service unter Linux) — seit 0.15.0, siehe
> `src-tauri/src/secrets.rs`. Bestehende Schlüssel werden beim ersten Start
> einmalig dorthin verschoben und aus `settings_store.json` entfernt.
>
> Dass es so weit kam, hatte einen Grund: Hier stand lange „API-Keys
> verschlüsselt abgelegt", während sie tatsächlich im Klartext in der
> Einstellungsdatei lagen. `SecretMap` verhindert nur, dass Schlüssel in
> Logausgaben auftauchen.
>
> Eigene Verschlüsselung wurde bewusst **nicht** gebaut: Der Schlüssel zum
> Entschlüsseln müsste auch irgendwo liegen, das Problem wäre nur verschoben.
> Ist kein Schlüsselbund erreichbar (Linux-Sitzung ohne laufenden Dienst),
> meldet Murmel das — es fällt **nicht** stillschweigend auf eine Datei zurück.

Ein Diktat kann also schon heute roh **oder** veredelt eingefügt werden — der
Unterschied liegt nur am gedrückten Hotkey.

### 5.2 Wohin es gehen soll

**Lokal als Standard.** Unter den vorkonfigurierten Providern ist bereits
**Ollama** (`http://localhost:11434/v1`). Das ist der Weg, der zur Privacy-Säule
passt: ein kleines Instruct-Modell (z. B. Qwen3 4B oder Llama 3.2 3B) räumt den
Text auf, ohne dass er den Rechner verlässt. Cloud-Provider bleiben möglich, sind
aber bewusst nicht der Default.

> **Konsequenz für das Manifest:** „Kein Netzwerk-Request" gilt uneingeschränkt nur
> mit lokalem LLM. Wer einen Cloud-Provider konfiguriert, schickt Text an Dritte.
> Murmel muss das in der UI **unmissverständlich** anzeigen — kein stilles Abfließen.

**Prompt-Presets statt Prompt-Basteln.** Ein paar durchdachte Voreinstellungen
schlagen ein leeres Textfeld:

| Preset           | Zweck                                                                            |
| ---------------- | -------------------------------------------------------------------------------- |
| _Aufräumen_      | Füllwörter entfernen, Interpunktion setzen, Satzbau glätten — Inhalt unverändert |
| _E-Mail_         | Anrede, Absätze, Grußformel                                                      |
| _Notiz_          | Stichpunkte statt Fließtext                                                      |
| _Code-Kommentar_ | Knapp, technisch, ohne Floskeln                                                  |
| _Roh_            | Keine Nachbearbeitung (Fallback)                                                 |

**Analyse, nicht nur Formatierung.** Der Structured-Output-Support im LLM-Client
erlaubt mehr als Umschreiben: erkannte Sprache, Tonalität, Länge, offene Fragen im
Diktat. Diese Metadaten fließen in die Statistiken (§6) — sie sind der Grund, warum
sich Analyse und Auswertung gegenseitig tragen.

---

## 6. Nutzungsdaten — hübsch, aber ausschließlich lokal

WisprFlow zeigt „du hast diese Woche X Wörter diktiert und Y Minuten gespart". Das
macht Spaß und motiviert. Murmel soll das auch können — **ohne dass ein einziger
Datenpunkt den Rechner verlässt**.

### 6.1 Was erfasst wird

Alles ergibt sich aus dem, was ohnehin passiert. Nichts wird zusätzlich beobachtet:

| Kennzahl                    | Herkunft                                          |
| --------------------------- | ------------------------------------------------- |
| Wörter / Zeichen pro Diktat | Transkript                                        |
| Diktatdauer                 | Aufnahmelänge                                     |
| Sprechgeschwindigkeit (WPM) | Wörter ÷ Dauer                                    |
| Verarbeitungszeit           | Zeitstempel der STT-Pipeline                      |
| Verwendetes Modell          | Transkriptions-Manager                            |
| Erkannte Sprache            | STT-Ausgabe                                       |
| Nachbearbeitung genutzt?    | `post_process_requested` (existiert bereits)      |
| Zeitersparnis (geschätzt)   | Wörter ÷ 40 WPM Tippgeschwindigkeit − Diktatdauer |

### 6.2 Was das Schema dafür braucht — erledigt

Die Migration liegt in `src-tauri/src/managers/history.rs` (`MIGRATIONS`). Das
Migrationssystem ist additiv — bestehende Einträge bekommen `NULL` und die
Statistik rechnet sie schlicht nicht mit.

Die fünf Metrik-Spalten kamen wie geplant dazu. Darüber hinaus wanderte die
**Nachbearbeitung in eine eigene Tabelle** (`post_process_runs`, §9), statt wie
ursprünglich skizziert als Spalten auf der History zu bleiben. Zwei Gründe:

1. **Ein Diktat kann mehrfach veredelt werden.** Ohne eigene Tabelle wäre jede
   zweite Veredelung ein Überschreiben der ersten — und der Command-Mode-Gedanke
   („markieren, umschreiben lassen") wäre dauerhaft verbaut.
2. **Fehlgeschlagene Läufe sind eine Kennzahl, kein Nichts.** Vorher gab die
   Pipeline bei einem LLM-Fehler schlicht `None` zurück, ununterscheidbar von
   „gar nicht erst versucht". Der Anteil fehlgeschlagener Läufe ist aber genau
   die Zahl, an der man abliest, ob ein lokales Modell alltagstauglich ist.

### 6.3 Wie es aussehen soll

Ein **Insights**-Bereich im Hauptfenster, bewusst zurückhaltend:

- Wörter pro Tag/Woche, als schlichter Balkenverlauf
- Kumulierte geschätzte Zeitersparnis („diesen Monat 2 h 14 min")
- Aktivste Tageszeit
- Meistgenutztes Modell und dessen Durchschnittsgeschwindigkeit
- Verhältnis roh zu nachbearbeitet

**Regeln, die nicht verhandelbar sind:**

1. **Keine Telemetrie.** Kein Opt-in, keine anonymisierte Übermittlung, gar nichts.
   Der Upstream hatte „Opt-in Analytics" auf der Roadmap — für Murmel ist das gestrichen.
2. **Vollständig exportierbar.** JSON oder CSV, ein Klick.
3. **Vollständig löschbar.** Statistiken zurücksetzen, ohne die Historie zu verlieren
   — und umgekehrt.

---

## 7. Systemintegration

### 7.1 Globale Hotkeys

| Plattform                  | Mechanismus                    | Status                   |
| -------------------------- | ------------------------------ | ------------------------ |
| **Windows**                | Tauri `global-shortcut` Plugin | ✅ Nativ, out-of-the-box |
| **Ubuntu 24.04 (X11)**     | `rdev` + `xdotool`             | ✅ Funktioniert          |
| **Ubuntu 24.04 (Wayland)** | **Native D-Bus-Anbindung**     | 🟡 Muss gebaut werden    |

### 7.2 D-Bus für Wayland — Kein Hack

Auf Wayland (Ubuntu 24.04 Default) können Anwendungen **keine globalen Hotkeys** direkt abfangen. Das ist ein Sicherheitsfeature von Wayland.

**Die saubere Lösung: D-Bus**

- Murmel registriert sich als **D-Bus-Service** (`org.murmel.app`)
- Über D-Bus expose Murmel Actions wie `ToggleTranscription` und `ShowHistory`
- Der Nutzer bindet die gewünschten Tastenkombinationen in **GNOME/KDE-Settings** an D-Bus-Methoden-Aufrufe
- Murmel reagiert auf diese D-Bus-Signale und führt die Aktion aus

**Warum das kein Hack ist:**

D-Bus ist der **offizielle IPC-Mechanismus** von Linux-Desktops. GNOME, KDE und alle anderen DEs nutzen D-Bus intern für genau solche Zwecke. Das ist die von den Desktop-Entwicklern vorgesehene Art, globale Aktionen zu triggern — nicht ein Workaround, sondern die korrekte Architektur.

**Beispiel GNOME-Shortcut:**

```bash
# In GNOME Settings > Keyboard > Custom Shortcuts
gdbus call --session --dest org.murmel.app --object-path /org/murmel/app --method org.murmel.app.ToggleTranscription
```

**Alternative:** Murmel könnte bei der Installation eine `.desktop`-Datei mit `Actions=` definieren, die über D-Bus ansprechbar sind.

### 7.3 Text-Injection (Pasten)

| Plattform         | Mechanismus                                              |
| ----------------- | -------------------------------------------------------- |
| **Windows**       | Native Win32 API (`SendInput` oder Clipboard + `Ctrl+V`) |
| **Linux X11**     | `xdotool` oder `enigo`                                   |
| **Linux Wayland** | `ydotool` (systemd-Service) oder `dotool`                |

**Wichtig für Ubuntu 24.04:** `wtype` funktioniert auf Ubuntu 24.04 Wayland **nicht**. `ydotool` ist erforderlich und muss als systemd-Service laufen. cite🛠web_search:1#2:~:text=Ubuntu 26.04: Has Wayland display server by default. wtype does not work, you need to install ydotool and configure systemd

Die Text-Injection-Logik aus Handy (`enigo` als Fallback, `xdotool`/`ydotool` als Primary) wird übernommen und an Murmel angepasst.

#### Windows-Zwischenablageverlauf (Win+V) — bewusste Abwägung

Standardmäßig fügt Murmel den Text ein und **stellt die vorherige Zwischenablage
sofort wieder her**. Das ist spurlos, hat aber eine Folge: Das Diktat taucht nicht
im Windows-Zwischenablageverlauf (`Win+V`) auf, weil es dort nie lange genug liegt.

Für Murmel ist stattdessen **„In Zwischenablage kopieren"** gesetzt
(`ClipboardHandling::CopyToClipboard`, Einstellungen → Erweitert). Der fertige
Text bleibt danach in der Zwischenablage und wird von Windows in den Verlauf
aufgenommen — so wie man es von WisprFlow kennt.

**Was das kostet, offen benannt:** Damit liegt jedes Diktat in einem Speicher, den
Windows verwaltet und nicht Murmel. Die Northstar-Regel „kein Byte verlässt den
Rechner" gilt weiterhin — Murmel sendet nichts. „Nichts bleibt liegen" gilt aber
nicht mehr. Zwei Konsequenzen:

- **Cloud-Zwischenablage muss aus bleiben.** Ist die Synchronisierung des Verlaufs
  aktiviert, schickt _Windows_ die Texte an Microsoft — an Murmel vorbei. Der
  Registry-Wert `HKCU\Software\Microsoft\Clipboard\CloudClipboardAutomaticUpload`
  gehört auf 0 bzw. ungesetzt.
- **Der Verlauf ist eine bewusste Nutzerentscheidung**, kein Default. Wer heikle
  Inhalte diktiert, stellt auf „Zwischenablage nicht ändern" zurück.

Interessant für später: Für den optionalen „reliable paste"-Pfad markiert Murmel
die _flüchtige_ Zwischenablage bereits mit den Windows-Opt-out-Formaten
(`CanIncludeInClipboardHistory=0`, `CanUploadToCloudClipboard=0` — dieselben, die
Chrome für Inkognito-Kopien nutzt). Dieselbe Technik wäre der saubere Weg für einen
späteren **„Privates Diktat"-Hotkey**, der bewusst nichts hinterlässt.

### 7.4 Overlay — Minimalistisch & Verschiebbar

WisprFlow's Overlay ist nicht verschiebbar — das ist nervig.

Murmel's Overlay:

- **Nur bei aktiver Aufnahme sichtbar**
- **Verschiebbar** per Drag (Tauri-Fenster mit `always-on-top` + `drag` Events)
- **Position wird gespeichert** (per Session oder persistent in SQLite)
- **Optional deaktivierbar** — Audio-Feedback (Piepton) reicht manchen Nutzern

**Design:** Ein kleines, halbtransparentes Widget (z. B. 200x40px) mit einem Mikrofon-Icon und einem kurzen Status-Text ("Aufnahme..." / "Verarbeite..."). Keine React-Komponenten, kein CSS-Framework — pure CSS, ~50 Zeilen.

---

## 8. UI-Philosophie: Ein Fenster, eine Aufgabe

### Das Hauptfenster (Historie)

Ein einziges App-Fenster. Keine Tabs, keine Seiten, keine Einstellungen-Seite (die kommt später, wenn überhaupt).

```
┌─────────────────────────────────────────┐
│  Murmel                    [─] [□] [×]  │
├─────────────────────────────────────────┤
│  🔍 Suche...                            │
├─────────────────────────────────────────┤
│  1. "Das ist ein Testdiktat..."         │
│     18:42  [📋 Kopieren]  [🗑 Löschen]  │
│                                         │
│  2. "Bitte buche den Termin für..."     │
│     18:38  [📋 Kopieren]  [🗑 Löschen]  │
│                                         │
│  3. "Idee für neues Projekt: Murmel..." │
│     18:15  [📋 Kopieren]  [🗑 Löschen]  │
│                                         │
├─────────────────────────────────────────┤
│  [⚙ Einstellungen]  [? Hilfe]           │
└─────────────────────────────────────────┘
```

- **Liste** der letzten N Transkriptionen (konfigurierbar, Default: 50)
- **Kopieren-Button** pro Eintrag → kopiert in Clipboard
- **Löschen-Button** pro Eintrag → entfernt aus DB
- **Suche** — Volltextsuche über alle Transkriptionen
- **Einstellungen** — minimalistisch: Hotkey-Config, Modell-Auswahl, Overlay-Position

### Kein React

Stattdessen:

- **HTML5** für Struktur
- **Vanilla JavaScript** für Interaktivität (Tauri-Invoke für Rust-Calls)
- **CSS** für Styling (kein Tailwind, kein Bootstrap — pure CSS-Variablen für Dark/Light Mode)

Das gesamte Frontend sind voraussichtlich **< 500 Zeilen Code**.

---

## 9. Datenmodell (SQLite)

**Ist-Zustand** (`src-tauri/src/managers/history.rs`, migriert über `rusqlite_migration`):

```sql
CREATE TABLE transcription_history (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    file_name               TEXT NOT NULL,      -- zugehörige Audiodatei
    timestamp               INTEGER NOT NULL,
    saved                   BOOLEAN NOT NULL DEFAULT 0,
    title                   TEXT NOT NULL,
    transcription_text      TEXT NOT NULL,      -- Rohtranskript
    post_process_requested  BOOLEAN NOT NULL DEFAULT 0,  -- welcher Hotkey
    duration_ms             INTEGER,            -- Aufnahmelänge
    word_count              INTEGER,            -- Wörter im *Rohtranskript*
    processing_ms           INTEGER,            -- reine STT-Zeit
    model_used              TEXT,
    language                TEXT                -- effektiv genutzte Sprache
);

CREATE TABLE post_process_runs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    history_id   INTEGER NOT NULL REFERENCES transcription_history(id) ON DELETE CASCADE,
    timestamp    INTEGER NOT NULL,
    provider_id  TEXT NOT NULL,     -- 'ollama', 'openai', 'opencc', …
    model        TEXT,
    prompt_id    TEXT,
    prompt_text  TEXT,              -- Wortlaut zum Zeitpunkt des Laufs
    input_text   TEXT NOT NULL,     -- nicht immer das Rohtranskript
    output_text  TEXT,              -- NULL bei Fehlschlag
    duration_ms  INTEGER,
    succeeded    BOOLEAN NOT NULL DEFAULT 0,
    error        TEXT
);
```

Ein Diktat hat **n** Veredelungsläufe. `word_count` zählt bewusst das
Rohtranskript — die veredelte Fassung zu zählen würde das Sprachmodell messen
statt den Sprecher. Der `provider_id` `opencc` markiert die
Chinesisch-Varianten-Konvertierung: kein LLM, aber sie schreibt den Text um, und
die Historie soll in jedem Fall beantworten können, warum der gespeicherte Text
vom transkribierten abweicht.

> **Falle, die Geld gekostet hätte:** SQLite erzwingt Fremdschlüssel nur, wenn
> `PRAGMA foreign_keys = ON` gesetzt ist — **pro Verbindung**, nicht pro
> Datenbank. Ohne das wäre `ON DELETE CASCADE` wirkungslos gewesen und gelöschte
> Diktate hätten ihre Läufe als Waisen zurückgelassen. Ein Test deckt das ab.

**Noch offen:** Volltextsuche. `fts5` über `transcription_text` wäre die naheliegende
Lösung, sobald die Historie mehr als ein paar hundert Einträge hat.

**Privacy:** Die Datenbank liegt lokal im App-Data-Verzeichnis:

- Windows: `%APPDATA%\com.suppenterrine.murmel\`
- Linux: `~/.local/share/com.suppenterrine.murmel/`

Kein Sync. Kein Backup in die Cloud. Export als `.txt` oder `.json` auf Wunsch.

---

## 10. Android — der Fernblick

Diktieren soll nicht am Schreibtisch aufhören. Das Ziel: dieselbe Erfahrung auf dem
Telefon, mit denselben Prinzipien (lokal, privat, ohne Konto).

### 10.1 Was dafür spricht

Tauri 2 unterstützt Android offiziell, und der Fork bringt die Icon-Struktur dafür
bereits mit (`src-tauri/icons/android/`). Der Rust-Kern — Audio, VAD, STT,
Post-Processing — ist plattformunabhängig geschrieben.

### 10.2 Was dagegen spricht — ehrlich betrachtet

Der Desktop-Weg lässt sich **nicht** direkt übertragen:

| Problem                    | Warum es auf Android nicht funktioniert                                                                                                                 |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Globale Hotkeys**        | Gibt es nicht. Android kennt keine systemweiten Tastenkürzel für Apps.                                                                                  |
| **Text-Injection**         | `enigo`/`xdotool`/`SendInput` existieren nicht. Text in fremde Apps zu schreiben geht **nur** über eine Input Method (IME) — also eine eigene Tastatur. |
| **Audio-Stack**            | `cpal` unterstützt Android eingeschränkt; realistisch braucht es Oboe/AAudio über JNI.                                                                  |
| **Modellgröße**            | Parakeet V3 (~478 MB) ist für ein Telefon grenzwertig. Moonshine Tiny/Base sind die realistischeren Kandidaten.                                         |
| **Hintergrund-Ausführung** | Aggressives Doze/Battery-Management; ein dauerlaufender Dienst braucht eine Foreground-Notification.                                                    |

### 10.3 Der wahrscheinliche Weg

**Murmel Android ist keine Portierung, sondern ein eigenständiger Client:**

1. **Als Tastatur (IME), nicht als App-mit-Fenster.** Eine Mikrofontaste in der
   Tastatur ist der einzige Weg, der systemweit in jedes Textfeld schreibt — und
   nebenbei genau das UX-Modell, das WisprFlow auf dem Handy nutzt.
2. **Kleines Modell, on-device.** Moonshine oder Whisper Tiny, quantisiert.
3. **Gemeinsamer Rust-Kern, getrennte Oberfläche.** Die Transkriptionspipeline als
   Rust-Bibliothek über JNI; die Tastatur selbst nativ in Kotlin.
4. **Optional: Kopplung an den Desktop.** Historie und Statistiken über das lokale
   Netz synchronisieren — nur wenn es ohne fremden Server geht.

**Einordnung:** Das ist ein **eigenes Projekt in Murmel-Kleidung**, kein Nebenprodukt.
Erst angehen, wenn Windows und Ubuntu wirklich rundlaufen.

---

## 11. Roadmap

### Phase 0: Fork-Übernahme ✅

- [x] Fork von Handy erstellen
- [x] Vollständiges Rebranding auf Murmel (Code, UI, Icons, Doku)
- [x] Eigenes Branding: Wortmarke, App-Icon, Tray-Icon-Set
- [x] MIT-Attribution und Herkunftsdokumentation ([NOTICE.md](NOTICE.md))

### Phase 1: Windows-Alltagstauglichkeit ✅

- [x] Build und Start auf Windows verifizieren
- [x] Hotkeys einrichten (Diktat + Diktat-mit-Nachbearbeitung)
- [x] Eigenen Updater-Signaturschlüssel erzeugen — und ab 0.11.0 einen
      vollständigen Release-Zyklus samt Selbstupdate durchlaufen
- [x] Transkriptionsgeschwindigkeit messen — nicht als einmalige Baseline,
      sondern dauerhaft: `processing_ms`, `duration_ms` und `word_count`
      fallen seit 0.11.0 bei jedem Diktat mit an (§6.2). Auszuwerten ist
      das noch, siehe Phase 2.
- [ ] Mehrere Modelle im Alltag vergleichen (deutsche Qualität, Latenz, RAM)
      und **dann** einen Default festlegen — jetzt datengestützt möglich,
      braucht aber Nutzungsdaten über mehrere Tage

### Phase 2: Murmel wird Murmel

- [x] Lokales LLM (Ollama) als Default-Provider für die Nachbearbeitung —
      eigener Anbieter mit Erreichbarkeitsprüfung, nicht mehr als „Custom"
      getarnt
- [x] Prompt-Presets statt freiem Textfeld (§5.2)
- [x] Deutliche UI-Kennzeichnung, wenn ein Cloud-Provider aktiv ist — als
      lokal gilt nur Loopback, ein Ollama im Heimnetz wird als „verlässt den
      Rechner" gekennzeichnet
- [x] History-Migration um Metrik-Spalten erweitern (§6.2) — plus eigene
      Tabelle `post_process_runs` für die Veredelungsläufe (§9)
- [x] UI-Reduktion: i18n auf DE/EN (§4.2). Onboarding eindampfen steht noch aus
- [ ] Insights-Ansicht: Wörter, Zeitersparnis, Modell-Performance (§6.3) —
      wartet bewusst auf Nutzungsdaten, die sich seit 0.11.0 ansammeln
- [ ] Startseite als eigener Tab: Bildmarke, kurzer Text, was die App tut
- [ ] Release-Notes-Archiv in der App — repariert nebenbei, dass eine
      übersprungene Version nie wieder sichtbar wird
- [ ] Volltextsuche über die Historie (`fts5`) mit Favoriten-Umschalter

### Phase 3: Ubuntu 24.04

- [ ] Build auf Ubuntu 24.04 verifizieren
- [ ] Text-Injection über X11 (`xdotool`)
- [ ] D-Bus-Service für Wayland-Hotkeys (§7.2)
- [ ] `ydotool`-Integration für Wayland-Pasten
- [ ] Verschiebbares Overlay mit gespeicherter Position

### Phase 4: Android

- [ ] Machbarkeitsstudie: Rust-Kern über JNI auf Android
- [ ] IME-Prototyp mit Mikrofontaste
- [ ] Kleines On-Device-Modell (Moonshine/Whisper Tiny)

### Irgendwann, vielleicht

- [ ] Sprachbefehle („Neuer Absatz", „Punkt", „Komma")
- [ ] Export der Statistiken als JSON/CSV

---

## 12. Referenzen & Quellen

### Basis-Projekte

| Projekt                  | Repo                                   | Lizenz   | Stars (Stand: Aug 2026) |
| ------------------------ | -------------------------------------- | -------- | ----------------------- |
| **Handy**                | `github.com/cjpais/Handy`              | MIT      | 22.435                  |
| **OpenWhispr**           | `github.com/OpenWhispr/openwhispr`     | MIT      | 3.394                   |
| **nerd-dictation**       | `github.com/ideasman42/nerd-dictation` | GPL v3.0 | ~500                    |
| **Speech Note (dsnote)** | `github.com/mkiol/dsnote`              | MPL 2.0  | ~2.000                  |

### Weitere angesehene Projekte

Zum Zeitpunkt der Fork-Entscheidung länger ohne neue Commits — als Ideengeber
trotzdem lesenswert:

| Projekt               | Repo                                 | Letzter Commit (Stand Aug 2026) |
| --------------------- | ------------------------------------ | ------------------------------- |
| **whisper-writer**    | `github.com/savbell/whisper-writer`  | August 2024                     |
| **whisper-dictation** | `github.com/foges/whisper-dictation` | Juni 2024                       |

### Technologien

| Komponente       | Zweck                               | Repo                               |
| ---------------- | ----------------------------------- | ---------------------------------- |
| **Tauri**        | Cross-Platform Desktop-Framework    | `github.com/tauri-apps/tauri`      |
| **whisper.cpp**  | Lokale STT-Inference                | `github.com/ggerganov/whisper.cpp` |
| **cpal**         | Cross-Platform Audio I/O            | `github.com/RustAudio/cpal`        |
| **enigo**        | Cross-Platform Text-Injection       | `github.com/enigo-rs/enigo`        |
| **rdev**         | Globale Hotkeys (X11/macOS/Windows) | `github.com/Narsil/rdev`           |
| **Silero VAD**   | Voice Activity Detection            | `github.com/snakers4/silero-vad`   |
| **Parakeet**     | NVIDIA CPU-optimiertes STT-Modell   | `github.com/NVIDIA/NeMo`           |
| **D-Bus (zbus)** | Rust D-Bus-Client/Server            | `github.com/dbus2/zbus`            |

### Dokumentation

- Handy README & BUILD.md: `github.com/cjpais/Handy`
- Handy Linux Notes (Wayland, ydotool): `github.com/cjpais/Handy#linux-notes`
- Tauri Global Shortcut Plugin: `tauri.app/plugin/global-shortcut/`
- zbus D-Bus Rust Crate: `docs.rs/zbus/latest/zbus/`

---

## 13. Design-Prinzipien (Murmel-Manifest)

1. **Privacy by Design** — Keine Telemetrie, kein Cloud-Sync, keine Analytics.
   Netzwerk nur für den einmaligen Modell-Download und den Update-Check. Wer
   bewusst einen Cloud-LLM-Provider konfiguriert, tut das sehenden Auges — die UI
   sagt es klar (§5.2).
2. **No Bloat** — Jedes Feature muss sich rechtfertigen. Wenn es nicht essenziell ist, kommt es nicht rein.
3. **No Hacks** — Wayland-Hotkeys über D-Bus, nicht über Workarounds. Text-Injection über offizielle APIs.
4. **One Window** — Ein App-Fenster für alles. Keine Popups, keine Wizards, keine Onboarding-Tours.
5. **Offline First** — Alles Wesentliche funktioniert ohne Internet. Modelle werden einmalig heruntergeladen und bleiben lokal.
6. **Windows zuerst, Ubuntu gleichberechtigt** — Windows ist der Alltagsrechner und gibt das Tempo vor; Ubuntu 24.04 wird nicht nachgereicht, sondern mitgeführt. Android ist ein eigenes Kapitel (§10). macOS läuft mit, wird aber nicht gepflegt.
7. **Deine Daten bleiben deine** — Historie und Statistiken liegen lokal, sind vollständig exportierbar und vollständig löschbar (§6.3).
8. **Fork-Friendly** — Der Code ist so geschrieben, dass ein zukünftiges Ich (oder jemand anderes) ihn in 2 Jahren noch versteht.
9. **Ehrlich zum Upstream** — Übernommenes wird als übernommen gekennzeichnet, Attribution bleibt erhalten, fremde Markenassets werden nicht mitgeschleppt ([NOTICE.md](NOTICE.md)).

---

> _"Ich murmle, also bin ich."_
>
> — Murmel, 2026
