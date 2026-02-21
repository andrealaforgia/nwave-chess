# SVG Chess Piece Sets for Web Frontend

**Research Date:** 2026-02-20
**Researcher:** Nova (nw-researcher)
**Purpose:** Identify and evaluate SVG chess piece sets for use in a browser-based chess UI for the nwave-chess self-learning chess project.

---

## Executive Summary

There are several high-quality, freely available SVG chess piece sets suitable for a web-based chess UI. The **Cburnett set** (from Wikimedia Commons or via Lichess) is the de facto standard used by Wikipedia, Lichess, python-chess, and most open-source chess projects. For a personal/hobby project, the best options are:

1. **Cburnett (via Lichess repository)** -- GPLv2+ or BSD/GFDL triple-license from Wikimedia; the most recognizable and battle-tested set.
2. **Lichess "Staunton" by James Clarke** -- MIT license; a clean, modern design.
3. **cm-chessboard "Staunty"** -- CC BY-NC-SA 4.0 (non-commercial OK for hobby use); ships ready-to-use as an SVG sprite.

**Recommendation:** Use the **Cburnett pieces from the Lichess GitHub repository** (`public/piece/cburnett/`). They are individual SVG files with a simple naming convention (`wK.svg`, `bQ.svg`, etc.), triple-licensed under permissive terms from the original Wikimedia source (BSD 3-clause option available), and are the most widely recognized chess piece design on the web.

---

## Table of Contents

1. [Comparison Table](#comparison-table)
2. [Detailed Analysis of Each Set](#detailed-analysis)
3. [Integration Considerations](#integration-considerations)
4. [Chess Web Libraries and Their Piece Sets](#chess-web-libraries)
5. [Knowledge Gaps and Limitations](#knowledge-gaps)
6. [Recommendation](#recommendation)
7. [Sources](#sources)

---

## Comparison Table

| Piece Set | License | Format | Source | Style | Ease of Use | Quality |
|-----------|---------|--------|--------|-------|-------------|---------|
| **Cburnett (Wikimedia)** | BSD-3 / GFDL / GPL (triple) | Individual SVGs | Wikimedia Commons | Classic Staunton, shaded | Medium (need to download individually or scrape) | High |
| **Cburnett (via Lichess)** | GPLv2+ | Individual SVGs | [lichess-org/lila GitHub](https://github.com/lichess-org/lila/tree/master/public/piece/cburnett) | Classic Staunton, shaded | **High** (git clone, 12 named files) | High |
| **Lichess "Staunton"** | MIT | Individual SVGs | [lichess-org/lila GitHub](https://github.com/lichess-org/lila/tree/master/public/piece/staunton) | Modern Staunton | High | High |
| **Lichess "Fantasy"** | MIT | Individual SVGs | [lichess-org/lila GitHub](https://github.com/lichess-org/lila/tree/master/public/piece/fantasy) | Decorative/artistic | High | High |
| **Lichess "Spatial"** | MIT | Individual SVGs | [lichess-org/lila GitHub](https://github.com/lichess-org/lila/tree/master/public/piece/spatial) | 3D-effect | High | High |
| **Lichess "Celtic"** | MIT | Individual SVGs | [lichess-org/lila GitHub](https://github.com/lichess-org/lila/tree/master/public/piece/celtic) | Celtic style | High | High |
| **Lichess "Chessnut"** | Apache 2.0 | Individual SVGs | [lichess-org/lila GitHub](https://github.com/lichess-org/lila/tree/master/public/piece/chessnut) | Minimalist | High | Medium-High |
| **Lichess "rhosgfx"** | CC0 1.0 (Public Domain) | Individual SVGs | [lichess-org/lila GitHub](https://github.com/lichess-org/lila/tree/master/public/piece/rhosgfx) | Unknown style | High | Unknown |
| **Lichess "Merida"** | GPLv2+ | Individual SVGs | [lichess-org/lila GitHub](https://github.com/lichess-org/lila/tree/master/public/piece/merida) | Classic Merida font style | High | High |
| **cm-chessboard "Staunty"** | CC BY-NC-SA 4.0 | SVG sprite | [npm cm-chessboard](https://github.com/shaack/cm-chessboard) | Modern Staunton | High (ships with library) | High |
| **cm-chessboard Wikimedia** | CC BY-SA 3.0 | SVG sprite | [npm cm-chessboard](https://github.com/shaack/cm-chessboard) | Cburnett/Wikipedia | High (ships with library) | High |
| **JohnPablok Improved Cburnett** | CC BY-SA 3.0 | SVG + PNG | [OpenGameArt](https://opengameart.org/content/chess-pieces-and-board-squares) | Improved Cburnett | Medium | High |
| **femrek Minimalist** | CC0 (Public Domain) | SVG | [OpenGameArt](https://opengameart.org/content/chess-pieces-in-svg-format) | 2D single-color minimalist | Medium | Low-Medium |
| **Wikimedia Sprite Sheet** | CC BY-SA 3.0 | Single SVG sprite | [Wikimedia Commons](https://commons.wikimedia.org/wiki/File:Chess_Pieces_Sprite.svg) | Cburnett | Low (need to extract pieces) | High |
| **Merida SVG (FelixKling)** | Check repo LICENSE | Individual SVGs | [Codeberg](https://codeberg.org/FelixKling/chess_pieces) | Merida + "Julius" variant | Medium | High |

---

## Detailed Analysis

### 1. Cburnett Pieces (Colin M.L. Burnett)

**The industry standard.** Created by Wikipedia user Cburnett, these are by far the most widely used SVG chess pieces on the internet. They appear on Wikipedia, Lichess (as the default set), python-chess, and dozens of other projects.

**License (Wikimedia Original):**
- Triple-licensed: GFDL 1.2+, BSD 3-clause, and GPL 2+
- The BSD 3-clause option is the most permissive -- it requires only attribution and license retention
- Source: Individual files on Wikimedia Commons, e.g., `Chess_klt45.svg`, `Chess_qdt45.svg`

**License (via Lichess repository):**
- GPLv2+ as listed in Lichess's `COPYING.md`
- For a hobby project, GPLv2+ is perfectly fine -- you just need to keep the code open-source if you distribute it

**Wikimedia File Naming Convention:**
```
Chess_{piece}{color}{background}45.svg

piece:      k=king, q=queen, r=rook, b=bishop, n=knight, p=pawn
color:      l=light(white), d=dark(black)
background: t=transparent, l=light square, d=dark square
45:         45x45 pixel viewbox
```

Example files:
- `Chess_klt45.svg` -- White king on transparent background
- `Chess_qdt45.svg` -- Black queen on transparent background
- `Chess_nlt45.svg` -- White knight on transparent background

**Lichess File Naming Convention (simpler, recommended):**
```
{color}{Piece}.svg

color: w=white, b=black
Piece: K=King, Q=Queen, R=Rook, B=Bishop, N=Knight, P=Pawn
```

The 12 files: `wK.svg`, `wQ.svg`, `wR.svg`, `wB.svg`, `wN.svg`, `wP.svg`, `bK.svg`, `bQ.svg`, `bR.svg`, `bB.svg`, `bN.svg`, `bP.svg`

**Visual Quality:** High. Clean lines, proper shading, good contrast between black and white pieces. Scales well from 20px to 200px+. The white pieces have a light fill with dark outlines; the black pieces have a dark fill with slightly lighter outlines.

**Confidence:** HIGH (5+ independent sources confirm availability, licensing, and quality)

---

### 2. Lichess Piece Sets (Various Authors)

Lichess is an open-source chess server that ships with 30+ piece sets. These are all individual SVG files stored in the `public/piece/{set-name}/` directory of the [lichess-org/lila repository](https://github.com/lichess-org/lila).

**Sets with permissive licenses (safe for hobby projects):**

| Set Name | Author | License | Notes |
|----------|--------|---------|-------|
| cburnett | Colin M.L. Burnett | GPLv2+ | Default set, classic Staunton |
| mono | Duplessis/Burnett | GPLv2+ | Monochrome variant |
| staunton | James Clarke | MIT | Clean modern Staunton |
| fantasy | Maurizio Monge | MIT | Artistic/decorative |
| spatial | Maurizio Monge | MIT | 3D perspective effect |
| celtic | Maurizio Monge | MIT | Celtic-inspired design |
| chessnut | Alexis Luengas | Apache 2.0 | Minimalist and clean |
| merida | A.H. Marroquin | GPLv2+ | Classic Merida design |
| shapes | flugsio | CC BY-SA 4.0 | Abstract/geometric |
| rhosgfx | RhosGFX | CC0 1.0 | Public domain |
| Firi | James Faure | CC BY 4.0 | Only requires attribution |
| kiwen-suwi | neverRare | CC BY 4.0 | Only requires attribution |
| mpchess | Maxime Chupin | GPLv3+ | Copyleft |

**Sets with non-commercial restrictions (check before using):**

| Set Name | License |
|----------|---------|
| horsey, california, caliente, maestro, fresca, cardinal, icpieces, gioco, tatiana, staunty, dubrovny, cooke, monarchy | CC BY-NC-SA 4.0 |
| anarcandy, disguised | CC BY-NC-SA 4.0 |

**Sets with unclear/restrictive licenses:**

| Set Name | License |
|----------|---------|
| alpha | "Free for personal non-commercial use" |
| chess7, companion, leipzig | "Freeware" (vague) |
| reillycraig, riohacha, Staunton 3D | Unspecified |

**Download:** Clone or download from `https://github.com/lichess-org/lila/tree/master/public/piece/`

All sets use the same naming convention: `{color}{Piece}.svg` (e.g., `wK.svg`, `bN.svg`).

**Confidence:** HIGH (Directly verified from Lichess COPYING.md on GitHub)

---

### 3. cm-chessboard Pieces

The [cm-chessboard](https://github.com/shaack/cm-chessboard) library ships with two SVG sprite files:

- **`chessboard-sprite-staunty.svg`** -- Default "Staunty" set (CC BY-NC-SA 4.0 by sadsnake1)
- **`chessboard-sprite.svg`** -- Wikimedia/Cburnett pieces (CC BY-SA 3.0)

**Format:** SVG sprites (all pieces in one file), not individual files. Pieces are identified by element IDs within the sprite: `bp`, `bn`, `bb`, `br`, `bq`, `bk`, `wp`, `wn`, `wb`, `wr`, `wq`, `wk`. Sprites are 40x40px viewBox per piece.

**Integration:** Install via `npm install cm-chessboard` and configure the `props.assetsUrl` to point to the sprite file.

**Confidence:** HIGH (Verified from cm-chessboard GitHub repository and npm)

---

### 4. Wikimedia Commons SVG Chess Pieces (Direct)

The original source of the Cburnett pieces and several other sets. Available at:
- Category page: https://commons.wikimedia.org/wiki/Category:SVG_chess_pieces
- Template page: https://commons.wikimedia.org/wiki/Template:SVG_chess_pieces
- Sprite sheet: https://commons.wikimedia.org/wiki/File:Chess_Pieces_Sprite.svg

**Additional sets on Wikimedia:**
- Colored variants (blue, green, red, yellow)
- Plain black icons (silhouette style)
- Various artistic interpretations

The sprite sheet (`Chess_Pieces_Sprite.svg`) is a single 270x90px SVG containing all 12 standard pieces. Licensed CC BY-SA 3.0.

**Confidence:** HIGH (Directly verified from Wikimedia Commons)

---

### 5. OpenGameArt Sets

**JohnPablok Improved Cburnett** (CC BY-SA 3.0):
- An improved version of the Cburnett set with updated knight, larger king, and consistent rook line widths
- Includes SVG source files plus PNG at multiple sizes
- Download: https://opengameart.org/content/chess-pieces-and-board-squares
- Also includes board square graphics

**femrek Minimalist** (CC0 -- Public Domain):
- Simple 2D, single-color pieces
- Only 6 SVGs (one per piece type, single color only)
- Not a complete 12-piece set -- would require CSS coloring for the second player
- Download: https://opengameart.org/content/chess-pieces-in-svg-format

**Confidence:** MEDIUM (Verified from OpenGameArt listings; less widely adopted than Cburnett/Lichess sets)

---

### 6. Merida SVG Variants

The Merida design (by Armando Hernandez Marroquin, originally a chess font from 1998) is another classic chess piece style.

**Available as SVG from:**
- Lichess repository (`public/piece/merida/`) -- GPLv2+
- Codeberg (FelixKling/chess_pieces) -- Includes "Julius" (Merida redesign) and "Merida Shaded" variants
- npm: `chess-merida-font` -- CSS font approach (not SVG images)

**Confidence:** MEDIUM (Multiple sources confirm existence; exact license for Codeberg variant needs verification from their LICENSE file)

---

## Integration Considerations

### File Format: Individual SVGs vs. Sprite Sheets

| Approach | Pros | Cons |
|----------|------|------|
| **Individual SVGs** (Lichess convention) | Simple to load per-piece; easy to swap sets; cacheable per piece; easy to embed inline | 12 HTTP requests (or inline all) |
| **SVG Sprite Sheet** (cm-chessboard convention) | Single HTTP request; compact | Need to extract pieces via viewBox or `<use>` references; harder to swap individual pieces |
| **Inline SVG in JS** | Zero additional HTTP requests; full CSS control | Increases bundle size; harder to swap sets |

**Recommendation for this project:** Use **individual SVG files**. For a chess UI, 12 small SVG files (typically 2-5KB each) are trivial to load. Individual files offer the most flexibility for swapping sets, applying CSS styles, and caching.

### Naming Conventions Across Libraries

| Library/Source | Convention | Example |
|----------------|-----------|---------|
| **Lichess** | `{color}{Piece}.svg` | `wK.svg`, `bQ.svg` |
| **chessboard.js** | `{color}{Piece}.png` (default) | `wK.png`, `bQ.png` |
| **chessboard-element** | `{color}{Piece}.svg` | `wK.svg`, `bQ.svg` |
| **cm-chessboard** | Element IDs in sprite: `{color}{piece}` | `wk`, `bq` (lowercase) |
| **Wikimedia** | `Chess_{piece}{color}t45.svg` | `Chess_klt45.svg` |
| **gchessboard** | Inline SVG, Cburnett-derived | N/A (embedded) |

The `{color}{Piece}.svg` pattern (e.g., `wK.svg`) is the most common and the one used by Lichess. Adopting this convention will maximize compatibility with chess UI libraries.

### CSS Customization

SVG chess pieces can be styled via CSS when embedded inline or loaded via `<object>` / `<use>`:
- **Fill colors:** Override piece colors for custom themes
- **Stroke:** Adjust outline thickness
- **Opacity/filters:** Add shadows, glow effects
- **Sizing:** SVGs scale infinitely -- set `width`/`height` or use `viewBox`

The Cburnett pieces use hardcoded fill colors in the SVG markup. To make them fully CSS-customizable, you would need to replace inline `fill` attributes with CSS classes -- a one-time modification.

### Scaling Behavior

All the recommended SVG sets scale cleanly from icon size (~16px) to large display (~200px+). SVGs are resolution-independent by nature. The Cburnett pieces maintain visual clarity at all sizes because they use path data rather than raster effects.

---

## Chess Web Libraries and Their Piece Sets

### chessboard.js
- **URL:** https://chessboardjs.com / https://github.com/oakmac/chessboardjs
- **License:** MIT
- **Default pieces:** Wikipedia/Cburnett set in PNG format
- **Piece theme:** Configurable via `pieceTheme` option: `'img/chesspieces/wikipedia/{piece}.png'`
- **SVG support:** Not native -- uses PNG by default; can be pointed at SVG files with custom `pieceTheme`
- **Naming:** `{color}{Piece}` (e.g., `wK`, `bQ`)

### chessboard-element
- **URL:** https://github.com/justinfagnani/chessboard-element
- **License:** MIT
- **Default pieces:** SVG, supports arbitrary piece renderers
- **Format:** Web Component (`<chess-board>`)
- **Naming:** `{color}{Piece}.svg`

### cm-chessboard
- **URL:** https://github.com/shaack/cm-chessboard
- **License:** MIT (code), CC BY-NC-SA 4.0 (Staunty pieces), CC BY-SA 3.0 (Wikimedia pieces)
- **Default pieces:** SVG sprites (Staunty and Wikimedia sets included)
- **Format:** ES6 module, SVG rendered
- **Naming:** SVG sprite with element IDs: `wp`, `bk`, etc.

### gchessboard
- **URL:** https://github.com/mganjoo/gchessboard
- **License:** MIT (code), CC BY-SA 3.0 (Cburnett pieces)
- **Default pieces:** Cburnett pieces adapted from Wikimedia, optimized with SVGO
- **Format:** Web Component, dependency-free, accessible
- **Naming:** Inline SVG

### react-chessboard
- **URL:** https://www.npmjs.com/package/react-chessboard
- **License:** MIT
- **Default pieces:** Wikipedia/Cburnett pieces
- **Format:** React component
- **Naming:** Standard `{color}{Piece}` convention

### python-chess (SVG rendering)
- **URL:** https://python-chess.readthedocs.io/en/latest/svg.html
- **License:** GPL-3.0 (code), BSD/GFDL/GPL triple (Cburnett pieces)
- **Default pieces:** Cburnett pieces built into the library
- **Format:** SVG Tiny 1.2 output

---

## Knowledge Gaps and Limitations

### Gaps Identified

1. **Exact visual comparison:** This research identifies sets by name and description but does not include visual previews. Visiting the Lichess piece selector at https://lichess.org/account/preferences/display or browsing the GitHub directories is recommended for visual comparison.

2. **rhosgfx set quality:** The rhosgfx set on Lichess has the most permissive license (CC0 -- public domain), but its visual style and quality could not be verified from the sources consulted. It may or may not be suitable for a polished UI.

3. **FelixKling Merida SVG license:** The Codeberg repository contains a LICENSE file, but its exact contents were not retrieved. The license needs to be verified directly if this set is chosen.

4. **react-chessboard piece details:** The npm page was inaccessible during research. The piece set details are inferred from related documentation and community reports rather than direct verification.

5. **Exact file sizes:** Individual SVG file sizes for each set were not measured. Based on general knowledge, chess piece SVGs typically range from 1-8KB per file.

6. **Browser compatibility edge cases:** While SVG is universally supported in modern browsers, specific rendering differences (e.g., Safari vs. Chrome gradient handling) were not tested for any of these sets.

---

## Recommendation

### Primary Choice: Cburnett via Lichess Repository

**Download from:** https://github.com/lichess-org/lila/tree/master/public/piece/cburnett

**Why this set:**
- **Most recognized:** Used by Wikipedia, Lichess (default), python-chess, gchessboard, and dozens of other projects
- **Clean naming:** 12 files named `wK.svg`, `wQ.svg`, ..., `bP.svg` -- directly compatible with chessboard.js naming conventions
- **Permissive licensing:** The original Cburnett pieces on Wikimedia are triple-licensed BSD/GFDL/GPL. The Lichess version is GPLv2+. For a hobby/personal project, either is fine. If you want maximum permissiveness, download from Wikimedia and use under the BSD 3-clause option.
- **Battle-tested:** These pieces have been rendered billions of times across the web
- **High quality:** Clean vectors, proper shading, scales perfectly at any size

**How to use:**
```bash
# Clone just the piece directory (sparse checkout) or download the 12 SVGs
# From the lichess-org/lila repository:
mkdir -p public/pieces/cburnett
cd public/pieces/cburnett
# Download each file from:
# https://raw.githubusercontent.com/lichess-org/lila/master/public/piece/cburnett/wK.svg
# ... (repeat for all 12 pieces)
```

Or download all 12 files:
```
wK.svg  wQ.svg  wR.svg  wB.svg  wN.svg  wP.svg
bK.svg  bQ.svg  bR.svg  bB.svg  bN.svg  bP.svg
```

### Alternative Choice: Lichess "Staunton" (MIT License)

If you want a cleaner MIT license with no copyleft concerns:

**Download from:** https://github.com/lichess-org/lila/tree/master/public/piece/staunton

- MIT license (the most permissive common open-source license)
- Modern, clean Staunton design by James Clarke
- Same file naming convention as Cburnett

### Budget/Minimal Choice: CC0 Public Domain

If you want zero licensing concerns whatsoever:

- **rhosgfx** from Lichess: CC0 1.0 -- https://github.com/lichess-org/lila/tree/master/public/piece/rhosgfx
- **femrek minimalist** from OpenGameArt: CC0 -- https://opengameart.org/content/chess-pieces-in-svg-format (note: only 6 files, single color)

---

## Sources

1. Lichess COPYING.md (license reference for all piece sets) -- https://github.com/lichess-org/lila/blob/master/COPYING.md
2. Lichess piece directory on GitHub -- https://github.com/lichess-org/lila/tree/master/public/piece
3. Lichess forum: "Are the Lichess piece sets free to use in other software?" -- https://lichess.org/forum/general-chess-discussion/are-the-lichess-piece-sets-free-to-use-in-other-software
4. Lichess forum: "License cburnett piece set" -- https://lichess.org/forum/lichess-feedback/license-cburnett-piece-set
5. Wikimedia Commons: Category:SVG chess pieces -- https://commons.wikimedia.org/wiki/Category:SVG_chess_pieces
6. Wikimedia Commons: Template:SVG chess pieces (naming convention) -- https://commons.wikimedia.org/wiki/Template:SVG_chess_pieces
7. Wikimedia Commons: Chess Pieces Sprite.svg -- https://commons.wikimedia.org/wiki/File:Chess_Pieces_Sprite.svg
8. python-chess SVG rendering documentation (confirms Cburnett triple license) -- https://python-chess.readthedocs.io/en/latest/svg.html
9. cm-chessboard GitHub repository -- https://github.com/shaack/cm-chessboard
10. gchessboard GitHub repository -- https://github.com/mganjoo/gchessboard
11. chessboard-element GitHub repository -- https://github.com/justinfagnani/chessboard-element
12. chessboard.js GitHub repository -- https://github.com/oakmac/chessboardjs
13. OpenGameArt: Chess Pieces and Board Squares (JohnPablok) -- https://opengameart.org/content/chess-pieces-and-board-squares
14. OpenGameArt: Chess Pieces in SVG Format (femrek) -- https://opengameart.org/content/chess-pieces-in-svg-format
15. FelixKling Merida SVG chess pieces (Codeberg) -- https://codeberg.org/FelixKling/chess_pieces
16. Lichess forum: "Convert 2D chess piece sets to SVG format" -- https://lichess.org/forum/lichess-feedback/convert-2d-chess-piece-sets-to-svg-format
17. chess-merida-font npm/GitHub -- https://github.com/vasiliyaltunin/chess-merida-font
