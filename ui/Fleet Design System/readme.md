# Fleet — Design System

**Fleet** (working name; the game is currently unnamed) is a competitive 2D real-time strategy game about
commanding a massive space fleet. Thousands of autonomous drones flow and fight around a small number of
powerful capital ships, and a few well-chosen commands translate strategic intent into action at enormous
scale. A match is decided by the **Shield Relay** — an 8-second capture objective that drops the opposing
**Command Core**'s shield.

The design system is named **"Calm Command, Sharp Commitment"**: the interface is quiet and sparse while a
player is positioning, and becomes precise and high-contrast the moment a fleet commits to warp or an
objective is contested.

## Sources

This system was authored **from a written brief only**. There was no attached codebase, Figma file, deck,
screenshot, or font/logo package. Everything here — tokens, components, the UI kit — is an original
interpretation of that brief and should be treated as a first proposal, not a recreation of shipped work.

- Brief: "Design a 21:9 ultrawide HUD for a competitive 2D space RTS using a *Calm Command, Sharp Commitment*
  design system" (provided in chat, July 2026).
- Product surfaces described in the brief: exactly one — the **in-match ultrawide HUD**. No marketing site,
  launcher, menus, or companion app were specified, so none were invented.

### Substitutions and gaps (please correct these)
- **Fonts.** No binaries were supplied. `tokens/fonts.css` loads the nearest Google Fonts matches —
  **Barlow Condensed** (condensed UI / labels), **Barlow** (body), **IBM Plex Mono** (numeric readouts).
  Swap in licensed binaries and rewrite that file as `@font-face` rules when the real faces exist.
- **Logo.** None supplied, and none was invented. The wordmark is plain type (see the *Wordmark* brand card).
- **Icons.** No icon set supplied. Interface affordances use **Lucide** (`lucide-static@0.544.0`, ISC),
  copied into `assets/icons/` and inlined in the `Icon` component so they colour from `currentColor` and
  need no network. Flagged as a substitution — replace with the studio's own set when one exists.
- **Copy.** All in-game strings here are placeholders written in the brand voice; no real localisation
  strings were available.

---

## Index

| Path | What it is |
|---|---|
| `styles.css` | Global entry point — `@import` list only. Link this one file. |
| `tokens/` | `fonts`, `colors`, `typography`, `spacing`, `lines`, `motion`, `base` |
| `components/` | React primitives (below) |
| `assets/icons/` | The 14 inlined Lucide interface icons, as source SVGs |
| `guidelines/` | Foundation specimen cards (Colors, Type, Spacing, Brand) |
| `ui_kits/hud/` | The 21:9 in-match HUD recreation — see its `README.md` |
| `thumbnail.html` | Homepage tile |
| `SKILL.md` | Agent Skills wrapper |

### Components

**`components/surfaces/`** — `HudPanel`, `Hairline`, `SelectionBrackets`
**`components/signals/`** — `CaptureRing`, `Meter`, `Readout`, `ConfidenceTag`, `StateDot`, `Glyph`
**`components/fleet/`** — `FleetChip`, `DoctrinePill`, `FrontRow`, `OrderStrip`
**`components/controls/`** — `ObjectiveBar`, `CommandButton`, `IconButton`
**`components/icons/`** — `Icon`

**Intentional additions.** The brief described visual treatments rather than a component inventory, so the
list above is derived from it directly: every component maps to a named element of the brief (capture ring,
doctrine pills, fleet chips, order strip, front roster, corner brackets, geometric glyphs, top bar). Two
utility wrappers were added: `Icon` (a Lucide wrapper, so interface icons are consistent and colourable)
and `Hairline` (the divider is used everywhere and was worth naming). No generic library primitives —
Toast, Avatar, Tabs, Modal — were added; Fleet has no place for them.

---

## Content fundamentals

Fleet's copy is **instrument copy**. It reads like a readout on a machine that is telling you the truth as
fast as it can.

- **Casing.** ALL-CAPS for every label, state word and unit (`DRONES`, `CONTESTED`, `WARP COMMITTED`),
  with `.14em` tracking. Sentence case only in tutorial/prose surfaces. Never Title Case.
- **Person.** No "I". No "you" in HUD chrome — the HUD does not address the player, it reports state:
  "WARP COMMITTED", not "Your warp is committed". "You" is allowed only in tutorial text and the core
  ownership chips (`YOU` / `OPP`), where brevity wins.
- **Length.** One to three words per label. A status phrase is at most four: `INBOUND 12s`,
  `CAPTURE PAUSED`, `RELAY DECAYING`, `ENEMY SEEN 32s AGO`.
- **Verbs.** Present continuous for ongoing state (`CAPTURING`, `DECAYING`, `ENGAGED`), past participle
  for settled state (`COMMITTED`, `HELD`, `SHIELDED`, `EXPOSED`).
- **Honesty about uncertainty.** Never state an estimate as a fact and never invent precision. Confirmed:
  `3 CAPITALS`. Estimated: `~3 CAPITALS` with a dashed tag. Old: `SEEN 32s AGO`. Unknown:
  `UNSCOUTED` — never `0`. This is the single strongest copy rule in the system.
- **Numbers.** Always tabular mono. Times carry a unit (`18.0s`, `T+04:12`); counts use thousands
  separators (`2,480`); percentages are whole numbers.
- **Naming.** Fleets are two-digit numbers (`04`, `11`). Systems get short two-word proper names
  (`Kestrel Gate`, `Meridian Gate`, `Vantage Outpost`, `Shield Relay`, `Command Core`). Fronts are
  named after the system they contest.
- **No emoji. Ever.** No exclamation marks, no flavour text in the HUD, no encouragement, no personality.
  The one place voice is permitted is out-of-match writing (patch notes, store copy), which stays terse and
  technical: "Relay decay now begins after 1.0s, down from 2.5s."
- **Vibe.** A flight instrument, not a game menu. If a string could appear on a mobile game card, delete it.

---

## Visual foundations

**Colour.** Near-black void (`#04060A`–`#0B0F15`) under everything; translucent graphite panels above it.
Exactly three signal colours, each with one meaning: **cyan `#22CFE8` = friendly**, **coral `#FF6A47` =
enemy**, **gray `#98A4AE` = neutral / unavailable / stale / uncertain**. Nothing else in the interface may
be coloured — no brand accent, no success green, no warning amber. Text is a four-step neutral ramp plus a
dedicated `--text-stale` for information the player should distrust.

**Treatments carry meaning; they are not decoration.**
Solid cyan/coral = confirmed ownership, active capture, committed route.
Diagonal stripes in the owner's colour = contested objective, progress paused.
Gray diagonal stripes / gray reverse-drain = relay progress decaying after the capturing team left.
Dashed outlines and dashed lines = uncertain scouting, estimated composition, planned-but-unconfirmed order.
Solid arrowed line + lock = irreversible warp commitment.
Thin corner brackets = selection, and nothing else.

**Type.** Barlow Condensed for labels and titles (condensed reads wide across an ultrawide at small sizes);
IBM Plex Mono with `tabular-nums` for every number; Barlow for the rare sentence of prose. Ramp:
10 / 11 / 13 / 15 / 18 / 28 / 44px — the HUD itself lives almost entirely at 11–15px. Labels sit **above**
their value, never beside it, never with a colon.

**Spacing and layout.** 4px base; chrome uses 4–16 only. The HUD is a frame: a 34px top bar and panels
pinned to the four corners inside a 14px edge margin. The centre of the screen is playfield and stays
clear — **no permanent element may overlap it**. Panel widths are fixed (280px side panels, 520px order
strip) so the player's eye learns where information lives. Anything about an objective is drawn *on* the
objective in the world, not mirrored into a panel.

**Backgrounds.** Flat near-black, an 80px hairline grid at 7% opacity, and one very soft radial lift around
the active objective. No imagery, no illustration, no texture, no nebulae, no stars-as-decoration, no
gradients other than that single radial and the meaning-bearing stripe patterns.

**Borders, radii, depth.** 1px hairlines at 18% (34% when a rule needs to carry). Radii are near-square:
1px for panels and rows, 2px for chips and pills, full round only on meter caps and dots. **No drop
shadows** — depth comes from an 82%-opaque graphite fill with `blur(10px) saturate(120%)` behind it, plus
a 1px black ring to separate the panel from the starfield. Cards, in the web sense, do not exist here:
containers are outlined panels, never elevated surfaces.

**Transparency and blur.** Panels: 82% opacity + backdrop blur, always. Fleet markers over the map: 72%
flat fill, no blur (blur at map scale costs frames). Never blur the playfield itself.

**Motion.** Instrument motion. Progress and drain are strictly **linear** — a capture bar must be readable
as time. State changes are 80–220ms with `cubic-bezier(.2,.6,.3,1)`. No bounce, no spring, no scale-in, no
parallax. Exactly one looping animation exists in the system: the 1400ms opacity pulse on
`WARP COMMITTED`, because that is the one moment worth interrupting a player's attention for.

**Hover, press, focus, disabled.** Hover lifts the row background to `rgba(255,255,255,.05)` and text one
step up the neutral ramp — never a colour change, since colour is allegiance. Press deepens the background
to `--graphite-3`; nothing scales or moves. Selected uses the cyan wash + cyan hairline (+ corner brackets
where space allows). Focus is a 1px cyan outline. Disabled is 38% opacity with no colour change.

**Imagery.** There is none. If key art is ever needed outside the HUD, it should be cold, desaturated, and
near-monochrome with a single cyan or coral light source — never warm, never lens-flared.

---

## Iconography

Two distinct, non-overlapping systems. Mixing them is a bug.

1. **Tactical glyphs — `Glyph`.** In-world meaning: system type (`core`, `relay`, `gate`, `outpost`),
   fleet composition (`drone`, `capital`), stance (`aggressive`, `defensive`, `screen`), formation
   (`wedge`, `dispersed`, `screen`), target priority (`priorityCapital`), movement (`warp`), and
   `unknown`. These are **geometric primitives only** — circles, triangles, diamonds, dots, chevrons —
   drawn at 1.25px stroke with square caps and mitred joins, on a square box, at 10–26px. They inherit
   allegiance colour. They are deliberately abstract: a player must be able to read a dozen of them in
   peripheral vision. Nothing pictographic, nothing illustrated, nothing with a perspective.
2. **Interface icons — `Icon`.** Affordances only: `lock`, `unlock`, `shield`, `shield-off`, `radar`,
   `crosshair`, `chevron-right`, `x`, `eye-off`, `git-branch`, `clock`. Sourced from **Lucide** v0.544.0 (ISC),
   copied into `assets/icons/` and inlined as SVG in `Icon.jsx` so they always paint in `currentColor`
   with no CDN dependency. 2px stroke, round caps, 10–18px. **This is a substitution** — no icon set was supplied
   with the brief; replace with the studio's own set when one exists.

**No emoji, anywhere.** No unicode characters used as icons (no `▲`, `✕`, `→` in strings) — every mark
is a `Glyph` or an `Icon`. No raster icons. No icon font.

The only mark that is ever "the logo" is the word **FLEET** set in Barlow Condensed 700, uppercase, `.22em`
tracking. There is no symbol, and one must not be drawn until the studio supplies it.
