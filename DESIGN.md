---
name: Cardinal
description: Instant, understandable file search for macOS
colors:
  accent: "oklch(58% 0.19 255)"
  accent-hover: "oklch(52% 0.18 255)"
  canvas-light: "oklch(98% 0.004 255)"
  surface-light: "oklch(100% 0.003 255)"
  text-light: "oklch(25% 0.012 255)"
  muted-light: "oklch(49% 0.018 255)"
  border-light: "oklch(88% 0.012 255)"
  canvas-dark: "oklch(20% 0.012 255)"
  surface-dark: "oklch(24% 0.014 255)"
  text-dark: "oklch(93% 0.009 255)"
  muted-dark: "oklch(70% 0.014 255)"
  border-dark: "oklch(34% 0.016 255)"
  success: "oklch(55% 0.13 155)"
  warning: "oklch(66% 0.14 65)"
  danger: "oklch(56% 0.19 25)"
typography:
  title:
    fontFamily: "-apple-system, BlinkMacSystemFont, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 650
    lineHeight: 1.25
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.4
  label:
    fontFamily: "-apple-system, BlinkMacSystemFont, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 550
    lineHeight: 1.25
rounded:
  sm: "6px"
  md: "9px"
  lg: "12px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.surface-light}"
    rounded: "{rounded.md}"
    padding: "7px 12px"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
    textColor: "{colors.surface-light}"
    rounded: "{rounded.md}"
    padding: "7px 12px"
  input-search:
    backgroundColor: "{colors.surface-light}"
    textColor: "{colors.text-light}"
    rounded: "{rounded.lg}"
    height: "38px"
---

# Design System: Cardinal

## 1. Overview

**Creative North Star: "The Mac File Lens"**

Cardinal is used by someone who has just created or downloaded a file and needs it now, often while other applications are active and the filesystem is changing. The interface follows the system appearance, stays visually quiet, and puts search, sort, filters, and direct actions within one glance.

The design rejects command-driven everyday search, decorative dashboard layouts, nested cards, and modal-first workflows. Dense results are welcome because density serves scanning, but controls remain labeled, predictable, and keyboard accessible.

**Key Characteristics:**

- Native-feeling system typography and controls
- Restrained cool neutrals with one functional blue accent
- Dense, virtualized data rows with generous interaction targets
- Progressive filters with visible active state
- Motion limited to 150 to 200 ms state feedback

## 2. Colors

The palette uses lightly blue-tinted neutrals so the utility feels calm without becoming sterile.

### Primary

- **Cardinal Action Blue** (`oklch(58% 0.19 255)`): Primary actions, selected filters, focus rings, and active sort state.

### Neutral

- **Day Canvas** (`oklch(98% 0.004 255)`): Main light background.
- **Day Surface** (`oklch(100% 0.003 255)`): Search field, toolbar, popovers, and raised controls.
- **Night Canvas** (`oklch(20% 0.012 255)`): Main dark background.
- **Night Surface** (`oklch(24% 0.014 255)`): Dark raised controls and panels.
- **Quiet Border** (`oklch(88% 0.012 255)` light, `oklch(34% 0.016 255)` dark): Dividers and field boundaries.

### Named Rules

**The Functional Accent Rule.** Blue indicates action, focus, selection, or active state. It is never decorative.

## 3. Typography

**Display Font:** macOS system sans-serif
**Body Font:** macOS system sans-serif
**Label/Mono Font:** SF Mono only for technical paths or syntax previews

**Character:** Compact, familiar, and neutral. Hierarchy comes from weight and spacing, not oversized headings.

### Hierarchy

- **Title** (650, 15px, 1.25): Panel and settings section titles.
- **Body** (400, 13px, 1.4): Result metadata, descriptions, and settings help.
- **Label** (550, 12px, 1.25): Buttons, chips, column headings, and field labels.

### Named Rules

**The Scan First Rule.** Filenames get strongest emphasis, paths are quieter, and dates and sizes align for rapid comparison.

## 4. Elevation

Cardinal is flat by default. Toolbars and result tables use tonal separation and one-pixel borders. Shadows are reserved for transient popovers and the settings sheet.

### Shadow Vocabulary

- **Popover** (`0 10px 30px oklch(20% 0.02 255 / 0.16)`): Filter menus and action menus only.
- **Overlay** (`0 24px 64px oklch(15% 0.02 255 / 0.24)`): Settings and permission sheets.

### Named Rules

**The Flat at Rest Rule.** Persistent surfaces do not float. Elevation communicates temporary hierarchy.

## 5. Components

### Buttons

- **Shape:** Compact rounded rectangle, 9px radius.
- **Primary:** Action blue with high-contrast text and 7px by 12px padding.
- **Hover / Focus:** Slightly darker blue on hover; 2px visible focus ring with 2px offset.
- **Secondary / Ghost:** Neutral surface or transparent background with clear hover fill.

### Chips

- **Style:** Pill shape, quiet neutral background, short label, optional icon, and a direct remove affordance.
- **State:** Active filters use a tinted blue surface and never rely on color alone; they also show a check or remove icon.

### Cards / Containers

- **Corner Style:** Cards are avoided for primary layout. Popovers and grouped settings use 12px radius.
- **Background:** Tonal surfaces with one-pixel borders.
- **Shadow Strategy:** Only transient surfaces use shadow.
- **Internal Padding:** 12px to 16px depending on density.

### Inputs / Fields

- **Style:** 38px search field, 12px radius, visible boundary, leading search icon, and clear button.
- **Focus:** Action-blue ring plus preserved border contrast.
- **Error / Disabled:** Text, icon, and color communicate state together.

### Navigation

Files and Events remain adjacent tabs in the bottom status area or compact toolbar. The active tab uses weight, color, and an indicator. Search scope and filters live beside the search field, not in application menus.

### Result Table

The result table is the signature component. It supports sortable headers, a persistent sort indicator, virtualized rows, keyboard selection, Quick Look, contextual actions, and a compact metadata rhythm. A recent-files preset appears as a visible action rather than requiring an empty query plus manual sorting.

## 6. Do's and Don'ts

### Do:

- **Do** keep plain text search as the default and translate UI filters into query syntax internally.
- **Do** make date sorting and the Recent view immediate at million-file scale.
- **Do** preserve familiar macOS shortcuts, selection, Quick Look, and Trash behavior.
- **Do** show progress, cancellation, permission errors, and index health in context.
- **Do** provide 44px effective pointer targets where controls are isolated, even when their visual height is compact.

### Don't:

- **Don't** require Everything-style command syntax in the standard search field.
- **Don't** hide important controls inside unlabeled toggles or raw multiline settings fields.
- **Don't** block the interface while sorting or indexing a full result set.
- **Don't** use decorative dashboards, nested cards, glassmorphism, gradient text, or excessive motion.
- **Don't** make Finder a required second step for routine file actions.
