# GSV UI Interactivity Plan (band 230)

Джерело: `cargo run --bin gsv-ui-probe` — Firefox headless click-through живого
`http://127.0.0.1:9999/` (geckodriver v0.37.1, `target/live/geckodriver.exe`;
звіт `target/live/ui_probe_report.json`, скриншоти `target/live/ui_probe_*.png`).

## Знахідки аудиту (2026-09-11)

| # | Факт | Дані |
|---|------|------|
| 1 | Uncaught JS / console errors кліків | **0** (усі дії) |
| 2 | Failed HTTP (≥400) із кліків | **0** |
| 3 | Картки на сторінці | 34, title OK |
| 4 | Одинакових сценарних кнопок у tickets-картці | 3×24 = **72** (from-scenario/walk/hook на кожен scenario id) |
| 5 | notify-update → повний resync | 39 fetch на один клік |
| 6 | vision-sync > 900 ms без busy-стану | fetch-обсерватор не встиг (відсутній зворотний зв'язок « триває… ») |
| 7 | Клієнтські JS-помилки | у сервер **не надходять** взагалі (немає каналу) |
| 8 | card-min усередині card-fs | `.collapsed` лишається клас `fullscreen` — UX-артефакт |

## План (PH-S2939… — band 230)

1. **gsv-ui-probe: рахунок помилок** — `js_errors_total` рахувати непорожні `errs`,
   а не записи з ключем; плюс `fetches` рахувати як undercount-safe (не блокує).
2. **Бінарни-обсервер клієнтських помилок** — `POST /api/ui/error` (body
   `{message, source, line}`, redact-free, rate-limited) → кільце останніх 50 у
   `data/gsv/ui_client_errors.jsonl`; health-рядок + картка «UI errors: N».
   UI: `window.addEventListener('error'|'unhandledrejection') → sendBeacon`.
3. **Busy-feedback для async-дій** — делегований клік-хендлер ставить
   `aria-busy`+клас `.busy` на кнопку до завершення промісу; CSS: opacity .55,
   cursor progress. Точково: resync / vision-sync / notify-update / tickets-*.
4. **notify-update: цільове оновлення** — лише `getText("update")` + badge,
   без повного resync (39 fetch → 1–2).
5. **Сценарні кнопки: select + 3 дії** — замість 3×N рендерити один
   `<select id=tixScenario>` + from-scenario/walk/hook без контексту в DOM
   (72 → 3 кнопки; contracts оновити).
6. **card-fs × card-min** — `exitFullscreen()` знімає `.fullscreen` і
   не чіпає collapsed; `collapseCard` у fullscreen-картці спершу виходить з fs.

## Поза межами цього band

- Реальний drag-n-drop layout, віртуальні списки карток, i18n.
- e2e-раннер у CI (пробіг — ручний/SLI, geckodriver не в репо).

##/Gate

fmt · clippy 0 · cargo test · stretch-96 · record-speed/record-rust · sync ·
`gsv-ui-probe` повторний прогін: expect `js_errors_total=0`, `client_errors`
канал чутливий (самотест: beacon 400 → у кільці) · bump --band 230 · fingerprint.
