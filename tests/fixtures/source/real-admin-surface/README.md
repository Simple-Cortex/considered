# Admin connections — `/admin/connections`

An IT-admin console for triaging third-party application connections: which are stale-and-active and need review, which are expiring or already expired, and bulk-disabling confirmed-obsolete ones without ever making delete a casual bulk action. Vanilla HTML/CSS/JS, no build step, no backend — one in-memory data source seeded deterministically.

Design record: `.considered/admin-connections/{PRODUCT,FRAME,STRUCTURE,CONTRACT,REVIEW-PACKET,REVIEW.json,DESIGN}.md`. Evidence screenshots: `evidence/`.

## Run it

```
npx serve -s .
```

The `-s` flag enables SPA fallback so a direct load of `/admin/connections` resolves to `index.html`. Without it (e.g. `python3 -m http.server`), only the root `/` resolves without a 404; `app.js` normalizes the URL to `/admin/connections` via `history.replaceState` once it loads, so the app still ends up on the right route.
