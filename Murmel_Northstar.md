# Murmel — Northstar & Projekt-Dokumentation

> **Ein persönlicher WisprFlow-Clone.**
> Kein SaaS. Kein Cloud-Zwang. Kein Vibe-Coder-Bloat.
> Nur du, deine Stimme und dein Text.

---

## 1. Northstar

### Was Murmel sein soll

Murmel ist ein **privater, lokaler Speech-to-Text-Diktierassistent** für Desktop-Systeme (Windows & Ubuntu 24.04). Er läuft vollständig offline, verlässt sich auf keine Cloud-Dienste und gehört allein dem Nutzer.

Die zentrale UX-Philosophie ist **Unsichtbarkeit mit Kontrolle**:

- **Unsichtbar:** Murmel ist da, wenn du ihn brauchst, und verschwindet, wenn du ihn nicht brauchst. Kein ständig sichtbares Overlay, kein Bloat.
- **Kontrolle:** Mit einem globalen Hotkey (z. B. `Ctrl + Win + M`) startest du die Diktat-Session. Mit einem zweiten Hotkey (`Ctrl + Shift + H`) öffnest du die Historie. Alles andere passiert im Hintergrund.
- **Sofortigkeit:** Gesprochener Text landet sofort am Cursor — egal in welcher Anwendung. Kein Copy-Paste-Zirkus.

### Was Murmel *nicht* sein soll

- Kein Meeting-Transkriptions-Tool
- Kein AI-Agent mit Chat-Interface
- Kein Cloud-Sync- oder Team-Collaboration-Tool
- Kein Electron-Monster mit 300 MB RAM-Verbrauch
- Kein React-Bloat mit 47 NPM-Dependencies
- Kein Hack (kein DE-Shortcut-Workaround auf Wayland)

---

## 2. Warum ein eigener Clone?

### Das Problem mit WisprFlow

WisprFlow ist ein exzellentes Produkt, aber:

- **Closed Source** — man weiß nicht, was mit den Audiodaten passiert
- **Cloud-Abhängigkeit** — auch "lokale" Modelle laufen oft über deren Infrastruktur
- **Schwere App** — das UI fühlt sich träge an, das Overlay ist nicht verschiebbar
- **Keine echte Kontrolle** — man ist an deren Update-Zyklus und Geschäftsmodell gebunden

### Das Problem mit Vibe-Coder-Clones

Auf GitHub gibt es Dutzende "WisprFlow-Clones", die meisten davon:

- **Abandoned** nach 3 Monaten (z. B. `savbell/whisper-writer`, letzter Commit August 2024)
- **Unzuverlässig** — funktionieren, bis das nächste OS-Update alles bricht
- **Unportabel** — oft nur Linux oder nur Windows
- **Unwartbar** — Python-Skripte ohne Tests, ohne CI, ohne Dokumentation

### Die Lösung: Murmel

Ein **solider, wartbarer, privater Fork** auf Basis bewährter Open-Source-Technologie, der exakt die Features implementiert, die *du* brauchst — und nichts davon wird je an einen Server gesendet.

---

## 3. Basis-Projekt: Handy

### Warum Handy?

| Kriterium | Handy | OpenWhispr | nerd-dictation |
|---|---|---|---|
| **Lizenz** | MIT (permissiv, privat nutzbar) | MIT | GPL v3.0 (Copyleft) |
| **Stack** | Rust (Tauri) + React | Electron + React | Python |
| **Cross-Platform** | ✅ macOS, Windows, Linux | ✅ macOS, Windows, Linux | ❌ nur Linux |
| **Stars** | 22.435 | 3.394 | ~500 |
| **Aktivität** | Sehr aktiv (wöchentliche Commits) | Aktiv | Verlangsamt |
| **Fork-Freundlichkeit** | 🟢 Modularer Rust-Core | 🟡 Komplexer Electron-Stack | 🟢 Einfach |
| **Ressourcen** | 🟢 Gering (Tauri/Rust) | 🟡 Mittel (Electron) | 🟢 Gering |

Handy ist die **einzige Codebase**, die einen soliden, modularen Rust-Core für STT mitbringt, der unter MIT steht und aktiv gepflegt wird.

### Was von Handy übernommen wird

- **`transcribe-cpp`** — Lokale Spracherkennung mit Whisper-Familie (GGML/GGUF)
- **`transcribe-rs`** — CPU-optimierte Erkennung mit Parakeet V3
- **`cpal`** — Cross-Platform Audio I/O
- **`vad-rs`** — Voice Activity Detection (Silero)
- **Text-Injection-Logik** — `xdotool` (X11), `wtype`/`ydotool` (Wayland), native Windows APIs
- **SQLite-History** — Datenbank für Transkriptions-Verlauf
- **Tauri-Grundgerüst** — Cross-Platform Desktop-App Framework

### Was von Handy entfernt wird

- **React-Frontend** — komplett entfernt, ersetzt durch ein natives, lightweight UI
- **Raycast-Extension** — irrelevant für Linux/Windows
- **Homebrew/winget-Pakete** — Murmel wird manuell gebaut und installiert

---

## 4. Architektur-Entscheidungen

### 4.1 Backend: Rust (Tauri)

- **Warum Rust?** Memory-Safety, native Performance, keine Garbage-Collection-Pausen
- **Warum Tauri?** Geringer Footprint (~600 KB Runtime vs. ~150 MB Electron), native System-Integration, Cross-Platform-Abstraktion für Fenster und System-Events

### 4.2 Frontend: Kein React

Statt React wird ein **ultra-leichtgewichtiges UI** verwendet:

- **Option A: Tauri + Vanilla HTML/JS/CSS** — Die Tauri-API erlaubt direkten Zugriff auf Rust-Funktionen aus JS. Ein paar hundert Zeilen vanilla JS reichen für die Historie-Liste.
- **Option B: Tauri + Leptos (Rust-WASM)** — Wenn du komplett auf JS verzichten willst. Leptos ist ein Rust-Frontend-Framework, das zu WASM kompiliert.
- **Empfohlung:** Option A. Für eine einzige Liste mit Kopierknöpfen ist React Overkill. Vanilla JS + CSS reicht völlig.

### 4.3 STT-Engine: Whisper.cpp + Parakeet V3

| Modell | Nutzung | Vor- / Nachteil |
|---|---|---|
| **Whisper (Small/Medium/Large)** | GPU-beschleunigt | Sehr genau, braucht VRAM |
| **Parakeet V3** | CPU-only | Schnell, automatische Spracherkennung, kein GPU nötig |

**Default:** Parakeet V3 (CPU-only, ~5x Echtzeit auf i5). Optional Whisper für maximale Genauigkeit auf Systemen mit GPU.

### 4.4 Audio-Pipeline

```
Mikrofon → cpal → VAD (Silero) → STT (Whisper/Parakeet) → Text-Injection
```

Alles läuft lokal. Kein Byte verlässt den Rechner.

---

## 5. Systemintegration

### 5.1 Globale Hotkeys

| Plattform | Mechanismus | Status |
|---|---|---|
| **Windows** | Tauri `global-shortcut` Plugin | ✅ Nativ, out-of-the-box |
| **Ubuntu 24.04 (X11)** | `rdev` + `xdotool` | ✅ Funktioniert |
| **Ubuntu 24.04 (Wayland)** | **Native D-Bus-Anbindung** | 🟡 Muss gebaut werden |

### 5.2 D-Bus für Wayland — Kein Hack

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

### 5.3 Text-Injection (Pasten)

| Plattform | Mechanismus |
|---|---|
| **Windows** | Native Win32 API (`SendInput` oder Clipboard + `Ctrl+V`) |
| **Linux X11** | `xdotool` oder `enigo` |
| **Linux Wayland** | `ydotool` (systemd-Service) oder `dotool` |

**Wichtig für Ubuntu 24.04:** `wtype` funktioniert auf Ubuntu 24.04 Wayland **nicht**. `ydotool` ist erforderlich und muss als systemd-Service laufen. cite🛠web_search:1#2:~:text=Ubuntu 26.04: Has Wayland display server by default. wtype does not work, you need to install ydotool and configure systemd

Die Text-Injection-Logik aus Handy (`enigo` als Fallback, `xdotool`/`ydotool` als Primary) wird übernommen und an Murmel angepasst.

### 5.4 Overlay — Minimalistisch & Verschiebbar

WisprFlow's Overlay ist nicht verschiebbar — das ist nervig.

Murmel's Overlay:
- **Nur bei aktiver Aufnahme sichtbar**
- **Verschiebbar** per Drag (Tauri-Fenster mit `always-on-top` + `drag` Events)
- **Position wird gespeichert** (per Session oder persistent in SQLite)
- **Optional deaktivierbar** — Audio-Feedback (Piepton) reicht manchen Nutzern

**Design:** Ein kleines, halbtransparentes Widget (z. B. 200x40px) mit einem Mikrofon-Icon und einem kurzen Status-Text ("Aufnahme..." / "Verarbeite..."). Keine React-Komponenten, kein CSS-Framework — pure CSS, ~50 Zeilen.

---

## 6. UI-Philosophie: Ein Fenster, eine Aufgabe

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

## 7. Datenmodell (SQLite)

```sql
CREATE TABLE transcripts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    text        TEXT NOT NULL,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    language    TEXT,           -- erkannte Sprache
    model       TEXT,           -- verwendetes Modell
    duration_ms INTEGER,        -- Diktat-Dauer
    confidence  REAL            -- optional: Konfidenz-Score
);

CREATE INDEX idx_created_at ON transcripts(created_at);
CREATE VIRTUAL TABLE transcripts_fts USING fts5(text);
```

**Privacy:** Die Datenbank liegt lokal unter:
- Windows: `%APPDATA%\Murmel\murmel.db`
- Linux: `~/.config/murmel/murmel.db`

Kein Sync. Kein Backup in die Cloud. Wenn du willst, exportierst du als `.txt` oder `.json`.

---

## 8. Roadmap

### Phase 1: MVP (Woche 1–2)
- [ ] Fork von Handy erstellen
- [ ] React-Frontend entfernen
- [ ] Vanilla-JS-Historie-Fenster bauen
- [ ] Globaler Hotkey für Diktat (Windows: nativ, Linux: D-Bus)
- [ ] Text-Injection (Windows + Linux X11)
- [ ] Parakeet V3 als Default-Modell

### Phase 2: Polish (Woche 3–4)
- [ ] Verschiebbares Overlay
- [ ] D-Bus-Service für Wayland (Ubuntu 24.04)
- [ ] `ydotool`-Integration für Wayland-Pasten
- [ ] Audio-Feedback (Piepton bei Start/Stop)
- [ ] Dark/Light Mode

### Phase 3: Erweiterungen (Woche 5+)
- [ ] Whisper-Modelle (GPU-beschleunigt)
- [ ] Einstellungen-UI
- [ ] Export-Funktion (JSON/TXT)
- [ ] Sprach-spezifische Modelle
- [ ] Optional: Sprachbefehle ("Neuer Absatz", "Punkt", "Komma")

---

## 9. Referenzen & Quellen

### Basis-Projekte

| Projekt | Repo | Lizenz | Stars (Stand: Aug 2026) |
|---|---|---|---|
| **Handy** | `github.com/cjpais/Handy` | MIT | 22.435 |
| **OpenWhispr** | `github.com/OpenWhispr/openwhispr` | MIT | 3.394 |
| **nerd-dictation** | `github.com/ideasman42/nerd-dictation` | GPL v3.0 | ~500 |
| **Speech Note (dsnote)** | `github.com/mkiol/dsnote` | MPL 2.0 | ~2.000 |

### Abandoned Clones (nicht nutzbar)

| Projekt | Repo | Letzter Commit |
|---|---|---|
| **whisper-writer** | `github.com/savbell/whisper-writer` | August 2024 |
| **whisper-dictation** | `github.com/foges/whisper-dictation` | Juni 2024 |

### Technologien

| Komponente | Zweck | Repo |
|---|---|---|
| **Tauri** | Cross-Platform Desktop-Framework | `github.com/tauri-apps/tauri` |
| **whisper.cpp** | Lokale STT-Inference | `github.com/ggerganov/whisper.cpp` |
| **cpal** | Cross-Platform Audio I/O | `github.com/RustAudio/cpal` |
| **enigo** | Cross-Platform Text-Injection | `github.com/enigo-rs/enigo` |
| **rdev** | Globale Hotkeys (X11/macOS/Windows) | `github.com/Narsil/rdev` |
| **Silero VAD** | Voice Activity Detection | `github.com/snakers4/silero-vad` |
| **Parakeet** | NVIDIA CPU-optimiertes STT-Modell | `github.com/NVIDIA/NeMo` |
| **D-Bus (zbus)** | Rust D-Bus-Client/Server | `github.com/dbus2/zbus` |

### Dokumentation

- Handy README & BUILD.md: `github.com/cjpais/Handy`
- Handy Linux Notes (Wayland, ydotool): `github.com/cjpais/Handy#linux-notes`
- Tauri Global Shortcut Plugin: `tauri.app/plugin/global-shortcut/`
- zbus D-Bus Rust Crate: `docs.rs/zbus/latest/zbus/`

---

## 10. Design-Prinzipien (Murmel-Manifest)

1. **Privacy by Design** — Kein Netzwerk-Request, keine Telemetrie, keine Cloud
2. **No Bloat** — Jedes Feature muss sich rechtfertigen. Wenn es nicht essenziell ist, kommt es nicht rein.
3. **No Hacks** — Wayland-Hotkeys über D-Bus, nicht über Workarounds. Text-Injection über offizielle APIs.
4. **One Window** — Ein App-Fenster für alles. Keine Popups, keine Wizard, keine Onboarding-Tours.
5. **Offline First** — Alles funktioniert ohne Internet. Modelle werden einmalig heruntergeladen und bleiben lokal.
6. **Cross-Platform** — Windows und Ubuntu 24.04 sind gleichberechtigte First-Class-Citizens.
7. **Fork-Friendly** — Der Code ist so geschrieben, dass ein zukünftiges Ich (oder jemand anderes) ihn in 2 Jahren noch versteht.

---

> *"Ich murmle, also bin ich."*
>
> — Murmel, 2026
