# LexiShield - Universal Design System & UX Specification (DESIGN.md)

This document establishes the official design specification, token contract, and aesthetic guidelines for **LexiShield**, adhering to the universal design standards defined in `.agents/rules/core/ui-ux.md`.

---

## 1. Character & Domain Intent

- **Product Identity**: LexiShield is an enterprise-grade, high-assurance data obfuscation and mathematical pseudonymization tool.
- **Aesthetic Tone**: *Technical / Dense, Sober, High-Assurance*.
- **Visual Personality**: Utilitarian precision with crisp visual hierarchy, avoiding decorative distractions while providing clear, unambiguous feedback on sensitive data states.

### Universal Design Dials
```
[Density]     Compact/Dense (0.2) <---------------------------> Expansive/Airy (1.0)
[Motion]      Static/Reduced (0.1) <---------------------------> Cinematic/Spatial (1.0)
[Personality] Utilitarian/System (0.4) <-----------------------> Expressive/Bespoke (1.0)
[Variance]    Uniform/Symmetric (0.2) <------------------------> Asymmetric/Dynamic (1.0)
```

---

## 2. Color Discipline & Palette Token Contract

LexiShield enforces strict 60-30-10 palette discipline:

| Layer | Proportion | Dark Mode | Light Mode | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| **Neutral Base** | 60% | `#020617` (`slate-950`) / `#0f172a` (`slate-900`) | `#f8fafc` (`slate-50`) / `#ffffff` | Background foundation, viewport canvas |
| **Structural Surface** | 30% | `#1e293b`/80 (`brand-surface`) / `border-white/10` | `#ffffff` / `border-slate-200` | Bento cards, panels, table containers |
| **Dynamic Accents** | 10% | Amber `#f59e0b` / Emerald `#10b981` / Indigo `#6366f1` | Amber `#d97706` / Emerald `#059669` | Interactive CTAs, state badges, status toasts |

### Semantic Entity Color Mapping
- **🌐 Red / Network** (`IPv4`, `IPv6`, `Domains`, `URLs`): Blue (`bg-blue-500/10 text-blue-500 border-blue-500/30`)
- **🪪 Identidad** (`DNI`, `NIE`, `Names`, `Users`): Purple (`bg-purple-500/10 text-purple-500 border-purple-500/30`)
- **🖥️ Sistemas** (`SIDs`, `Logon IDs`, `Hex IDs`, `Paths`): Emerald (`bg-emerald-500/10 text-emerald-500 border-emerald-500/30`)
- **💳 Contacto / Claves** (`Emails`, `Telephones`, `Cards`): Amber (`bg-amber-500/10 text-amber-500 border-amber-500/30`)

---

## 3. Typography Scale & Hierarchy

- **Display & Monospace**: `font-mono` (`JetBrains Mono`, `Fira Code`, `ui-monospace`, `Courier New`) for all code, raw logs, hashes, tokens, OIDs, and tabular data.
- **Interface Body**: System UI sans-serif (`system-ui`, `-apple-system`, `Segoe UI`, `Inter`, sans-serif) for navigational chrome, tab labels, headers, and descriptions.
- **Contrast**: Strict WCAG 2.1 AA/AAA compliance; minimum 4.5:1 contrast for all readable body text.

---

## 4. 5-Tier Unified Icon Geometry Scale

To prevent non-deterministic emoji rendering across Linux (Noto Color), Windows (Segoe UI), and macOS (Apple Color Emoji), all glyphs and icons are clamped in explicit geometric containers:

1. **Tier 1 - Action Buttons**: `w-8 h-8 rounded-xl text-sm flex items-center justify-center transition border` (Header controls, clipboard toggle, panel actions).
2. **Tier 2 - Banner & Tip Icons**: `text-base shrink-0` (Contextual guidance banners, toast notifications).
3. **Tier 3 - Modal Header Badges**: `w-10 h-10 rounded-xl text-xl flex items-center justify-center` (Action confirmation dialogs, unlock modals).
4. **Tier 4 - Empty State Badges**: `text-3xl mb-2` (Unpopulated tables, drag & drop dropzones).
5. **Tier 5 - Dismiss Buttons**: `text-xs px-2 py-0.5 rounded` (Onboarding tip closures).

---

## 5. Universal Privacy Surface Masking (`privacyBlur`)

When Privacy Mode is enabled:
- Applies CSS filter `blur(6px)` with smooth unmasking on `:hover`.
- Universally active across source text, detected tokens table ("Valor Original"), manual pair modals, and clipboard history feeds.
- **Zero Leakage Invariant**: Sensitive data is never displayed in plaintext when screen sharing or recording is underway.

---

## 6. Accessibility & Keyboard Navigation

- **Navigation**: Full `Tab`, `Shift+Tab`, `Enter`, and `Space` traversal across tabs, inputs, and modal triggers.
- **Focus Indicators**: 2px high-visibility focus ring (`focus-within:ring-1 focus-within:ring-primary-500`).
- **Color Independence**: Every status or category is always paired with a distinct icon and text label (e.g., `🛡️ Ofuscado` vs `🚫 Omitido`).

