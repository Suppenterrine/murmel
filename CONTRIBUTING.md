# Contributing to Murmel

Thanks for your interest. This guide covers how to build Murmel and how contributions are handled.

## ⚠️ Read this first

**Murmel is a personal fork of [Handy](https://github.com/cjpais/Handy), maintained by one person for their own use.** That has consequences:

- **The [Northstar](Murmel_Northstar.md) decides.** A feature can be well-built and still be rejected because it does not fit the vision. Please open an issue before investing time in a feature PR.
- **Bug fixes and platform fixes (Windows, Ubuntu 24.04) are the most welcome contributions.**
- **Bugs in upstream code belong upstream.** If a bug also exists in Handy, reporting it at [cjpais/Handy](https://github.com/cjpais/Handy/issues) helps more people than fixing it only here.

## 📖 Philosophy

Murmel is a private, local dictation assistant — nothing more, deliberately. Priorities:

- **Privacy**: Local by default. No telemetry, no cloud sync, no analytics.
- **Restraint**: Every feature and every dependency has to justify itself.
- **Speed on Windows**: The daily driver sets the pace; Ubuntu 24.04 is carried along, not retrofitted.
- **Maintainability**: Code a future self still understands in two years.

## 🚀 Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

- [Rust](https://rustup.rs/) (latest stable)
- [Bun](https://bun.sh/) package manager
- Platform-specific build tools (see [BUILD.md](BUILD.md))

### Setting Up Your Development Environment

1. **Fork the repository** on GitHub

2. **Clone your fork**:

   ```bash
   git clone git@github.com:YOUR_USERNAME/Murmel.git
   cd Murmel
   ```

3. **Add upstream remote**:

   ```bash
   git remote add upstream git@github.com:Suppenterrine/murmel.git
   ```

4. **Install dependencies**:

   ```bash
   bun install
   ```

5. **Download required models**:

   ```bash
   mkdir -p src-tauri/resources/models
   curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
   ```

6. **Run in development mode**:
   ```bash
   bun run tauri dev
   # On macOS if you encounter cmake errors:
   CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev
   ```

For detailed platform-specific setup instructions, see [BUILD.md](BUILD.md).

### Understanding the Codebase

Murmel follows a clean architecture pattern:

**Backend (Rust - `src-tauri/src/`):**

- `lib.rs` - Main application entry point with Tauri setup
- `managers/` - Core business logic (audio, model, transcription)
- `audio_toolkit/` - Low-level audio processing (recording, VAD)
- `commands/` - Tauri command handlers for frontend communication
- `shortcut.rs` - Global keyboard shortcut handling
- `settings.rs` - Application settings management

**Frontend (React/TypeScript - `src/`):**

- `App.tsx` - Main application component
- `components/` - React UI components
- `hooks/` - Reusable React hooks
- `lib/types.ts` - Shared TypeScript types

For more details, see the Architecture section in [README.md](README.md) or [AGENTS.md](AGENTS.md).

## 🐛 Reporting Bugs

### Before Submitting a Bug Report

1. **Search existing issues** at [github.com/Suppenterrine/murmel/issues](https://github.com/Suppenterrine/murmel/issues)
2. **Check discussions** at [github.com/Suppenterrine/murmel/discussions](https://github.com/Suppenterrine/murmel/discussions)
3. **Try the latest release** to see if the issue has been fixed
4. **Enable debug mode** (`Cmd/Ctrl+Shift+D`) to gather diagnostic information

### Submitting a Bug Report

When creating a bug report, please include:

**System Information:**

- App version (found in settings or about section)
- Operating System (e.g., macOS 14.1, Windows 11, Ubuntu 22.04)
- CPU (e.g., Apple M2, Intel i7-12700K, AMD Ryzen 7 5800X)
- GPU (e.g., Apple M2 GPU, NVIDIA RTX 4080, Intel UHD Graphics)

**Bug Details:**

- Clear description of the bug
- Steps to reproduce
- Expected behavior
- Actual behavior
- Screenshots or logs if applicable
- Information from debug mode if relevant

Use the [Bug Report template](.github/ISSUE_TEMPLATE/bug_report.md) when creating an issue.

## 💡 Suggesting Features

1. **Read the [Northstar](Murmel_Northstar.md) first** — especially "Was Murmel _nicht_ sein soll" (§1) and the roadmap (§11). It will answer most questions about whether an idea fits.
2. **Open an issue** describing:
   - The problem you are trying to solve (not just the solution you have in mind)
   - Which Northstar section it supports
   - Alternatives you considered
3. **Wait for a reply before writing code.** A rejected feature PR wastes your time, not mine.

## 🔧 Making Code Contributions

### Before You Start

**This is critical:** Before writing any code, please do the following:

1. **Search existing issues and PRs** - Check both open AND closed issues and pull requests. Someone may have already addressed this, or there may be a reason it was closed.
   - [Open issues](https://github.com/Suppenterrine/murmel/issues)
   - [Closed issues](https://github.com/Suppenterrine/murmel/issues?q=is%3Aissue+is%3Aclosed)
   - [Open PRs](https://github.com/Suppenterrine/murmel/pulls)
   - [Closed PRs](https://github.com/Suppenterrine/murmel/pulls?q=is%3Apr+is%3Aclosed)

2. **If something was previously closed** - If you want to revisit a closed issue or PR, you need to:
   - Provide a strong argument for why it should be reconsidered
   - Gather community feedback first via [Discussions](https://github.com/Suppenterrine/murmel/discussions)
   - Link to that discussion in your PR

3. **Get community feedback for features** - PRs with demonstrated community interest are **much more likely to be merged**. Start a discussion, get feedback, and link to it in your PR. This helps ensure Murmel stays focused and useful for the most people without becoming bloated.

Community feedback is essential to keeping Murmel the best it can be for everyone. It helps prioritize what matters most and prevents feature creep.

### Development Workflow

1. **Create a feature branch**:

   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   ```

2. **Make your changes**:
   - Write clean, maintainable code
   - Follow existing code style and patterns
   - Add comments for complex logic
   - Keep commits focused and atomic

3. **Test thoroughly**:
   - Test on your target platform(s)
   - Verify existing functionality still works
   - Test edge cases and error conditions
   - Use debug mode to verify audio/transcription behavior

4. **Commit your changes**:

   ```bash
   git add .
   git commit -m "feat: add your feature description"
   # or
   git commit -m "fix: describe the bug fix"
   ```

   Use conventional commit messages:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `refactor:` for code refactoring
   - `test:` for test additions/changes
   - `chore:` for maintenance tasks

5. **Keep your fork updated**:

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

6. **Push to your fork**:

   ```bash
   git push origin feature/your-feature-name
   ```

7. **Create a Pull Request**:
   - Go to the [Murmel repository](https://github.com/Suppenterrine/murmel)
   - Click "New Pull Request"
   - Select your fork and branch
   - Fill out the PR template completely, including:
     - Clear description of changes
     - Links to related issues or discussions
     - **Community feedback** (especially important for features)
     - How you tested the changes
     - Screenshots/videos if applicable
     - Breaking changes (if any)

   **Remember:** PRs with community support are prioritized. If you haven't already, start a [discussion](https://github.com/Suppenterrine/murmel/discussions) to gather feedback before or alongside your PR. It is not explicitly required to gather feedback, but it certainly helps your PR get merged faster.

### AI Assistance Disclosure

**AI-assisted PRs are welcome!** Use whatever tools help you contribute, just be upfront about it.

In your PR description, please include:

- Whether AI was used (yes/no)
- Which tools were used (e.g., "Claude Code", "GitHub Copilot", "ChatGPT")
- How extensively it was used (e.g., "generated boilerplate", "helped debug", "wrote most of the code")

### Code Style Guidelines

**Rust:**

- Follow standard Rust formatting (`cargo fmt`)
- Run `cargo clippy` and address warnings
- Use descriptive variable and function names
- Add doc comments for public APIs
- Handle errors explicitly (avoid unwrap in production code)

**TypeScript/React:**

- Use TypeScript strictly, avoid `any` types
- Follow React hooks best practices
- Use functional components
- Keep components small and focused
- Use Tailwind CSS for styling

**General:**

- Write self-documenting code
- Add comments for non-obvious logic
- Keep functions small and single-purpose
- Prioritize readability over cleverness

### Testing Your Changes

**Manual Testing:**

- Run the app in development mode: `bun run tauri dev`
- Test your changes with debug mode enabled
- Verify on multiple platforms if possible
- Test with different audio devices
- Try various transcription scenarios

**Building for Production:**

```bash
bun run tauri build
```

Test the production build to ensure it works as expected.

## 📝 Documentation Contributions

Documentation improvements are highly valued! You can contribute by:

- Improving README.md, BUILD.md, or this CONTRIBUTING.md
- Adding code comments and doc comments
- Creating tutorials or guides
- Improving error messages
- Updating the project website content

## 🤝 Community Guidelines

- **Be respectful and inclusive** - We welcome contributors of all skill levels
- **Be patient** - This is maintained by a small team, responses may take time
- **Be constructive** - Focus on solutions and improvements
- **Be collaborative** - Help others and share knowledge
- **Search first** - Check existing issues/discussions before creating new ones

## 🎯 Good First Issues

Look for issues labeled `good first issue` or `help wanted` if you're new to the project. These are typically:

- Well-defined and scoped
- Good for learning the codebase
- Mentor support available

## 📞 Getting Help

- **Discord**: Join our [Discord community](https://discord.com/invite/WVBeWsNXK4)
- **Discussions**: Ask questions in [GitHub Discussions](https://github.com/Suppenterrine/murmel/discussions)
- **Email**: Reach out at [contact@handy.computer](mailto:contact@handy.computer)

## 📜 License

By contributing to Murmel, you agree that your contributions will be licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

**Thank you for contributing to Murmel!** Your efforts help make speech-to-text technology more accessible, private, and extensible for everyone.
