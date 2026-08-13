# Herkunft & Attribution

Murmel ist ein Fork von **[Handy](https://github.com/cjpais/Handy)** von CJ Pais.

|                    |                                 |
| ------------------ | ------------------------------- |
| **Upstream**       | https://github.com/cjpais/Handy |
| **Fork-Basis**     | Commit `b50b52a`                |
| **Lizenz**         | MIT (Upstream und Fork)         |
| **Fork-Zeitpunkt** | August 2026                     |

Der ursprüngliche Copyright-Vermerk bleibt in [`LICENSE`](LICENSE) erhalten, wie es
die MIT-Lizenz verlangt. Der überwiegende Teil des Codes in `src-tauri/` und `src/`
stammt aus dem Upstream-Projekt.

## Markenrechte und eigenes Branding

Upstream stellt den **Code** unter MIT, behält sich aber **Name, Logo, Icon und
Markenassets** von Handy ausdrücklich vor und verlangt, dass Forks eigenes Branding
verwenden. Murmel setzt das um:

- **Name:** `Murmel` (Produktname, Crate, Binary, Bundle-Identifier `com.suppenterrine.murmel`)
- **Wortmarke:** `src/components/icons/MurmelTextLogo.tsx` — neu gesetzt, keine
  Übernahme der Handy-Pfade
- **App-Icon:** `src-tauri/icons/` — vollständig neu generiert (Murmel-Motiv)
- **Tray-Icons:** `src-tauri/resources/` — vollständig neu generiert; die
  Handy-Motive (Hand / Ohr / Gehirn) wurden nicht übernommen

Murmel steht in keiner Verbindung zu Handy oder CJ Pais und wird von diesen weder
unterstützt noch befürwortet.

## Bewusst nicht umbenannte Bezeichner

Diese Namen enthalten „handy", gehören aber nicht zum Handy-Branding und dürfen
nicht mit umbenannt werden:

| Bezeichner                                    | Was es ist                                             |
| --------------------------------------------- | ------------------------------------------------------ |
| `handy-keys`, `handy_keys`, `HandyKeys`       | Paket von crates.io für globale Hotkeys                |
| `blob.handy.computer`                         | CDN, von dem der Modellkatalog die Modelle lädt        |
| `handy-computer/*`                            | Hugging-Face-Organisation, die die GGUF-Modelle hostet |
| `github.com/cjpais/{vad-rs,rodio,tao,hf-hub}` | Echte Git-Dependencies (Forks von CJ Pais)             |

## Weitere Drittkomponenten

- **Whisper** (OpenAI) — Spracherkennungsmodelle
- **ggml / transcribe.cpp** — Inferenz-Backend
- **Silero VAD** — Voice Activity Detection
- **Parakeet** (NVIDIA NeMo) — CPU-optimiertes STT-Modell
- **Tauri** — Application-Framework

Die jeweiligen Lizenzen gelten unverändert fort.
