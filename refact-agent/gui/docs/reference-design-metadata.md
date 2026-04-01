# Reference UI Design Metadata

## Typography
- Font family: `Inter`, fallback sans-serif
- Monospace usage: code/output blocks and technical chips
- Primary text sizes:
  - Display/title: `30px`, `18px`
  - Body: `13px`, `14px`
  - Meta/supporting: `10px` to `12px`
  - Micro labels: `8px` to `10px` uppercase
- Font weights: `400`, `500`, `600`, `700`
- Line-height: mostly `1.5`, with relaxed technical text around `1.65` to `1.7`
- Letter spacing:
  - Tight for titles: `-0.02em`
  - Slight emphasis: `0.01em` to `0.02em`
  - Uppercase labels: `0.06em` to `0.10em`

## Color System
- Base dark surfaces:
  - App root: `#0B1120`
  - Header: `#091526`
  - Sidebar: `#091B2A`
  - Deep panel: `#070E1A`
  - Composer: `#111827`
- Text:
  - Primary: `#E2E8F0` / `#F1F5F9`
  - Secondary: `#94A3B8`
  - Muted: `#64748B` / `#475569`
- Primary accent: `#3EC6FF` (cyan)
- Stage accents:
  - Planning: `#9D4EDD` / soft `#C084FC`
  - Code: `#4895EF` / soft `#7BBEFF`
  - Compiling: `#52B788` / soft `#80D4A8`
  - Flashing: `#FFBE0B` / soft `#FFD060`
- Status colors:
  - Success green, warning amber, destructive red

## Layout and Spacing
- Shell layout: top header, left sidebar, central chat workspace, bottom expandable device panels
- Common spacing steps: `4px`, `6px`, `8px`, `10px`, `12px`, `16px`, `20px`, `24px`
- Main content gutters: frequently `64px` for chat rows/composer on desktop-like layouts
- Dense compact control spacing inside toolbars and chips

## Borders, Radius, and Shadows
- Border style: translucent white/cyan overlays (`rgba(...)`) instead of heavy opaque lines
- Radius patterns:
  - small controls: `6px` to `8px`
  - medium cards/inputs: `10px` to `12px`
  - pills: `18px`
  - avatars: `50%`
- Shadow patterns:
  - soft elevation on composer/cards (`0 2px 12px ...`)
  - cyan glow for active/focus CTA states
  - inset glow for terminal/output surfaces

## Components and Interaction Patterns
- Buttons:
  - Filled gradient CTA for important actions
  - Soft translucent chips for secondary toggles
  - Ghost icon buttons for utility actions
- Inputs:
  - Dark background, subtle border
  - Cyan border/focus glow on active state
  - Placeholder in muted slate
- Cards/containers:
  - Dark layered surfaces with low-contrast borders
  - Slight blur and translucent overlays in nav/stepper regions
- Icons:
  - Small size (`11px` to `15px`) with semantic color tint
  - Consistent icon+label pair usage for toggles and tabs

## Theme Tokens and Variables
- Root design tokens driven by CSS variables
- Semantic aliases for:
  - background/foreground/card/popover
  - primary/secondary/accent/destructive
  - border/input/ring
  - stage and sidebar-specific values
- Radius, spacing, font, and shadow tokens expected for reusable styling
