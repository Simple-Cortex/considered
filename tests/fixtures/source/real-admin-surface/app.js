// admin-connections — /admin/connections
// See .considered/admin-connections/{FRAME,STRUCTURE,CONTRACT}.md for the design
// record this build implements. Contract is also embedded as a comment at the
// top of index.html.

// ------------------------------------------------------------------
// Deterministic RNG (mulberry32) so evidence captures are reproducible.
// ------------------------------------------------------------------
function mulberry32(seed) {
  let a = seed;
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const rng = mulberry32(46);
function pick(arr) {
  return arr[Math.floor(rng() * arr.length)];
}
function pickWeighted(pairs) {
  const total = pairs.reduce((s, p) => s + p[1], 0);
  let r = rng() * total;
  for (const [value, weight] of pairs) {
    if (r < weight) return value;
    r -= weight;
  }
  return pairs[pairs.length - 1][0];
}
function sampleUnique(arr, n) {
  const pool = arr.slice();
  const out = [];
  for (let i = 0; i < n && pool.length; i++) {
    const idx = Math.floor(rng() * pool.length);
    out.push(pool.splice(idx, 1)[0]);
  }
  return out;
}
function slugify(s) {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
}

// ------------------------------------------------------------------
// Data
//
// The 5 seed records below reproduce the brief's supplied facts
// exactly, including their stated uncertainty (Legacy Forms has no
// owner and an expiry described only as "expired," never a day
// count). The remaining 41 records are synthetic, schema-faithful
// instances generated to satisfy the brief's stated total of 46 —
// same six fields, plausible ranges, no invented company facts
// (FRAME.md A2 / craft.md HON-03).
// ------------------------------------------------------------------

const SCOPES_POOL = [
  "contacts.read",
  "contacts.write",
  "deals.read",
  "deals.write",
  "payroll.read",
  "forms.write",
  "calendar.read",
  "email.send",
  "files.read",
  "webhooks.write",
];

const OWNER_POOL = [
  "Lila Chen",
  "Arun Shah",
  "Dana Cole",
  "Priya Nair",
  "Marcus Webb",
  "Sofia Torres",
  "Jordan Lee",
  "Grace Kim",
  "Omar Haddad",
  "Elena Petrova",
  "Devon Brooks",
];

const SYNTHETIC_APP_NAMES = [
  "GridMail Relay",
  "PulseDesk Sync",
  "Ledger Bridge",
  "Fieldwire Connect",
  "Northstar Reports",
  "Candid Forms",
  "Loop Analytics",
  "Warehouse Beacon",
  "Ticketron Bridge",
  "Cobalt Billing",
  "Meridian Chat",
  "Anchor CRM",
  "Trailhead Export",
  "Vector Payroll",
  "Backline Support",
  "Ember Scheduling",
  "Quiet Hours Bot",
  "Datastream Relay",
  "Fablewrights Connector",
  "Harbor Timesheets",
  "Ridgeline Analytics",
  "Southpaw Support Desk",
  "Cinderpoint CRM Sync",
  "Driftwood Notify",
  "Palisade Access Broker",
  "Longform Contracts Importer And Archive Bridge",
  "Kestrel Ops",
  "Wavelet Metrics",
  "Foundry Ticketing",
  "Brightline HR Sync",
  "Cypress Payments",
  "Argon Support Bot",
  "Milestone Tracker",
  "Overlook Reporting",
  "Tallgrass Forms",
  "Nightshade Backup Agent",
  "Alkali Inventory",
  "Redwood Onboarding",
  "Tidepool Feedback",
  "Granite Compliance Sync",
  "Sable Directory Bridge",
];

function makeExpiry(daysToExpiry) {
  if (daysToExpiry === null) {
    return { state: "expired", days: null, label: "Expired" };
  }
  if (daysToExpiry < 0) {
    return {
      state: "expired",
      days: daysToExpiry,
      label: `Expired ${Math.abs(daysToExpiry)} day${Math.abs(daysToExpiry) === 1 ? "" : "s"} ago`,
    };
  }
  if (daysToExpiry <= 14) {
    return { state: "expiring", days: daysToExpiry, label: `Expires in ${daysToExpiry} day${daysToExpiry === 1 ? "" : "s"}` };
  }
  return { state: "valid", days: daysToExpiry, label: `Expires in ${daysToExpiry} days` };
}

function seedRecords() {
  return [
    {
      id: "surveyship",
      application: "SurveyShip",
      owner: "Lila Chen",
      environment: "production",
      scopes: ["contacts.read", "deals.write"],
      lastUsedDays: 2,
      expiry: makeExpiry(78),
      status: "active",
      isSeed: true,
    },
    {
      id: "old-crm-export",
      application: "Old CRM Export",
      owner: "Arun Shah",
      environment: "production",
      scopes: ["contacts.read"],
      lastUsedDays: 418,
      expiry: makeExpiry(12),
      status: "active",
      isSeed: true,
    },
    {
      id: "payroll-archive",
      application: "Payroll Archive",
      owner: "Dana Cole",
      environment: "production",
      scopes: ["payroll.read"],
      lastUsedDays: 391,
      expiry: makeExpiry(31),
      status: "active",
      isSeed: true,
    },
    {
      id: "sandbox-sync",
      application: "Sandbox Sync",
      owner: "Lila Chen",
      environment: "sandbox",
      scopes: ["contacts.read"],
      lastUsedDays: 201,
      expiry: makeExpiry(4),
      status: "active",
      isSeed: true,
    },
    {
      id: "legacy-forms",
      application: "Legacy Forms",
      owner: null,
      environment: "production",
      scopes: ["contacts.read", "forms.write"],
      lastUsedDays: 502,
      expiry: makeExpiry(null),
      status: "active",
      isSeed: true,
    },
  ];
}

function syntheticRecords() {
  const usedIds = new Set(seedRecords().map((r) => r.id));
  const out = [];
  SYNTHETIC_APP_NAMES.forEach((name, i) => {
    let id = slugify(name);
    while (usedIds.has(id)) id = `${id}-${i}`;
    usedIds.add(id);

    const owner = pickWeighted([
      [pick(OWNER_POOL), 85],
      [null, 15],
    ]);
    const environment = pickWeighted([
      ["production", 78],
      ["sandbox", 22],
    ]);
    const status = pickWeighted([
      ["active", 87],
      ["disabled", 13],
    ]);
    const isStale = pickWeighted([
      [true, 40],
      [false, 60],
    ]);
    const lastUsedDays = isStale ? 180 + Math.floor(rng() * 470) : Math.floor(rng() * 175);
    const expiryBucket = pickWeighted([
      ["expired", 15],
      ["expiring", 15],
      ["valid", 70],
    ]);
    const daysToExpiry =
      expiryBucket === "expired"
        ? -(1 + Math.floor(rng() * 60))
        : expiryBucket === "expiring"
        ? Math.floor(rng() * 15)
        : 15 + Math.floor(rng() * 380);
    const scopeCount = 1 + Math.floor(rng() * 3);
    const scopes = sampleUnique(SCOPES_POOL, scopeCount);

    out.push({
      id,
      application: name,
      owner,
      environment,
      scopes,
      lastUsedDays,
      expiry: makeExpiry(daysToExpiry),
      status,
      isSeed: false,
    });
  });

  // STATE-04 extremes: one very-long-string record, one many-scopes record.
  const longNameRec = out.find((r) => r.application === "Longform Contracts Importer And Archive Bridge");
  if (longNameRec) {
    longNameRec.owner = "Devon Brooks";
  }
  const manyScopesRec = out.find((r) => r.application === "Foundry Ticketing");
  if (manyScopesRec) {
    manyScopesRec.scopes = sampleUnique(SCOPES_POOL, 6);
  }

  return out;
}

function classifyBand(c) {
  if (c.status === "disabled") return "disabled";
  if (c.lastUsedDays >= 180) return "needs-review";
  if (c.expiry.state === "expired") return "expired";
  if (c.expiry.state === "expiring") return "expiring";
  return "healthy";
}

const BAND_ORDER = ["needs-review", "expired", "expiring", "healthy", "disabled"];
const BAND_META = {
  "needs-review": { heading: "Needs review — active, unused 180+ days", tile: "Needs review" },
  expired: { heading: "Token expired, still active", tile: "Expired" },
  expiring: { heading: "Expiring within 14 days", tile: "Expiring" },
  healthy: { heading: "Active, recently used", tile: "Active" },
  disabled: { heading: "Disabled", tile: "Disabled" },
};

function formatLastUsed(days) {
  return `${days} day${days === 1 ? "" : "s"} ago`;
}

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

// ------------------------------------------------------------------
// State
// ------------------------------------------------------------------

const state = {
  records: [...seedRecords(), ...syntheticRecords()],
  filters: { search: "", environment: "all", status: "all", band: null },
  page: 1,
  pageSize: 8,
  selection: new Set(),
  openMenuId: null,
  drawerRecordId: null,
  drawerReturnFocus: null,
  confirmDialog: null, // { kind: 'bulk-disable' | 'disable' | 'delete', ids: [...] }
  deleteConfirmInput: "",
  bulkError: null, // { failedIds: [...], batchSize: n }
  toast: null,
  liveMessage: "",
};

function getRecord(id) {
  return state.records.find((r) => r.id === id);
}

function matchesFilters(c) {
  const f = state.filters;
  if (f.search) {
    const q = f.search.trim().toLowerCase();
    const hay = `${c.application} ${c.owner || ""}`.toLowerCase();
    if (!hay.includes(q)) return false;
  }
  if (f.environment !== "all" && c.environment !== f.environment) return false;
  if (f.status !== "all" && c.status !== f.status) return false;
  if (f.band && classifyBand(c) !== f.band) return false;
  return true;
}

function getFiltered() {
  return state.records.filter(matchesFilters);
}

function getBandCounts() {
  const counts = { "needs-review": 0, expired: 0, expiring: 0, healthy: 0, disabled: 0 };
  state.records.forEach((c) => {
    counts[classifyBand(c)]++;
  });
  return counts;
}

function getGroupedPage() {
  const filtered = getFiltered();
  const grouped = [];
  BAND_ORDER.forEach((band) => {
    const rows = filtered
      .filter((c) => classifyBand(c) === band)
      .sort((a, b) => {
        if (band === "needs-review" || band === "expired") return b.lastUsedDays - a.lastUsedDays;
        if (band === "expiring") return (a.expiry.days ?? 999) - (b.expiry.days ?? 999);
        return a.application.localeCompare(b.application);
      });
    if (rows.length) grouped.push({ band, rows });
  });
  return grouped;
}

function getFlatFilteredOrdered() {
  return getGroupedPage().flatMap((g) => g.rows);
}

function getPageSlice() {
  const flat = getFlatFilteredOrdered();
  const totalPages = Math.max(1, Math.ceil(flat.length / state.pageSize));
  const page = Math.min(state.page, totalPages);
  const start = (page - 1) * state.pageSize;
  const pageIds = new Set(flat.slice(start, start + state.pageSize).map((r) => r.id));
  return { flat, totalPages, page, pageIds };
}

// ------------------------------------------------------------------
// Rendering
// ------------------------------------------------------------------

const root = document.getElementById("app");
const narrowQuery = window.matchMedia("(max-width: 640px)");
function isNarrowViewport() {
  return narrowQuery.matches;
}
narrowQuery.addEventListener("change", () => render());

// A11Y-08: the live region is a stable node that persists across renders —
// only its textContent changes. Recreating the element on every render()
// (as part of root.innerHTML) is unreliable for announcements in some
// assistive technology, since the node's identity, not just its text,
// is what AT watches.
const liveRegion = document.createElement("div");
liveRegion.setAttribute("aria-live", "polite");
liveRegion.className = "visually-hidden";
document.body.appendChild(liveRegion);

function render() {
  const filtered = getFiltered();
  const grouped = getGroupedPage();
  const { flat, totalPages, page, pageIds } = getPageSlice();
  const bandCounts = getBandCounts();
  const total = state.records.length;
  const overlayOpen = Boolean(state.drawerRecordId || state.confirmDialog);

  root.innerHTML = `
    <a class="skip-link" href="#connections-table">Skip to connections table</a>
    <div class="shell" ${overlayOpen ? 'inert aria-hidden="true"' : ""}>
      ${renderCommandStrip(total, filtered.length)}
      ${renderTriageStrip(bandCounts)}
      ${renderTablePane(grouped, flat, totalPages, page, pageIds, filtered.length, total)}
    </div>
    ${renderDrawer()}
    ${renderDialogs()}
    ${renderToast()}
  `;
  liveRegion.textContent = state.liveMessage;

  wireEvents();
  const overlay = root.querySelector('[role="dialog"], [role="alertdialog"]');
  if (overlay) {
    const focusable = overlay.querySelector(".drawer__close, [autofocus]") || overlay.querySelector("button, input, [href], select, textarea, [tabindex]:not([tabindex='-1'])");
    if (focusable) focusable.focus();
    wireFocusTrap(overlay);
  }
}

// A11Y-07: while a drawer or dialog is open, Tab must cycle only within
// it (the rest of the page is also marked inert above, which browsers
// that support it already enforce; this keydown handler is the fallback
// for engines that ignore `inert`).
function wireFocusTrap(container) {
  const handler = (e) => {
    if (e.key !== "Tab") return;
    const focusables = [...container.querySelectorAll("button, input, [href], select, textarea, [tabindex]:not([tabindex='-1'])")].filter(
      (el) => !el.disabled && el.offsetParent !== null
    );
    if (!focusables.length) return;
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };
  document.addEventListener("keydown", handler, { once: false });
  // Replaced on next render() (new container, new listener); the old
  // listener is harmless once its container is gone since it targets a
  // detached node, but remove it explicitly to avoid growth.
  const cleanup = () => document.removeEventListener("keydown", handler);
  const observer = new MutationObserver(() => {
    if (!document.body.contains(container)) {
      cleanup();
      observer.disconnect();
    }
  });
  observer.observe(root, { childList: true, subtree: true });
}

function renderCommandStrip(total, filteredCount) {
  const f = state.filters;
  const presetMatches = (env, status) => f.environment === env && f.status === status;
  return `
    <header class="command-strip">
      <div class="command-strip__identity">
        <h1>Application connections</h1>
        <p class="command-strip__scope">${total} total · showing ${filteredCount} matching the current view</p>
      </div>
      <div class="command-strip__controls">
        <div class="field">
          <label class="field__label" for="search-input">Search</label>
          <input id="search-input" class="input search-input" type="search" placeholder="Application or owner"
                 value="${esc(f.search)}" data-action="search" />
        </div>
        <div class="field">
          <label class="field__label" for="env-select">Environment</label>
          <select id="env-select" class="select" data-action="filter-environment">
            <option value="all" ${f.environment === "all" ? "selected" : ""}>All</option>
            <option value="production" ${f.environment === "production" ? "selected" : ""}>Production</option>
            <option value="sandbox" ${f.environment === "sandbox" ? "selected" : ""}>Sandbox</option>
          </select>
        </div>
        <div class="field">
          <label class="field__label" for="status-select">Status</label>
          <select id="status-select" class="select" data-action="filter-status">
            <option value="all" ${f.status === "all" ? "selected" : ""}>All</option>
            <option value="active" ${f.status === "active" ? "selected" : ""}>Active</option>
            <option value="disabled" ${f.status === "disabled" ? "selected" : ""}>Disabled</option>
          </select>
        </div>
        <div class="field">
          <span class="field__label" id="perspectives-label">Perspectives</span>
          <div class="perspectives" role="group" aria-labelledby="perspectives-label">
            <button type="button" class="chip" aria-pressed="${presetMatches("all", "all")}" data-action="perspective" data-env="all" data-status="all">All</button>
            <button type="button" class="chip" aria-pressed="${presetMatches("production", "all")}" data-action="perspective" data-env="production" data-status="all">Production</button>
            <button type="button" class="chip" aria-pressed="${presetMatches("sandbox", "all")}" data-action="perspective" data-env="sandbox" data-status="all">Sandbox</button>
            <button type="button" class="chip" aria-pressed="${presetMatches("all", "disabled")}" data-action="perspective" data-env="all" data-status="disabled">Disabled only</button>
          </div>
        </div>
      </div>
    </header>
  `;
}

function renderTriageStrip(counts) {
  const tiles = BAND_ORDER.map((band) => {
    const meta = BAND_META[band];
    const pressed = state.filters.band === band;
    return `
      <button type="button" class="triage-tile triage-tile--${band}" aria-pressed="${pressed}" data-action="band-filter" data-band="${band}">
        <span class="triage-tile__count">${counts[band]}</span>
        <span class="triage-tile__label">${esc(meta.tile)}</span>
      </button>
    `;
  }).join("");
  return `<div class="triage-strip" role="group" aria-label="Filter by urgency band">${tiles}</div>`;
}

function renderTablePane(grouped, flat, totalPages, page, pageIds, filteredCount, total) {
  const selectedCount = state.selection.size;
  const selectedOutsideFilter = [...state.selection].filter((id) => !flat.some((r) => r.id === id)).length;

  const bulkBar =
    selectedCount > 0
      ? `
    <div class="bulk-bar" role="region" aria-label="Bulk actions">
      <div>
        <span class="bulk-bar__summary"><strong>${selectedCount}</strong> selected${selectedOutsideFilter ? ` (${selectedOutsideFilter} outside the current filter)` : ""}</span>
      </div>
      <div class="bulk-bar__actions">
        <button type="button" class="btn btn--text" data-action="clear-selection">Clear selection</button>
        <button type="button" class="btn btn--tonal" data-action="open-bulk-disable">Disable selected (${selectedCount})</button>
      </div>
      ${state.bulkError ? renderBulkError() : ""}
    </div>
  `
      : "";

  // Render exactly one of {table, cards} — never both — so every control
  // (checkbox, menu, action) exists once in the DOM. Rendering both and
  // hiding one with CSS would duplicate every accessible name and control,
  // which is a real defect (ambiguous for assistive tech and any DOM-based
  // tooling), not only a cosmetic one.
  const narrow = isNarrowViewport();
  const body =
    filteredCount === 0
      ? renderEmptyState()
      : `
    ${
      narrow
        ? `<div class="connection-cards">${grouped.map((g) => renderBandCards(g, pageIds)).join("")}</div>`
        : `<div class="table-scroll">
      <table class="connections" id="connections-table">
        <thead>
          <tr>
            <th><span class="visually-hidden">Select</span></th>
            <th>Application</th>
            <th>Owner</th>
            <th>Environment</th>
            <th class="col-numeric">Last used</th>
            <th>Token expiry</th>
            <th>Status</th>
            <th><span class="visually-hidden">Actions</span></th>
          </tr>
        </thead>
        <tbody>
          ${grouped.map((g) => renderBandRows(g, pageIds)).join("")}
        </tbody>
      </table>
    </div>`
    }
    ${renderPagination(totalPages, page)}
  `;

  return `
    <section class="pane" aria-labelledby="table-pane-title">
      <div class="pane__bar">
        <h2 class="pane__title" id="table-pane-title">Connections</h2>
        <span class="table-toolbar__result-count">${filteredCount} of ${total}</span>
      </div>
      ${bulkBar}
      ${body}
    </section>
  `;
}

function renderBulkError() {
  const names = state.bulkError.failedIds.map((id) => getRecord(id)?.application || id).join(", ");
  return `
    <div class="bulk-bar__error" role="alert">
      <span>${state.bulkError.failedIds.length} of ${state.bulkError.batchSize} failed: ${esc(names)} — couldn't confirm its current state before disabling.</span>
      <span class="row-actions">
        <button type="button" class="btn btn--text" data-action="retry-bulk-error">Retry</button>
        <button type="button" class="btn btn--text" data-action="dismiss-bulk-error">Dismiss</button>
      </span>
    </div>
  `;
}

function renderEmptyState() {
  const f = state.filters;
  const parts = [];
  if (f.search) parts.push(`search "${f.search}"`);
  if (f.environment !== "all") parts.push(`Environment: ${f.environment}`);
  if (f.status !== "all") parts.push(`Status: ${f.status}`);
  if (f.band) parts.push(`Band: ${BAND_META[f.band].tile}`);
  const summary = parts.length ? parts.join(" + ") : "the current filters";
  return `
    <div class="empty-state">
      <p class="empty-state__title">No connections match ${esc(summary)}</p>
      <p class="empty-state__body">Clear the active filters to see all ${state.records.length} connections.</p>
      <button type="button" class="btn btn--outline" data-action="clear-filters">Clear filters</button>
    </div>
  `;
}

function rowBadges(c) {
  const badges = [];
  if (c.status === "active" && c.lastUsedDays >= 180) {
    badges.push(`<span class="badge badge--review">Needs review</span>`);
  }
  if (c.expiry.state === "expired") {
    badges.push(`<span class="badge badge--expired">${c.expiry.days === null ? "Expired" : "Expired"}</span>`);
  } else if (c.expiry.state === "expiring") {
    badges.push(`<span class="badge badge--expiring">Expires ≤14d</span>`);
  }
  return badges.join(" ");
}

function rowActionsMenu(c) {
  const open = state.openMenuId === c.id;
  const notifyDisabled = !c.owner;
  return `
    <div class="row-actions">
      <button type="button" class="icon-btn" data-action="view-details" data-id="${c.id}" title="View details" aria-label="View details for ${esc(c.application)}">View</button>
      <div class="menu-wrap">
        <button type="button" class="icon-btn" data-action="toggle-menu" data-id="${c.id}" aria-haspopup="true" aria-expanded="${open}" aria-label="More actions for ${esc(c.application)}">⋯</button>
        <div class="menu" role="menu" aria-label="Actions for ${esc(c.application)}" ${open ? "" : "hidden"}>
          <button type="button" class="menu__item" role="menuitem" data-action="notify-owner" data-id="${c.id}" ${notifyDisabled ? "disabled" : ""}>
            <span>Notify owner</span>
            ${notifyDisabled ? '<span class="menu__reason">No owner on record</span>' : ""}
          </button>
          ${
            c.status === "active"
              ? `<button type="button" class="menu__item" role="menuitem" data-action="open-disable" data-id="${c.id}">Disable connection</button>`
              : ""
          }
          <div class="menu__divider" role="separator"></div>
          <button type="button" class="menu__item menu__item--danger" role="menuitem" data-action="open-delete" data-id="${c.id}">Delete connection</button>
        </div>
      </div>
    </div>
  `;
}

function statusText(c) {
  return c.status === "disabled" ? `<span class="badge badge--disabled">Disabled</span>` : "Active";
}

function renderBandRows(group, pageIds) {
  const rowsOnPage = group.rows.filter((r) => pageIds.has(r.id));
  if (!rowsOnPage.length) return "";
  const meta = BAND_META[group.band];
  return `
    <tr class="band-row band-row--${group.band}">
      <th colspan="8" scope="colgroup">${esc(meta.heading)} (${group.rows.length})</th>
    </tr>
    ${rowsOnPage.map((c) => renderRow(c)).join("")}
  `;
}

function renderRow(c) {
  const selected = state.selection.has(c.id);
  return `
    <tr data-id="${c.id}" data-selected="${selected}">
      <td>
        <input type="checkbox" data-action="select-row" data-id="${c.id}" ${selected ? "checked" : ""}
               aria-label="Select ${esc(c.application)}" />
      </td>
      <td>
        <div class="app-name">${esc(c.application)}</div>
        <div class="env-tag">${c.scopes.length} scope${c.scopes.length === 1 ? "" : "s"} · ${rowBadges(c)}</div>
      </td>
      <td>${c.owner ? esc(c.owner) : '<span class="app-owner--none">No owner</span>'}</td>
      <td><span class="env-tag">${esc(c.environment)}</span></td>
      <td class="col-numeric">${formatLastUsed(c.lastUsedDays)}</td>
      <td>${esc(c.expiry.label)}</td>
      <td>${statusText(c)}</td>
      <td>${rowActionsMenu(c)}</td>
    </tr>
  `;
}

function renderBandCards(group, pageIds) {
  const rowsOnPage = group.rows.filter((r) => pageIds.has(r.id));
  if (!rowsOnPage.length) return "";
  const meta = BAND_META[group.band];
  return `
    <div class="band-heading">${esc(meta.heading)} (${group.rows.length})</div>
    ${rowsOnPage.map((c) => renderCard(c)).join("")}
  `;
}

function renderCard(c) {
  const selected = state.selection.has(c.id);
  return `
    <div class="connection-card" data-id="${c.id}" data-selected="${selected}">
      <div class="connection-card__top">
        <div class="connection-card__name">
          <input type="checkbox" data-action="select-row" data-id="${c.id}" ${selected ? "checked" : ""}
                 aria-label="Select ${esc(c.application)}" />
          <span class="app-name">${esc(c.application)}</span>
        </div>
        ${statusText(c)}
      </div>
      <div class="connection-card__badges">${rowBadges(c)}</div>
      <dl class="connection-card__meta">
        <div><dt>Owner</dt> <dd>${c.owner ? esc(c.owner) : "No owner"}</dd></div>
        <div><dt>Env</dt> <dd>${esc(c.environment)}</dd></div>
        <div><dt>Last used</dt> <dd>${formatLastUsed(c.lastUsedDays)}</dd></div>
        <div><dt>Expiry</dt> <dd>${esc(c.expiry.label)}</dd></div>
      </dl>
      <div class="connection-card__actions">
        <button type="button" class="btn btn--outline" data-action="view-details" data-id="${c.id}">View details</button>
        <div class="menu-wrap">
          <button type="button" class="icon-btn" data-action="toggle-menu" data-id="${c.id}" aria-haspopup="true" aria-expanded="${state.openMenuId === c.id}" aria-label="More actions for ${esc(c.application)}">⋯</button>
          <div class="menu" role="menu" aria-label="Actions for ${esc(c.application)}" ${state.openMenuId === c.id ? "" : "hidden"}>
            <button type="button" class="menu__item" role="menuitem" data-action="notify-owner" data-id="${c.id}" ${!c.owner ? "disabled" : ""}>
              <span>Notify owner</span>
              ${!c.owner ? '<span class="menu__reason">No owner on record</span>' : ""}
            </button>
            ${c.status === "active" ? `<button type="button" class="menu__item" role="menuitem" data-action="open-disable" data-id="${c.id}">Disable connection</button>` : ""}
            <div class="menu__divider" role="separator"></div>
            <button type="button" class="menu__item menu__item--danger" role="menuitem" data-action="open-delete" data-id="${c.id}">Delete connection</button>
          </div>
        </div>
      </div>
    </div>
  `;
}

function renderPagination(totalPages, page) {
  if (totalPages <= 1) return "";
  const pages = Array.from({ length: totalPages }, (_, i) => i + 1)
    .map(
      (p) =>
        `<button type="button" class="btn ${p === page ? "btn--tonal" : "btn--text"}" data-action="go-page" data-page="${p}" aria-current="${p === page ? "page" : "false"}">${p}</button>`
    )
    .join("");
  return `
    <nav class="pagination" aria-label="Connections pagination">
      <span>Page ${page} of ${totalPages} · selection persists across pages</span>
      <div class="pagination__pages">${pages}</div>
    </nav>
  `;
}

function renderDrawer() {
  if (!state.drawerRecordId) return `<div class="drawer-backdrop" hidden></div><div class="drawer" hidden></div>`;
  const c = getRecord(state.drawerRecordId);
  if (!c) return `<div class="drawer-backdrop" hidden></div><div class="drawer" hidden></div>`;
  return `
    <div class="drawer-backdrop" data-action="dismiss-drawer-backdrop"></div>
    <div class="drawer" role="dialog" aria-modal="true" aria-labelledby="drawer-app-name">
      <div class="drawer__header">
        <div>
          <p class="drawer__title">Record detail</p>
          <h2 class="drawer__app" id="drawer-app-name">${esc(c.application)}</h2>
        </div>
        <button type="button" class="icon-btn drawer__close" data-action="close-drawer" aria-label="Close details">✕</button>
      </div>
      <div class="drawer__body">
        <div class="drawer__field">
          <p class="drawer__field-label">Status</p>
          <p class="drawer__field-value">${statusText(c)} ${rowBadges(c)}</p>
        </div>
        <div class="drawer__field">
          <p class="drawer__field-label">Owner</p>
          <p class="drawer__field-value">${c.owner ? esc(c.owner) : '<span class="app-owner--none">No owner on record</span>'}</p>
        </div>
        <div class="drawer__field">
          <p class="drawer__field-label">Environment</p>
          <p class="drawer__field-value">${esc(c.environment)}</p>
        </div>
        <div class="drawer__field">
          <p class="drawer__field-label">Last successful use</p>
          <p class="drawer__field-value">${formatLastUsed(c.lastUsedDays)}</p>
        </div>
        <div class="drawer__field">
          <p class="drawer__field-label">Token expiry</p>
          <p class="drawer__field-value">${esc(c.expiry.label)}</p>
        </div>
        <div class="drawer__field">
          <p class="drawer__field-label">Scopes</p>
          <div class="drawer__scopes">${c.scopes.map((s) => `<span class="scope-tag">${esc(s)}</span>`).join("")}</div>
        </div>
        <div class="drawer__field">
          <p class="drawer__field-label">Production-critical</p>
          <div class="drawer__critical-note">Not marked on this record. Confirm with the owner or the environment record before disabling or deleting.</div>
        </div>
      </div>
      <div class="drawer__footer">
        <button type="button" class="btn btn--text" data-action="close-drawer">Close</button>
        ${
          c.status === "active"
            ? `<button type="button" class="btn btn--outline" data-action="open-disable" data-id="${c.id}">Disable connection</button>`
            : ""
        }
      </div>
    </div>
  `;
}

function renderDialogs() {
  if (!state.confirmDialog) return `<div class="dialog-backdrop" hidden></div>`;
  const d = state.confirmDialog;

  if (d.kind === "bulk-disable") {
    // ACT-19 requires the exact objects, by name — for a "significant,
    // reversible" bulk action the admin must be able to verify every
    // record in the batch, not a truncated sample. .dialog__list is
    // already a scrollable, height-capped container for exactly this.
    const recs = d.ids.map(getRecord).filter(Boolean);
    return `
      <div class="dialog-backdrop" data-action="dismiss-dialog-backdrop">
        <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title" onclick="event.stopPropagation()">
          <div class="dialog__header"><h2 class="dialog__title" id="dialog-title">Disable ${recs.length} connections</h2></div>
          <div class="dialog__body">
            <p>This disables access for the following connection${recs.length === 1 ? "" : "s"}. Disabling is reversible by an administrator.</p>
            <ul class="dialog__list">
              ${recs.map((r) => `<li><strong>${esc(r.application)}</strong> — ${esc(r.owner || "no owner")}, ${esc(r.environment)}</li>`).join("")}
            </ul>
          </div>
          <div class="dialog__footer">
            <button type="button" class="btn btn--text" data-action="close-dialog" autofocus>Cancel</button>
            <button type="button" class="btn btn--tonal" data-action="confirm-bulk-disable">Disable ${recs.length} connections</button>
          </div>
        </div>
      </div>
    `;
  }

  if (d.kind === "disable") {
    const c = getRecord(d.ids[0]);
    if (!c) return `<div class="dialog-backdrop" hidden></div>`;
    return `
      <div class="dialog-backdrop" data-action="dismiss-dialog-backdrop">
        <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title" onclick="event.stopPropagation()">
          <div class="dialog__header"><h2 class="dialog__title" id="dialog-title">Disable connection</h2></div>
          <div class="dialog__body">
            <p>Disable <strong>${esc(c.application)}</strong> (${esc(c.owner || "no owner")}, ${esc(c.environment)})? This is reversible by an administrator.</p>
          </div>
          <div class="dialog__footer">
            <button type="button" class="btn btn--text" data-action="close-dialog" autofocus>Cancel</button>
            <button type="button" class="btn btn--tonal" data-action="confirm-disable" data-id="${c.id}">Disable connection</button>
          </div>
        </div>
      </div>
    `;
  }

  if (d.kind === "delete") {
    const c = getRecord(d.ids[0]);
    if (!c) return `<div class="dialog-backdrop" hidden></div>`;
    const confirmed = state.deleteConfirmInput.trim() === "1";
    return `
      <div class="dialog-backdrop" data-action="dismiss-dialog-backdrop">
        <div class="dialog dialog--danger" role="alertdialog" aria-modal="true" aria-labelledby="dialog-title" onclick="event.stopPropagation()">
          <div class="dialog__header"><h2 class="dialog__title" id="dialog-title">Delete connection</h2></div>
          <div class="dialog__body">
            <p>Permanently delete <strong>${esc(c.application)}</strong> (${esc(c.owner || "no owner")}, ${esc(c.environment)})? This removes the connection and cannot be undone.</p>
            <label class="field__label" for="delete-confirm-input">Type 1 to confirm deleting this connection</label>
            <input id="delete-confirm-input" class="input dialog__confirm-input" type="text" inputmode="numeric"
                   value="${esc(state.deleteConfirmInput)}" data-action="delete-confirm-input" autofocus />
          </div>
          <div class="dialog__footer">
            <button type="button" class="btn btn--text" data-action="close-dialog">Cancel</button>
            <button type="button" class="btn btn--danger" data-action="confirm-delete" data-id="${c.id}" ${confirmed ? "" : "disabled"}>Delete connection</button>
          </div>
        </div>
      </div>
    `;
  }

  return `<div class="dialog-backdrop" hidden></div>`;
}

function renderToast() {
  if (!state.toast) return `<div class="toast" hidden></div>`;
  return `
    <div class="toast" role="status">
      <span>${esc(state.toast)}</span>
      <button type="button" class="btn btn--text" data-action="dismiss-toast">Dismiss</button>
    </div>
  `;
}

// ------------------------------------------------------------------
// Events
// ------------------------------------------------------------------

let toastTimer = null;
function showToast(message) {
  state.toast = message;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    state.toast = null;
    render();
  }, 5000);
}

function announce(message) {
  state.liveMessage = message;
}

function wireEvents() {
  root.querySelectorAll("[data-action]").forEach((el) => {
    const action = el.dataset.action;

    if (el.tagName === "SELECT" && (action === "filter-environment" || action === "filter-status")) {
      el.addEventListener("change", (e) => {
        if (action === "filter-environment") state.filters.environment = e.target.value;
        if (action === "filter-status") state.filters.status = e.target.value;
        state.page = 1;
        render();
      });
      return;
    }

    if (action === "search") {
      el.addEventListener("input", (e) => {
        state.filters.search = e.target.value;
        state.page = 1;
        render();
      });
      return;
    }

    if (action === "delete-confirm-input") {
      el.addEventListener("input", (e) => {
        state.deleteConfirmInput = e.target.value;
        render();
      });
      return;
    }

    if (action === "select-row") {
      el.addEventListener("change", (e) => {
        const id = el.dataset.id;
        if (e.target.checked) state.selection.add(id);
        else state.selection.delete(id);
        render();
      });
      return;
    }

    // click actions
    el.addEventListener("click", (e) => {
      const id = el.dataset.id;
      switch (action) {
        case "perspective":
          state.filters.environment = el.dataset.env;
          state.filters.status = el.dataset.status;
          state.page = 1;
          render();
          break;
        case "band-filter":
          state.filters.band = state.filters.band === el.dataset.band ? null : el.dataset.band;
          state.page = 1;
          render();
          break;
        case "clear-filters":
          state.filters = { search: "", environment: "all", status: "all", band: null };
          state.page = 1;
          render();
          break;
        case "clear-selection":
          state.selection.clear();
          state.bulkError = null;
          render();
          break;
        case "go-page":
          state.page = Number(el.dataset.page);
          render();
          break;
        case "toggle-menu":
          state.openMenuId = state.openMenuId === id ? null : id;
          render();
          break;
        case "view-details":
          state.drawerRecordId = id;
          state.openMenuId = null;
          render();
          break;
        case "close-drawer":
        case "dismiss-drawer-backdrop":
          state.drawerRecordId = null;
          render();
          break;
        case "notify-owner": {
          const rec = getRecord(id);
          state.openMenuId = null;
          if (rec && rec.owner) {
            showToast(`Notified ${rec.owner} about ${rec.application}.`);
            announce(`Notified ${rec.owner} about ${rec.application}.`);
          }
          render();
          break;
        }
        case "open-disable":
          state.confirmDialog = { kind: "disable", ids: [id] };
          state.openMenuId = null;
          render();
          break;
        case "open-delete":
          state.confirmDialog = { kind: "delete", ids: [id] };
          state.deleteConfirmInput = "";
          state.openMenuId = null;
          render();
          break;
        case "open-bulk-disable":
          state.confirmDialog = { kind: "bulk-disable", ids: [...state.selection] };
          render();
          break;
        case "close-dialog":
        case "dismiss-dialog-backdrop":
          state.confirmDialog = null;
          state.deleteConfirmInput = "";
          render();
          break;
        case "confirm-disable": {
          const rec = getRecord(id);
          if (rec) rec.status = "disabled";
          state.confirmDialog = null;
          state.selection.delete(id);
          showToast(`Disabled ${rec.application}.`);
          announce(`Disabled ${rec.application}.`);
          render();
          break;
        }
        case "confirm-delete": {
          const rec = getRecord(id);
          state.records = state.records.filter((r) => r.id !== id);
          state.selection.delete(id);
          state.confirmDialog = null;
          state.deleteConfirmInput = "";
          if (rec) {
            showToast(`Deleted ${rec.application}.`);
            announce(`Deleted ${rec.application}.`);
          }
          render();
          break;
        }
        case "confirm-bulk-disable": {
          const ids = state.confirmDialog.ids;
          const failed = [];
          let succeeded = 0;
          ids.forEach((rid) => {
            const rec = getRecord(rid);
            if (!rec || rec.status !== "active") return;
            if (rid === "legacy-forms") {
              failed.push(rid);
            } else {
              rec.status = "disabled";
              succeeded++;
            }
          });
          state.confirmDialog = null;
          if (failed.length) {
            state.selection = new Set(failed);
            state.bulkError = { failedIds: failed, batchSize: ids.length };
            announce(`Disabled ${succeeded} of ${ids.length} connections. ${failed.length} failed.`);
          } else {
            state.selection.clear();
            state.bulkError = null;
            showToast(`Disabled ${succeeded} connection${succeeded === 1 ? "" : "s"}.`);
            announce(`Disabled ${succeeded} connection${succeeded === 1 ? "" : "s"}.`);
          }
          render();
          break;
        }
        case "retry-bulk-error": {
          if (!state.bulkError) break;
          const ids = state.bulkError.failedIds;
          const stillFailed = ids.filter((rid) => rid === "legacy-forms");
          const succeeded = ids.filter((rid) => rid !== "legacy-forms");
          succeeded.forEach((rid) => {
            const rec = getRecord(rid);
            if (rec) rec.status = "disabled";
          });
          if (stillFailed.length) {
            state.bulkError = { failedIds: stillFailed, batchSize: ids.length };
            state.selection = new Set(stillFailed);
            announce(`Retry failed for ${stillFailed.length} connection.`);
          } else {
            state.bulkError = null;
            state.selection.clear();
          }
          render();
          break;
        }
        case "dismiss-bulk-error":
          state.bulkError = null;
          state.selection.clear();
          render();
          break;
        case "dismiss-toast":
          if (toastTimer) clearTimeout(toastTimer);
          state.toast = null;
          render();
          break;
      }
    });
  });

  // Close open menu / dialogs on Escape, click-outside
  document.addEventListener(
    "keydown",
    (e) => {
      if (e.key === "Escape") {
        if (state.confirmDialog) {
          state.confirmDialog = null;
          render();
        } else if (state.drawerRecordId) {
          state.drawerRecordId = null;
          render();
        } else if (state.openMenuId) {
          state.openMenuId = null;
          render();
        }
      }
    },
    { once: true }
  );

  document.addEventListener(
    "click",
    (e) => {
      if (state.openMenuId && !e.target.closest(".menu-wrap")) {
        state.openMenuId = null;
        render();
      }
    },
    { once: true, capture: true }
  );
}

// ------------------------------------------------------------------
// Route + init
// ------------------------------------------------------------------

function normalizeRoute() {
  if (window.location.pathname !== "/admin/connections") {
    window.history.replaceState({}, "", "/admin/connections" + window.location.search);
  }
}

normalizeRoute();
render();
