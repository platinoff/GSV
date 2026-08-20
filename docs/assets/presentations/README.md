# README presentations (SMIL SVG)

GitHub strips JavaScript and external CSS from Markdown. Animation that **plays on the repo page** lives in standalone `.svg` files referenced with `<img>`.

Research this band used:

| Source | What we took |
|--------|----------------|
| [WaterTian/svg-motion-cookbook](https://github.com/WaterTian/svg-motion-cookbook) | Pure **SMIL** (`<animate>` / `<animateTransform>`). No JS. Loop+hold, stagger, clip-path reveal, discrete cursor, heartbeat. Static attributes stay readable if a scraper strips SMIL. |
| [awesome-github-profile](https://github.com/beydemirfurkan/awesome-github-profile) | One focal motion, not a carnival. Neon pulse + typewriter as the useful ceiling. |
| GSV Galaxy palette | `#06080f` / `#7eb8ff` / `#c4a5ff` / `#22d3ee` — same as `boxes/vision.rs` starfield/galaxy SVG. |

Do **not** add GIFs or PNG screenshots here unless they are the live Galaxy UI (those files were never committed; the old README 404'd). Charts in the product stay Rust-rendered at `/api/vision/*.svg`.

| File | Role |
|------|------|
| [`gsv-hero.svg`](./gsv-hero.svg) | Wordmark wipe + nebula + live `:9999` |
| [`gsv-install.svg`](./gsv-install.svg) | Four install steps |
| [`gsv-flow.svg`](./gsv-flow.svg) | What to do after the server is up |
