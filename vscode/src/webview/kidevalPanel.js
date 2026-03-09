// -----------------------------------------------------------------------
// KidEval panel webview script
//
// Communicates with kidevalPanel.ts via PostMessage:
//   → discoverDatabases, runAnalysis
//   ← databases, results, error, fileInfo
// -----------------------------------------------------------------------

// @ts-check
/* global acquireVsCodeApi, MODE_CONFIG */

const vscode = acquireVsCodeApi();

const ACCENT_COLORS = [
    '#4ec9b0',  // teal
    '#ce9178',  // rust / salmon
    '#569cd6',  // cornflower blue
    '#dcdcaa',  // khaki / gold
    '#c586c0',  // lavender
    '#9cdcfe',  // sky blue
];

// -- DOM refs -------------------------------------------------------------
const dbSelect = document.getElementById('db-select');
const genderSelect = document.getElementById('gender-select');
const ageFrom = document.getElementById('age-from');
const ageTo = document.getElementById('age-to');
const btnRun = document.getElementById('btn-run');
const btnRunNoDb = document.getElementById('btn-run-no-db');
const statusEl = document.getElementById('status');
const resultsEl = document.getElementById('results');
const fileNameEl = document.getElementById('file-name');

let databases = [];

// -- Apply mode config to header ------------------------------------------
document.getElementById('cmd-eyebrow').textContent =
    'CLAN Analysis \u00b7 ' + MODE_CONFIG.analyzeCommand;
document.getElementById('cmd-display').textContent = MODE_CONFIG.title;
document.getElementById('cmd-description').textContent = MODE_CONFIG.description;

// -- Initialize -----------------------------------------------------------
vscode.postMessage({ command: 'discoverDatabases', libDir: MODE_CONFIG.defaultLibDir });
setStatus('Discovering databases\u2026', false);

// -- Event listeners ------------------------------------------------------

dbSelect.addEventListener('change', () => {
    btnRun.disabled = !dbSelect.value;
});

btnRun.addEventListener('click', () => {
    runAnalysis(true);
});

btnRunNoDb.addEventListener('click', () => {
    runAnalysis(false);
});

// -- PostMessage handler --------------------------------------------------

window.addEventListener('message', (event) => {
    const msg = event.data;
    switch (msg.command) {
        case 'databases':
            handleDatabases(msg.databases);
            break;
        case 'results':
            handleResults(msg.data);
            break;
        case 'error':
            setStatus(msg.message, true);
            break;
        case 'fileInfo':
            fileNameEl.textContent = msg.fileName;
            break;
    }
});

// -- Database discovery ---------------------------------------------------

function handleDatabases(dbs) {
    databases = dbs || [];

    dbSelect.innerHTML = '';

    if (databases.length === 0) {
        const opt = document.createElement('option');
        opt.value = '';
        opt.textContent = 'No databases found';
        dbSelect.appendChild(opt);
        btnRun.disabled = true;
        setStatus('No normative databases found. You can still run without a database.', false);
        return;
    }

    const placeholder = document.createElement('option');
    placeholder.value = '';
    placeholder.textContent = 'Select a database…';
    dbSelect.appendChild(placeholder);

    for (const db of databases) {
        const opt = document.createElement('option');
        opt.value = db.path;
        opt.textContent = formatDbName(db);
        dbSelect.appendChild(opt);
    }

    btnRun.disabled = true;
    setStatus('Ready. Select a database and click Run, or run without database comparison.', false);
}

function formatDbName(db) {
    const lang = db.language ? db.language.toUpperCase() : '???';
    const corpus = db.corpus_type || 'unknown';
    const count = db.entry_count != null ? ` (${db.entry_count} entries)` : '';
    return `${lang} — ${corpus}${count}`;
}

// -- Run analysis ---------------------------------------------------------

function runAnalysis(withDatabase) {
    setStatus('<span class="spinner"></span>Running KidEval analysis…', false);
    resultsEl.innerHTML = '';
    btnRun.disabled = true;
    btnRunNoDb.disabled = true;

    const msg = { command: 'runAnalysis' };

    if (withDatabase && dbSelect.value) {
        msg.databasePath = dbSelect.value;
        msg.databaseFilter = buildFilter();
    }

    vscode.postMessage(msg);
}

function buildFilter() {
    const filter = {};

    const gender = genderSelect.value;
    if (gender) { filter.gender = gender; }

    const from = parseInt(ageFrom.value, 10);
    if (!isNaN(from) && from > 0) { filter.age_from_months = from; }

    const to = parseInt(ageTo.value, 10);
    if (!isNaN(to) && to > 0) { filter.age_to_months = to; }

    return Object.keys(filter).length > 0 ? filter : undefined;
}

// -- Render results -------------------------------------------------------

function handleResults(data) {
    btnRun.disabled = !dbSelect.value;
    btnRunNoDb.disabled = false;
    resultsEl.innerHTML = '';

    if (!data || typeof data !== 'object') {
        setStatus('No results returned.', true);
        return;
    }

    setStatus('', false);

    let colorIdx = 0;
    for (const [key, val] of Object.entries(data)) {
        resultsEl.appendChild(renderSection(key, val, colorIdx++));
    }

    btnExportCsv.disabled = false;
}

// -- Section renderer (mirrors analysisPanel.js but with comparison awareness)

function renderSection(title, value, colorIndex) {
    const accentColor = ACCENT_COLORS[colorIndex % ACCENT_COLORS.length];
    const section = document.createElement('div');
    section.className = 'section';
    section.style.setProperty('--section-color', accentColor);

    // Section header
    const header = document.createElement('div');
    header.className = 'section-header';

    const titleEl = document.createElement('div');
    titleEl.className = 'section-title';
    titleEl.textContent = title;
    header.appendChild(titleEl);

    if (/^[A-Z]{2,6}$/.test(title.trim())) {
        const badge = document.createElement('span');
        badge.className = 'speaker-badge';
        badge.textContent = title.trim();
        header.appendChild(badge);
    }
    section.appendChild(header);

    if (typeof value !== 'object' || value === null) {
        // Scalar
        const grid = document.createElement('div');
        grid.className = 'stat-grid';
        grid.appendChild(makeStatCard(title, value, accentColor));
        section.appendChild(grid);
        return section;
    }

    if (Array.isArray(value) && value.length > 0 && typeof value[0] === 'object') {
        // Array of objects → check if it looks like comparison data
        if (isComparisonTable(value)) {
            section.appendChild(renderComparisonTable(value, accentColor));
        } else {
            section.appendChild(renderGenericTable(value, accentColor));
        }
        return section;
    }

    // Object with mixed scalar/array fields
    const primitives = Object.entries(value).filter(
        ([, v]) => !Array.isArray(v) && (typeof v !== 'object' || v === null)
    );
    const arrays = Object.entries(value).filter(([, v]) => Array.isArray(v));

    if (primitives.length > 0) {
        const grid = document.createElement('div');
        grid.className = 'stat-grid';
        for (const [k, v] of primitives) {
            grid.appendChild(makeStatCard(k, v, accentColor));
        }
        section.appendChild(grid);
    }

    for (const [k, v] of arrays) {
        if (v.length > 0 && typeof v[0] === 'object') {
            const label = document.createElement('div');
            label.className = 'sub-section-label';
            label.textContent = k;
            section.appendChild(label);
            if (isComparisonTable(v)) {
                section.appendChild(renderComparisonTable(v, accentColor));
            } else {
                section.appendChild(renderGenericTable(v, accentColor));
            }
        }
    }

    return section;
}

function isComparisonTable(rows) {
    if (rows.length === 0) return false;
    const first = rows[0];
    return 'z_score' in first || 'db_mean' in first || 'db_sd' in first;
}

function renderComparisonTable(rows, accentColor) {
    const table = document.createElement('table');

    const thead = document.createElement('thead');
    const hrow = document.createElement('tr');
    const cols = [
        { key: 'label', label: 'Measure', numeric: false },
        { key: 'score', label: 'Score', numeric: true },
        { key: 'db_mean', label: 'DB Mean', numeric: true },
        { key: 'db_sd', label: 'DB SD', numeric: true },
        { key: 'z_score', label: 'Z-Score', numeric: true },
        { key: 'db_n', label: 'N', numeric: true },
    ];

    for (const col of cols) {
        const th = document.createElement('th');
        th.textContent = col.label;
        if (col.numeric) th.className = 'num';
        hrow.appendChild(th);
    }
    thead.appendChild(hrow);
    table.appendChild(thead);

    const tbody = document.createElement('tbody');
    for (const row of rows) {
        const tr = document.createElement('tr');
        for (const col of cols) {
            const td = document.createElement('td');
            const val = row[col.key];

            if (col.key === 'z_score') {
                if (val === null || val === undefined) {
                    td.textContent = '—';
                    td.className = 'num';
                } else {
                    const z = parseFloat(val);
                    td.textContent = z.toFixed(2);
                    td.className = 'num';
                    if (z >= 0) td.classList.add('z-pos');
                    else td.classList.add('z-neg');
                    if (Math.abs(z) >= 1.5) td.classList.add('z-extreme');
                }
            } else if (col.numeric) {
                td.className = 'num';
                td.textContent = val === null || val === undefined ? '—'
                    : typeof val === 'number' ? formatNum(val) : String(val);
            } else {
                td.textContent = val === null || val === undefined ? '' : String(val);
            }

            tr.appendChild(td);
        }
        tbody.appendChild(tr);
    }
    table.appendChild(tbody);
    return table;
}

function renderGenericTable(rows, accentColor) {
    if (!rows || rows.length === 0) {
        const p = document.createElement('p');
        p.className = 'empty';
        p.textContent = '(no data)';
        return p;
    }

    const headers = [];
    const headerSet = new Set();
    for (const row of rows) {
        for (const key of Object.keys(row)) {
            if (!headerSet.has(key)) { headers.push(key); headerSet.add(key); }
        }
    }

    const table = document.createElement('table');
    const thead = document.createElement('thead');
    const hrow = document.createElement('tr');
    for (const h of headers) {
        const th = document.createElement('th');
        th.textContent = h;
        hrow.appendChild(th);
    }
    thead.appendChild(hrow);
    table.appendChild(thead);

    const tbody = document.createElement('tbody');
    for (const row of rows) {
        const tr = document.createElement('tr');
        for (const h of headers) {
            const td = document.createElement('td');
            const val = row[h];
            td.textContent = val === null || val === undefined ? '' : String(val);
            tr.appendChild(td);
        }
        tbody.appendChild(tr);
    }
    table.appendChild(tbody);
    return table;
}

function makeStatCard(key, value, accentColor) {
    const card = document.createElement('div');
    card.className = 'stat-card';

    const keyEl = document.createElement('div');
    keyEl.className = 'stat-key';
    keyEl.textContent = key;

    const valStr = String(value);
    const numVal = parseFloat(valStr);
    const isNumeric = !isNaN(numVal) && isFinite(numVal);

    const valEl = document.createElement('div');
    valEl.className = 'stat-value';
    if (isNumeric) {
        valEl.style.color = accentColor;
        valEl.textContent = formatNum(numVal);
    } else {
        valEl.textContent = valStr;
    }

    card.appendChild(keyEl);
    card.appendChild(valEl);
    return card;
}

function formatNum(n) {
    if (Number.isInteger(n)) return n.toLocaleString();
    return n.toFixed(2);
}

function setStatus(html, isError) {
    statusEl.innerHTML = html;
    statusEl.className = isError ? 'status error' : 'status';
}

// -- CSV export -----------------------------------------------------------

const btnExportCsv = document.getElementById('btn-export-csv');

function csvEscape(value) {
    const str = (value || '').trim();
    if (str.includes(',') || str.includes('"') || str.includes('\n')) {
        return '"' + str.replace(/"/g, '""') + '"';
    }
    return str;
}

function collectCsvFromTables() {
    const tables = document.querySelectorAll('#results table');
    if (tables.length === 0) return null;
    const lines = [];
    for (const table of tables) {
        // Find section title (preceding .section-title element)
        const section = table.closest('.section');
        if (section) {
            const title = section.querySelector('.section-title');
            if (title) lines.push('"' + title.textContent.replace(/"/g, '""') + '"');
        }
        // Headers
        const ths = table.querySelectorAll('thead th');
        if (ths.length > 0) {
            lines.push(Array.from(ths).map(th => csvEscape(th.textContent)).join(','));
        }
        // Rows
        const trs = table.querySelectorAll('tbody tr');
        for (const tr of trs) {
            const tds = tr.querySelectorAll('td');
            lines.push(Array.from(tds).map(td => csvEscape(td.textContent)).join(','));
        }
        lines.push(''); // blank line between tables
    }
    return lines.join('\n');
}

btnExportCsv.addEventListener('click', () => {
    const csv = collectCsvFromTables();
    if (csv) {
        vscode.postMessage({ command: 'exportCsv', csv });
    }
});
