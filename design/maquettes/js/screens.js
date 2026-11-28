import { i } from './icons.js';

// ---------- Fictitious test data ----------
const ROWS = [
  { n: 'OVHcloud', c: 'O', u: 'admin@example-studio.com', totp: '204 816', p: 35, st: 'Active', price: '€9.99', per: 'mo', lic: 1, cost: '€119.88' },
  { n: 'Cloudflare', c: 'C', u: 'admin@example-studio.com', totp: '771 302', p: 80, st: 'Free', price: '—', per: '', lic: '—', cost: '—' },
  { n: 'Figma', c: 'F', u: 'design@example-studio.com', totp: '482 913', p: 60, st: 'Active', price: '€15.00', per: 'mo', lic: 4, cost: '€720.00' },
  { n: 'Adobe Creative Cloud', c: 'A', u: 'studio@example-studio.com', totp: '', st: 'Active', price: '€69.99', per: 'mo', lic: 2, cost: '€1,679.76' },
  { n: 'Notion', c: 'N', u: 'team@example-studio.com', totp: '', st: 'Renew soon', price: '€10.00', per: 'mo', lic: 5, cost: '€600.00' },
  { n: 'Google Workspace', c: 'G', u: 'admin@example-studio.com', totp: '093 557', p: 15, st: 'Active', price: '€6.80', per: 'mo', lic: 6, cost: '€489.60' },
  { n: 'Microsoft 365', c: 'M', u: 'accounting@example-studio.com', totp: '', st: 'Active', price: '€12.50', per: 'mo', lic: 3, cost: '€450.00' },
  { n: 'Bank — business portal', c: 'B', u: '0042 1187', totp: '618 240', p: 45, st: 'Free', price: '—', per: '', lic: '—', cost: '—' },
  { n: 'GitHub', c: 'G', u: 'dev@example-studio.com', totp: '330 914', p: 70, st: 'Free', price: '—', per: '', lic: '—', cost: '—' },
  { n: 'Webflow', c: 'W', u: 'design@example-studio.com', totp: '', st: 'Pending', price: '€29.00', per: 'mo', lic: 1, cost: '€348.00' },
  { n: 'Slack', c: 'S', u: 'team@example-studio.com', totp: '', st: 'Off', price: '—', per: '', lic: '—', cost: '—' },
  { n: 'Office router', c: 'R', u: 'admin', totp: '', st: 'Free', price: '—', per: '', lic: '—', cost: '—' },
  { n: 'Canva', c: 'C', u: 'design@example-studio.com', totp: '', st: 'Active', price: '€110.00', per: 'yr', lic: 1, cost: '€110.00' },
  { n: 'Dropbox', c: 'D', u: 'studio@example-studio.com', totp: '', st: 'Off', price: '—', per: '', lic: '—', cost: '—' },
];

const statusChip = (s) => {
  const cls = s === 'Active' ? 'chip--ok' : s === 'Renew soon' ? 'chip--warn' : '';
  return `<span class="chip ${cls}"><span class="dot"></span>${s}</span>`;
};

// ---------- App frame ----------
const TABS = ['Vault', 'Contacts', 'Health', 'Activity', 'Settings'];
function shell(active, inner, { primary = `${i('plus')}New password`, overlay = '', tabs = TABS, doc = 'Example Studio' } = {}) {
  return `<div class="app">
    <header class="topbar">
      <div class="topbar-start">
        <span class="logo" aria-hidden="true">${i('key')}</span>
        <span class="doc-switch" title="Switch vault">${doc} ${i('chevron', 'chev')}</span>
      </div>
      <nav class="tabs" role="tablist">${tabs.map((t) => `<span class="tab ${t === active ? 'is-active' : ''}" role="tab">${t}</span>`).join('')}</nav>
      <div class="topbar-end">
        <span class="sync" title="Synced with vault.example-studio.com at 11:34"><span class="dot"></span>Synced</span>
        <button class="icon-btn" aria-label="Lock" title="Lock (Ctrl+L)">${i('lock')}</button>
        <span class="avatar" title="Lea Martin">LM</span>
        ${primary ? `<button class="btn btn--primary">${primary}</button>` : ''}
      </div>
    </header>
    <main class="main">${inner}</main>
    ${overlay}
  </div>`;
}

function sidebar(activeItem = 'All items') {
  const it = (ico, label, count, tip = '') =>
    `<div class="side-item ${label === activeItem ? 'is-active' : ''}" ${tip ? `title="${tip}"` : ''}>${i(ico)}<span>${label}</span>${count !== undefined ? `<span class="count">${count}</span>` : ''}</div>`;
  return `<aside class="sidebar pane">
    ${it('list', 'All items', 86)}
    ${it('star', 'Favorites', 9)}
    ${it('clock', 'Renew soon', 2)}
    <div class="side-group"><span>Personal</span></div>
    <div class="side-item" title="Only you · end-to-end encrypted">${i('user')}<span>My vault</span><span class="lock">${i('lock')}</span></div>
    <div class="side-group"><span>Example Studio</span><button class="icon-btn icon-btn--sm" aria-label="New space" title="New space">${i('plus')}</button></div>
    ${it('folder', 'General', 41)}
    ${it('folder', 'Design', 18)}
    ${it('folder', 'Accounting', 12, 'End-to-end encrypted')}
    ${it('folder', 'Dev', 9)}
    ${it('folder', 'Interns', 6)}
    <div style="flex:1"></div>
    <div class="side-item" title="Import, export">${i('upload')}<span>Import</span></div>
  </aside>`;
}

function vaultTable({ selected = 'Figma', hover = null, lic = false, aimed = null } = {}) {
  const head = `<tr>
    <th style="width:${lic ? 230 : 190}px" class="sorted">Name <span class="sort">↑</span></th>
    <th>Username</th>
    <th style="width:116px">Password</th>
    <th style="width:100px" title="Two-step verification code">2FA code</th>
    <th style="width:120px">Subscription</th>
    <th class="num" style="width:120px">Price</th>
    ${lic ? '<th class="num" style="width:72px">Licenses</th>' : ''}
    <th class="num" style="width:100px" title="Price × licenses, per year">Yearly cost</th>
  </tr>`;
  const rows = ROWS.map((r) => {
    const isSel = r.n === selected, isHov = r.n === hover;
    const acts = (isSel || isHov)
      ? `<span class="cell-acts"><button class="icon-btn icon-btn--sm" aria-label="Copy" title="Copy (Ctrl+C)">${i('copy')}</button></span>` : '';
    const pwActs = (isSel || isHov)
      ? `<span class="cell-acts"><button class="icon-btn icon-btn--sm" aria-label="Show" title="Show">${i('eye')}</button><button class="icon-btn icon-btn--sm" aria-label="Copy password" title="Copy password (Ctrl+Shift+C)">${i('copy')}</button></span>` : '';
    const totp = r.totp
      ? `<span class="totp"><span class="ring ${r.p < 25 ? 'low' : ''}" style="--p:${r.p}%"></span>${r.totp}</span>` : '<span style="color:var(--text-faint)">—</span>';
    return `<tr class="${isSel ? 'is-selected' : ''} ${isHov ? 'is-hover' : ''} ${r.n === aimed ? 'is-aimed' : ''}">
      <td><span class="site"><span class="site-ico">${r.c}</span><span class="site-name">${r.n}</span></span></td>
      <td class="muted"><span class="cellflex"><span class="ell">${r.u}</span>${acts}</span></td>
      <td><span class="cellflex"><span class="dots ell">••••••</span>${pwActs}</span></td>
      <td>${totp}</td>
      <td>${statusChip(r.st)}</td>
      <td class="num">${r.price}${r.per ? `<span style="color:var(--text-faint)"> /${r.per}</span>` : ''}</td>
      ${lic ? `<td class="num">${r.lic}</td>` : ''}
      <td class="num">${r.cost}</td>
    </tr>`;
  }).join('');
  const foot = `<tr><td colspan="${6 + (lic ? 1 : 0)}" style="color:var(--text-muted);font-size:var(--text-sm);border-bottom:0">14 items</td><td class="num" style="font-weight:var(--weight-medium);border-bottom:0" title="Total of the subscriptions shown">€4,517.24</td></tr>`;
  return `<table class="table table--fixed"><thead>${head}</thead><tbody>${rows}${foot}</tbody></table>`;
}

function vaultToolbar(colsActive = false) {
  return `<div class="toolbar">
    <div class="input search">${i('search')}<span class="grow ph">Search</span><span style="color:var(--text-faint);font-size:var(--text-sm)">Ctrl+K</span></div>
    <button class="btn btn--ghost">${i('filter')}Filter</button>
    <button class="btn ${colsActive ? '' : 'btn--ghost'}" ${colsActive ? 'style="background:var(--surface-active)"' : ''}>${i('columns')}Columns</button>
    <div style="flex:1"></div>
    <div class="segmented" role="radiogroup" aria-label="View">
      <button role="radio" aria-checked="true" title="Table">${i('list')}</button>
      <button role="radio" aria-checked="false" title="Cards">${i('columns')}</button>
    </div>
  </div>`;
}

function statusbar() {
  return `<footer class="statusbar">
    <span>General › Figma</span>
    <span class="sp"></span>
    <span class="sb-item" title="Encrypted on this device; the server keeps a recovery copy of the space key">${i('shield')}Standard encryption</span>
    <span>Saved</span>
  </footer>`;
}

function inspector() {
  return `<aside class="inspector pane">
    <div class="inspector-head">
      <span class="site-ico site-ico--lg">F</span>
      <div style="flex:1;min-width:0;display:flex;flex-direction:column">
        <span style="font-weight:var(--weight-medium);font-size:var(--text-lg)">Figma</span>
        <span style="color:var(--text-muted);font-size:var(--text-sm)">Design</span>
      </div>
      <button class="icon-btn" aria-label="Favorite" title="Favorite">${i('star')}</button>
      <button class="icon-btn" aria-label="More" title="More">${i('more')}</button>
    </div>
    <div class="inspector-tabs"><span class="is-active">Details</span><span>History</span><span>Access</span></div>
    <div class="inspector-body" style="padding-top:var(--s-3)">
      <div class="field"><span class="label">Website</span>
        <div class="input"><span class="grow">figma.com</span><span class="act"><button class="icon-btn icon-btn--sm" aria-label="Open website" title="Open website">${i('external')}</button></span></div></div>
      <div class="field"><span class="label">Username</span>
        <div class="input"><span class="grow">design@example-studio.com</span><span class="act"><button class="icon-btn icon-btn--sm" aria-label="Copy" title="Copy">${i('copy')}</button></span></div></div>
      <div class="field"><span class="label">Password</span>
        <div class="input"><span class="grow dots">••••••••••••••</span><span class="act"><button class="icon-btn icon-btn--sm" aria-label="Show" title="Show">${i('eye')}</button><button class="icon-btn icon-btn--sm" aria-label="Copy" title="Copy">${i('copy')}</button><button class="icon-btn icon-btn--sm" aria-label="Generate" title="Generate a password">${i('refresh')}</button></span></div>
        <div class="strength s4" title="Strong · 18 characters"><i></i><i></i><i></i><i></i></div></div>
      <div class="field"><span class="label">2FA code</span>
        <div class="input" style="height:36px"><span class="totp grow" style="font-size:17px;letter-spacing:.06em"><span class="ring" style="--p:60%"></span>482 913</span><span class="act"><button class="icon-btn icon-btn--sm" aria-label="Copy code" title="Copy code · 18 s left">${i('copy')}</button></span></div></div>
      <div class="field"><span class="label">Notes</span>
        <div class="input" style="height:auto;min-height:48px;align-items:flex-start;padding:6px 8px;color:var(--text-muted)">Team account · invoices in the studio's name</div></div>
      <div class="section" style="padding:var(--s-3);gap:var(--s-2);background:var(--surface-raised)">
        <div style="display:flex;align-items:center;justify-content:space-between"><h3>Subscription</h3>${statusChip('Active')}</div>
        <dl class="kv">
          <dt>Price</dt><dd class="num">€15.00 / month</dd>
          <dt>Licenses</dt><dd class="num">4</dd>
          <dt>Yearly cost</dt><dd class="num" style="font-weight:var(--weight-medium)">€720.00</dd>
          <dt>Renewal</dt><dd>Dec 3, 2026</dd>
        </dl>
      </div>
      <span style="color:var(--text-faint);font-size:var(--text-sm)">Edited Nov 18, 2026 by Hugo</span>
    </div>
  </aside>`;
}

// ---------- 1. Vault ----------
function coffre() {
  return shell('Vault', `
    <div class="work">${sidebar()}
      <section class="center">${vaultToolbar()}<div class="table-wrap">${vaultTable({ hover: 'Notion' })}</div></section>
      ${inspector()}
    </div>${statusbar()}`);
}

// ---------- 1b. Filter ----------
function filtre() {
  const opt = (label, on, count) => `<div class="menu-item"><span class="check ${on ? 'on' : ''}">${on ? i('check') : ''}</span><span>${label}</span><span class="kbd" style="margin-left:auto">${count}</span></div>`;
  const menu = `<div class="menu" style="top:40px;left:300px;width:240px">
    <div class="menu-head">Subscription</div>
    ${opt('Active', true, 7)}${opt('Renew soon', false, 1)}${opt('Pending', false, 1)}${opt('Free', false, 4)}${opt('Off', false, 2)}
    <div class="menu-sep"></div>
    <div class="menu-head">2FA</div>
    ${opt('With a 2FA code', false, 7)}${opt('Without', false, 8)}
    <div class="menu-sep"></div>
    <div class="menu-item">${i('x')}<span>Clear filters</span></div>
  </div>`;
  return shell('Vault', `
    <div class="work">${sidebar()}
      <section class="center">${vaultToolbar().replace('btn btn--ghost">' + i('filter') + 'Filter', 'btn" style="background:var(--surface-active)">' + i('filter') + 'Filter · 1')}<div class="table-wrap">${vaultTable({ selected: null })}</div>${menu}</section>
      ${inspector()}
    </div>${statusbar()}`);
}

// ---------- 2. Columns ----------
function colonnes() {
  const col = (ico, name, on, locked = false) =>
    `<div class="menu-item"><span class="drag">${i('grip')}</span><span class="check ${on ? 'on' : ''}">${on ? i('check') : ''}</span>${i(ico)}<span>${name}</span>${locked ? `<span class="kbd" title="Always shown">${i('lock')}</span>` : ''}</div>`;
  const menu = `<div class="menu" style="top:40px;left:614px;width:260px">
    <div class="menu-head">Shown</div>
    ${col('text', 'Name', true, true)}${col('user', 'Username', true)}${col('key', 'Password', true)}${col('totp', '2FA code', true)}${col('list', 'Subscription', true)}${col('euro', 'Price', true)}${col('hash', 'Licenses', true)}${col('sigma', 'Yearly cost', true)}
    <div class="menu-sep"></div>
    <div class="menu-head">Hidden</div>
    ${col('link', 'Website', false)}${col('calendar', 'Renewal', false)}${col('note', 'Notes', false)}${col('folder', 'Space', false)}
    <div class="menu-sep"></div>
    <div class="menu-item is-hover">${i('plus')}<span>New column</span></div>
  </div>`;
  const type = (ico, n, sel = false) => `<div class="menu-item ${sel ? 'is-hover' : ''}">${i(ico)}<span>${n}</span>${sel ? `<span class="kbd">${i('check')}</span>` : ''}</div>`;
  const create = `<div class="menu" style="top:40px;left:882px;width:260px;padding:var(--s-3);display:flex;flex-direction:column;gap:var(--s-3)">
    <div class="field"><span class="label">Name</span><div class="input is-focus"><span class="grow">Cancellation date</span></div></div>
    <div class="field"><span class="label">Type</span>
      <div style="display:flex;flex-direction:column;margin:0 -4px">
        ${type('text', 'Text')}${type('lock', 'Secret text')}${type('hash', 'Number')}${type('euro', 'Price')}${type('calendar', 'Date', true)}${type('list', 'Choice list')}${type('totp', '2FA code')}${type('sigma', 'Formula')}${type('link', 'Web address')}
      </div></div>
    <div class="row" style="padding:0"><span class="row-label" title="Shown in every Example Studio space">All spaces</span><span class="switch on"></span></div>
    <div class="dialog-actions"><button class="btn btn--ghost">Cancel</button><button class="btn btn--primary">Create</button></div>
  </div>`;
  return shell('Vault', `
    <div class="work work--no-inspector">${sidebar()}
      <section class="center">${vaultToolbar(true)}<div class="table-wrap">${vaultTable({ selected: null, lic: true })}</div>${menu}${create}</section>
    </div>${statusbar()}`);
}

// ---------- 3. First launch ----------
function premierLancement() {
  return `<div class="app"><main class="main" style="margin:6px"><div class="center-stage">
    <div class="card" style="width:460px">
      <div class="card-head"><span class="logo logo--lg">${i('key')}</span><h1>Welcome to Cleis</h1></div>
      <div class="choice"><span class="radio"></span><div class="txt"><b style="font-weight:var(--weight-medium)">This computer only</b><span>Personal vault</span></div><span class="ico">${i('laptop')}</span></div>
      <div class="choice is-active"><span class="radio on"></span><div class="txt"><b style="font-weight:var(--weight-medium)">Join an organization</b><span>Shared spaces, on top of your personal vault</span>
        <div class="field" style="margin-top:var(--s-3)"><span class="label" title="Cleis server (https://…), network folder (\\\\server\\share), SFTP (sftp://…) or private GitHub repository">Server or folder address</span><div class="input"><span class="grow">vault.example-studio.lan</span><span class="chip chip--ok" title="Cleis server reachable through your VPN · certificate trusted">${i('server')}Server</span></div></div>
        <div class="field" style="margin-top:var(--s-2)"><span class="label">Invitation code</span><div class="input is-focus"><span class="grow code">K7Q2-9MXA</span></div></div>
      </div><span class="ico">${i('server')}</span></div>
      <div style="display:flex;justify-content:space-between;align-items:center">
        <button class="btn btn--ghost" title="From GitHub, OneDrive, Google Drive, SFTP or a setup code">${i('sync')}Restore my vault</button>
        <button class="btn btn--primary">Continue</button>
      </div>
    </div></div></main></div>`;
}

// ---------- 4. Unlock ----------
function deverrouiller() {
  return `<div class="app"><main class="main" style="margin:6px"><div class="center-stage">
    <div class="card">
      <div class="card-head"><span class="logo logo--lg">${i('key')}</span><h1>Cleis</h1></div>
      <div class="section" style="gap:var(--s-4)">
        <div class="member"><span class="avatar" style="width:32px;height:32px">LM</span><div class="meta"><span style="font-weight:var(--weight-medium)">Lea Martin</span><span class="sub">lea@example-studio.com</span></div><button class="btn btn--ghost btn--sm" style="margin-left:auto">Switch</button></div>
        <div class="field"><span class="label">Master password</span>
          <div class="input is-focus"><span class="grow dots">••••••••••••</span><span class="act"><button class="icon-btn icon-btn--sm" aria-label="Show" title="Show">${i('eye')}</button></span></div></div>
        <button class="btn btn--primary" style="justify-content:center;height:32px">Unlock</button>
      </div>
      <div style="display:flex;justify-content:center"><button class="btn btn--ghost btn--sm">Forgot password</button></div>
    </div></div></main></div>`;
}

// ---------- 5. 2FA code ----------
function codeVerif() {
  return `<div class="app"><main class="main" style="margin:6px"><div class="center-stage">
    <div class="card">
      <div class="card-head">
        <span class="logo logo--lg">${i('shield')}</span>
        <h1>Verification code</h1>
        <span style="color:var(--text-muted)">Authenticator app</span>
      </div>
      <div class="section" style="gap:var(--s-4);align-items:stretch">
        <div class="otp"><span>4</span><span>8</span><span>2</span><span class="gap"></span><span class="is-focus"></span><span></span><span></span></div>
        <div class="row" style="padding:0;justify-content:center;gap:var(--s-2)"><span class="check on">${i('check')}</span><span title="Duration set by your organization">Remember this device for 30 days</span></div>
        <button class="btn btn--primary" style="justify-content:center;height:32px">Verify</button>
      </div>
      <div style="display:flex;justify-content:center;gap:var(--s-2)">
        <button class="btn btn--ghost btn--sm">${i('mail')}Send by email</button>
        <button class="btn btn--ghost btn--sm">Use a recovery code</button>
      </div>
    </div></div></main></div>`;
}

// ---------- Settings navigation ----------
function settingsNav(active, admin = true) {
  const it = (ico, n) => `<div class="side-item ${n === active ? 'is-active' : ''}">${i(ico)}<span>${n}</span></div>`;
  return `<nav class="settings-nav">
    <div class="side-group" style="padding-top:var(--s-1)">My account</div>
    ${it('cloud', 'Account')}${it('user', 'Profile')}${it('lock', 'Security')}${it('laptop', 'Devices')}${it('gear', 'Preferences')}${it('fill', 'Autofill')}${it('sync', 'Sync & backups')}
    ${admin ? `<div class="side-group">Example Studio</div>
    ${it('users', 'Members')}${it('folder', 'Spaces')}${it('shield', 'Security policy')}${it('key', 'Encryption')}${it('server', 'Server')}${it('upload', 'Import & export')}` : ''}
  </nav>`;
}
const settings = (active, body, overlay = '', scroll = 0) => shell('Settings', `<div class="settings">${settingsNav(active)}<div class="settings-body"><div class="page" ${scroll ? `style="margin-top:-${scroll}px"` : ''}>${body}</div></div></div>`, { primary: null, overlay });
const keys = (...k) => `<span class="keys">${k.map((x) => `<kbd>${x}</kbd>`).join('')}</span>`;
const sectionTitle = (t, tip) => `<div class="section-title"><h2>${t}</h2>${tip ? `<i class="info" title="${tip}">i</i>` : ''}</div>`;
const inherited = (label) => `<span class="chip chip--warn" title="Set by your organization">${label}</span>`;

// ---------- 6. Account security ----------
function securite() {
  return settings('Security', `
    <div class="page-head"><h1>Security</h1></div>
    <div class="section">${sectionTitle('Opening the app', 'Code asked each time Cleis opens, on top of the master password. Also protects your personal vault offline.')}
      <div class="row"><span class="row-label">Verification code on open</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Method</span>
        <div class="segmented" role="radiogroup"><button role="radio" aria-checked="true">${i('phone')}App</button><button role="radio" aria-checked="false">${i('mail')}Email</button><button role="radio" aria-checked="false">SMS</button></div></div>
      <div class="row"><span class="row-label">Recovery codes</span><span style="color:var(--text-muted);font-size:var(--text-sm)">8 left</span><button class="btn">Show</button></div>
    </div>
    <div class="section">${sectionTitle('Signing in to the server', 'Rules set by the Example Studio administrator.')}
      <div class="row"><span class="row-label" style="color:var(--inherit-text)">Verification code</span>${inherited('Required')}</div>
      <div class="row"><span class="row-label">Method</span><div class="input select" style="width:220px"><span class="grow">Authenticator app</span></div></div>
      <div class="row"><span class="row-label" style="color:var(--inherit-text)">Remember device</span>${inherited('30 days')}</div>
    </div>
    <div class="section">${sectionTitle('Locking')}
      <div class="row"><span class="row-label" style="color:var(--inherit-text)" title="Set by your organization">After inactivity</span>${inherited('Never')}</div>
    </div>
    <div class="section">${sectionTitle('Clipboard')}
      <div class="row"><span class="row-label">Clear after copying</span><div class="input select" style="width:120px"><span class="grow">30 s</span></div><span class="switch on"></span></div>
    </div>
    <div class="section">${sectionTitle('Master password')}
      <div class="row"><span class="row-label" style="color:var(--text-muted)">Changed Oct 2, 2026</span><button class="btn">Change</button></div>
    </div>`);
}

// ---------- 7. Preferences (theme, language) ----------
function preferences() {
  const sw = (c, on = false, label) => `<button class="swatch ${on ? 'on' : ''}" style="${c ? `background:${c}` : ''}" aria-label="${label}" title="${label}">${c ? '' : i('x')}</button>`;
  return settings('Preferences', `
    <div class="page-head"><h1>Preferences</h1></div>
    <div class="section">${sectionTitle('Appearance')}
      <div class="row"><span class="row-label">Theme</span><div class="segmented" role="radiogroup" aria-label="Theme"><button role="radio" aria-checked="true">Dark</button><button role="radio" aria-checked="false">Light</button><button role="radio" aria-checked="false" title="Follow Windows or macOS">System</button></div></div>
      <div class="row"><span class="row-label">Accent color</span><div class="swatches">${sw('', false, 'None')}${sw('var(--accent)', true, 'Ochre (default)')}${sw('#3f5a75', false, 'Blue')}${sw('#494a6a', false, 'Violet')}${sw('#48664f', false, 'Green')}${sw('#3f6766', false, 'Teal')}${sw('#7a4646', false, 'Red')}</div></div>
      <div class="row"><span class="row-label" title="Where the accent color is used">Accent scope</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="false" title="Focus and window outline">Minimal</button><button role="radio" aria-checked="false" title="Also the main button, switches and selection">Standard</button><button role="radio" aria-checked="true" title="Also tabs, choices and active rows">Extended</button></div></div>
    </div>
    <div class="section">${sectionTitle('Tabs', 'Hidden tabs keep their data.')}
      <div class="row"><span class="row-label">${i('users')}Contacts</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label" title="Shown to administrators only">${i('shield')}Health</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label" title="Shown to administrators only · hiding the tab does not stop recording">${i('clock')}Activity</span><span class="switch on"></span></div>
    </div>
    <div class="section">${sectionTitle('Language and region')}
      <div class="row"><span class="row-label">Language</span><div class="input select" style="width:200px"><span class="grow">English</span></div></div>
      <div class="row"><span class="row-label">Currency</span><div class="input select" style="width:200px"><span class="grow">Euro (€)</span></div></div>
    </div>
    <div class="section">${sectionTitle('Startup')}
      <div class="row"><span class="row-label">Open at login</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Start in</span><div class="input select" style="width:200px"><span class="grow">All items</span></div></div>
    </div>`);
}

// ---------- 8. Members ----------
function membres(menuOn = null) {
  const m = (ini, n, e, role, groups, tfa, exp, last, lockExp = false) => `<tr>
    <td><div class="member"><span class="avatar">${ini}</span><div class="meta"><span>${n}</span><span class="sub">${e}</span></div></div></td>
    <td>${ini === 'LM' ? `<span style="color:var(--text-muted)" title="You">${role}</span>` : `<div class="input select" style="width:120px;height:24px"><span class="grow">${role}</span></div>`}</td>
    <td>${groups.map((g) => `<span class="chip" style="margin-right:4px">${g}</span>`).join('')}</td>
    <td>${tfa ? '<span class="chip chip--ok"><span class="dot"></span>On</span>' : '<span class="chip chip--warn" title="Asked at next sign-in"><span class="dot"></span>To set up</span>'}</td>
    <td title="${lockExp ? 'Administrators can always export' : 'Can export the organization’s passwords'}"><span class="switch ${exp ? 'on' : ''} ${lockExp ? 'planned' : ''}"></span></td>
    <td class="muted">${last}</td>
    <td style="position:relative">${ini === 'LM' ? '' : `<button class="icon-btn icon-btn--sm" aria-label="More" title="More">${i('more')}</button>`}${menuOn === ini ? `<div class="menu" style="top:30px;right:0;width:220px">
      <div class="menu-item">${i('shield')}<span>Make Super Admin</span></div>
      <div class="menu-item" title="You become Admin">${i('refresh')}<span>Transfer Super Admin</span></div>
      <div class="menu-sep"></div>
      <div class="menu-item" style="color:var(--danger-text)">${i('trash')}<span>Remove</span></div></div>` : ''}</td></tr>`;
  const perm = (n, cells) => `<tr><td>${n}</td>${cells.map((c) => `<td style="text-align:center">${c ? `<span class="check on" style="display:inline-grid">${i('check')}</span>` : '<span class="check" style="display:inline-grid"></span>'}</td>`).join('')}</tr>`;
  return settings('Members', `
    <div class="page-head"><h1>Members</h1><div class="input search" style="width:220px">${i('search')}<span class="grow ph">Search</span></div><button class="btn btn--primary">${i('plus')}Invite</button></div>
    <div class="section" style="padding:0;overflow:hidden">
      <table class="table table--roomy"><thead><tr><th>Member</th><th>Role</th><th>Spaces</th><th>2FA</th><th style="width:70px" title="Allowed to export the organization’s passwords">Export</th><th>Last sign-in</th><th style="width:40px"></th></tr></thead><tbody>
      ${m('LM', 'Lea Martin', 'lea@example-studio.com', 'Super Admin', ['All'], true, true, 'Today, 9:12 AM', true)}
      ${m('HB', 'Hugo Bernard', 'hugo@example-studio.com', 'Member', ['General', 'Design', 'Dev'], true, false, 'Today, 10:47 AM')}
      ${m('IP', 'Ines Petit', 'ines@example-studio.com', 'Member', ['General', 'Accounting'], true, true, 'Yesterday, 5:30 PM')}
      ${m('SR', 'Sarah Roux', 'sarah@example-studio.com', 'Read only', ['General'], false, false, 'Nov 20, 2026')}
      ${m('TD', 'Tom Durand', 'tom@example-studio.com', 'Intern', ['Interns'], true, false, 'Nov 21, 2026')}
      <tr><td colspan="7" style="color:var(--text-muted);border-bottom:0"><div class="member"><span class="avatar" style="background:transparent;border:1px dashed var(--line-strong)">${i('mail')}</span><div class="meta"><span>noa@example-studio.com</span><span class="sub">Invited Nov 22 · expires in 5 days</span></div><button class="btn btn--ghost btn--sm" style="margin-left:auto">Resend</button><button class="btn btn--danger-ghost btn--sm">Revoke</button></div></td></tr>
      </tbody></table>
    </div>
    <div class="section">${sectionTitle('Roles', 'What each role can do in the spaces it can access.')}
      <table class="table" style="margin-top:-4px"><thead><tr><th></th><th style="text-align:center" title="The organization's creator; can be transferred or shared">Super Admin</th><th style="text-align:center">Admin</th><th style="text-align:center">Member</th><th style="text-align:center">Read only</th><th style="text-align:center" title="Copy and fill, without showing">Intern</th></tr></thead><tbody>
        ${perm('See passwords', [1, 1, 1, 1, 0])}
        ${perm('Copy, fill', [1, 1, 1, 1, 1])}
        ${perm('Add, edit', [1, 1, 1, 0, 0])}
        ${perm('Delete', [1, 1, 1, 0, 0])}
        ${perm('Manage members and spaces', [1, 1, 0, 0, 0])}
        ${perm('Activity log, health report', [1, 1, 0, 0, 0])}
        ${perm('Admins, roles, server keys', [1, 0, 0, 0, 0])}
      </tbody></table>
      <div><button class="btn btn--ghost btn--sm">${i('plus')}New role</button></div>
    </div>`);
}

// ---------- 9. Security policy ----------
function politique(scroll = 0) {
  const meth = (ico, n, on) => `<div class="row plain"><span class="check ${on ? 'on' : ''}">${on ? i('check') : ''}</span>${i(ico)}<span class="row-label">${n}</span></div>`;
  return settings('Security policy', `
    <div class="page-head"><h1>Security policy</h1></div>
    <div class="section">${sectionTitle('Signing in to the server', 'Applies to every Example Studio member.')}
      <div class="row"><span class="row-label">Verification code</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="false">Optional</button><button role="radio" aria-checked="true">Required</button></div></div>
      <div class="row"><span class="row-label">Remember device</span><div class="input select" style="width:160px"><span class="grow">30 days</span></div></div>
    </div>
    <div class="section">${sectionTitle('Allowed methods')}
      <div class="grid-2" style="gap:0 var(--s-6)">
        <div>${meth('phone', 'Authenticator app', 1)}${meth('mail', 'Email', 1)}${meth('phone', 'SMS', 0)}</div>
        <div>${meth('globe', 'Google account', 0)}${meth('globe', 'Microsoft account', 1)}</div>
      </div>
    </div>
    <div class="section">${sectionTitle('Locking')}
      <div class="row"><span class="row-label">After inactivity</span><div class="input select" style="width:160px"><span class="grow">Never</span></div></div>
      <div class="row"><span class="row-label" title="How long the vault stays readable without a connection">Offline access</span><div class="input select" style="width:160px"><span class="grow">14 days</span></div></div>
    </div>
    <div class="section">${sectionTitle('Passwords')}
      <div class="row"><span class="row-label">Flag shorter than</span><div class="input" style="width:80px"><span class="grow num">12</span></div></div>
      <div class="row"><span class="row-label" title="Flagged in the health report">Flag older than</span><div class="input select" style="width:160px"><span class="grow">1 year</span></div></div>
    </div>
    <div class="section">${sectionTitle('Activity log', 'Server: the server writes the log. Shared folder: each app writes its own signed, chained log; edits and gaps are detected, not prevented.')}
      <div class="row"><span class="row-label">Record activity</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Detail</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="false">Admin actions</button><button role="radio" aria-checked="false" title="Admin actions, creations, edits, deletions">Changes</button><button role="radio" aria-checked="true" title="Also views, copies and fills">Everything</button></div></div>
      <div class="row"><span class="row-label">Keep</span><div class="input select" style="width:160px"><span class="grow">1 year</span></div></div>
      <div class="row"><span class="row-label">Integrity</span><span class="chip chip--ok" title="Every signature and link checked · 6 devices">${i('chain')}Verified today, 11:30</span><button class="btn">Verify now</button></div>
    </div>
    <div class="section">${sectionTitle('Autofill', 'Ways members may fill passwords outside Cleis.')}
      <div class="grid-2" style="gap:0 var(--s-6)">
        <div>${meth('fill', 'Fill fields in apps', 1)}${meth('keyboard', 'Type keys', 1)}</div>
        <div>${meth('window', 'Read browser addresses', 1)}</div>
      </div>
    </div>`, '', scroll);
}
function politiqueSuite() { return politique(400); }

// ---------- 10. Encryption (+ re-encrypt dialog) ----------
function chiffrementBody() {
  const mode = (m) => `<div class="segmented" role="radiogroup"><button role="radio" aria-checked="${m === 's'}">Standard</button><button role="radio" aria-checked="${m === 'e'}">End-to-end</button></div>`;
  const sp = (n, items, members, m, changed = false) => `<tr>
    <td><span class="site">${i('folder')}<span>${n}</span></span></td>
    <td class="num muted">${items}</td><td class="num muted">${members}</td>
    <td>${mode(m)}${changed ? ' <span class="chip chip--set" style="margin-left:6px">Changed</span>' : ''}</td></tr>`;
  return `
    <div class="page-head"><h1>Encryption</h1></div>
    <div class="section">${sectionTitle('Mode per space', 'Both modes encrypt on the device. Standard: the server keeps a recovery copy of the space key, so an admin can restore access. End-to-end: only the members hold the key; the recovery key is essential.')}
      <table class="table"><thead><tr><th>Space</th><th class="num" style="width:90px">Items</th><th class="num" style="width:90px">Members</th><th style="width:300px">Mode</th></tr></thead><tbody>
        <tr><td><span class="site">${i('user')}<span>Personal vaults</span></span></td><td class="num muted">—</td><td class="num muted">6</td><td>${mode('e')}</td></tr>
        ${sp('General', 41, 6, 's')}${sp('Design', 18, 3, 's')}${sp('Accounting', 12, 2, 'e', true)}${sp('Dev', 9, 2, 's')}${sp('Interns', 6, 2, 's')}
      </tbody></table>
      <div style="display:flex;justify-content:flex-end;gap:var(--s-2)"><button class="btn btn--ghost">Cancel</button><button class="btn btn--primary">${i('refresh')}Re-encrypt</button></div>
    </div>
    <div class="section">${sectionTitle('Recovery key', 'Restores an end-to-end space when a member loses their master password.')}
      <div class="row"><span class="row-label">Admin recovery</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Recovery key</span><span class="chip chip--ok"><span class="dot"></span>Downloaded Nov 14</span><button class="btn">${i('download')}Download</button></div>
    </div>
    <div class="section">${sectionTitle('Server keys')}
      <div class="row"><span class="row-label">Fingerprint</span><span class="code" style="color:var(--text-muted)">SHA256:4f1c·9a2e·77b0·d31f</span><button class="icon-btn" aria-label="Copy" title="Copy">${i('copy')}</button></div>
      <div class="row"><span class="row-label">Rotate server key</span><button class="btn">${i('refresh')}Rotate</button></div>
    </div>`;
}
function chiffrement() { return settings('Encryption', chiffrementBody()); }
function rechiffrer() {
  const overlay = `<div class="overlay"><div class="dialog" role="dialog" aria-label="Re-encrypt Accounting">
    <h2>Re-encrypt “Accounting”</h2>
    <div class="row" style="padding:0;gap:var(--s-2)"><span class="chip">Standard</span>${i('arrowR')}<span class="chip chip--set">End-to-end</span></div>
    <dl class="kv"><dt>Items</dt><dd class="num">12</dd><dt>Members</dt><dd>Lea, Ines</dd><dt>Recovery key</dt><dd style="color:var(--success-text)">Included</dd></dl>
    <div class="row" style="padding:0;border:0;gap:var(--s-2)"><span class="check on">${i('check')}</span><span>Back up first</span></div>
    <div class="dialog-actions"><button class="btn" style="outline:2px solid var(--focus);outline-offset:1px">Cancel</button><button class="btn btn--primary">Re-encrypt</button></div>
  </div></div>`;
  return settings('Encryption', chiffrementBody(), overlay);
}

// ---------- 11. Activity log ----------
function journal() {
  const A = { Copied: '', Viewed: '', Filled: 'chip--auto', Edited: 'chip--set', Created: 'chip--ok', Deleted: 'chip--danger', Exported: 'chip--warn', 'Signed in': '' };
  const r = (d, who, a, item, sp, dev) => `<tr><td class="muted">${d}</td><td>${who}</td><td><span class="chip ${A[a]}">${a}</span></td><td>${item}</td><td class="muted">${sp}</td><td class="muted">${dev}</td></tr>`;
  return shell('Activity', `
    <div class="toolbar" style="gap:var(--s-2)">
      <div class="input search" style="width:240px">${i('search')}<span class="grow ph">Search</span></div>
      <div class="input select" style="width:150px"><span class="grow">All members</span></div>
      <div class="input select" style="width:150px"><span class="grow">All actions</span></div>
      <div class="input select" style="width:150px"><span class="grow">All spaces</span></div>
      <div class="input select" style="width:150px"><span class="grow">Last 7 days</span></div>
      <div style="flex:1"></div>
      <button class="btn btn--ghost" title="Check every signature and link">${i('chain')}Verify</button>
      <button class="btn">${i('download')}Export</button>
    </div>
    <div class="table-wrap">
      <table class="table"><thead><tr><th style="width:170px" class="sorted">Date <span class="sort">↓</span></th><th style="width:170px">Member</th><th style="width:130px">Action</th><th>Item</th><th style="width:140px">Space</th><th style="width:220px">Device</th></tr></thead><tbody>
        ${r('Today, 11:31 AM', 'Hugo Bernard', 'Copied', 'Figma · password', 'Design', 'PC-HUGO · Windows 11')}
        ${r('Today, 11:29 AM', 'Tom Durand', 'Filled', 'Canva', 'Interns', 'MAC-TOM · Chrome')}
        ${r('Today, 10:58 AM', 'Ines Petit', 'Viewed', 'Bank — business portal', 'Accounting', 'PC-INES · Windows 11')}
        ${r('Today, 10:47 AM', 'Hugo Bernard', 'Signed in', '—', '—', 'PC-HUGO · Windows 11')}
        ${r('Today, 9:40 AM', 'Lea Martin', 'Edited', 'OVHcloud · password', 'General', 'MAC-LEA · macOS 15')}
        ${r('Today, 9:38 AM', 'Lea Martin', 'Created', 'Webflow', 'Design', 'MAC-LEA · macOS 15')}
        ${r('Yesterday, 6:02 PM', 'Lea Martin', 'Exported', 'General (41 items)', 'General', 'MAC-LEA · macOS 15')}
        ${r('Yesterday, 5:30 PM', 'Ines Petit', 'Copied', 'Microsoft 365 · username', 'Accounting', 'PC-INES · Windows 11')}
        ${r('Yesterday, 4:12 PM', 'Hugo Bernard', 'Deleted', 'Old hosting', 'Dev', 'PC-HUGO · Windows 11')}
        ${r('Yesterday, 3:44 PM', 'Tom Durand', 'Filled', 'Google Workspace', 'Interns', 'MAC-TOM · Chrome')}
        ${r('Yesterday, 11:20 AM', 'Sarah Roux', 'Viewed', 'Slack', 'General', 'PC-SARAH · Windows 10')}
        ${r('Nov 21, 5:55 PM', 'Hugo Bernard', 'Edited', 'GitHub · 2FA code', 'Dev', 'PC-HUGO · Windows 11')}
        ${r('Nov 21, 9:03 AM', 'Tom Durand', 'Signed in', '—', '—', 'MAC-TOM · macOS 14')}
      </tbody></table>
    </div>
    <footer class="statusbar"><span>214 events</span><span class="sp"></span><span class="sb-item" style="color:var(--success-text)" title="Each event is signed by the device that wrote it and linked to the previous one">${i('chain')}Signed and chained · verified 11:30</span><span>Kept 1 year</span></footer>`, { primary: null });
}

// ---------- 12. Health ----------
function sante() {
  const pb = (t, cls) => `<span class="chip ${cls}" style="margin-right:4px">${t}</span>`;
  const r = (c, n, probs, sp, ch) => `<tr><td><span class="site"><span class="site-ico">${c}</span><span class="site-name">${n}</span></span></td><td>${probs}</td><td class="muted">${sp}</td><td class="muted">${ch}</td><td><button class="btn btn--sm">Change</button></td></tr>`;
  return shell('Health', `
    <div class="settings-body" style="padding-top:var(--s-6)"><div class="page" style="max-width:1100px">
      <div class="page-head"><h1>Health report</h1><div class="input select" style="width:170px"><span class="grow">All spaces</span></div><button class="btn">${i('refresh')}Scan</button></div>
      <div class="grid-3" style="grid-template-columns:repeat(5,1fr)">
        <div class="section stat danger"><b>4</b><span>Weak</span></div>
        <div class="section stat warn"><b>6</b><span>Reused</span></div>
        <div class="section stat warn"><b>9</b><span>Over a year old</span></div>
        <div class="section stat"><b>5</b><span title="The service offers two-step verification">No 2FA</span></div>
        <div class="section stat ok"><b>62</b><span>No issue</span></div>
      </div>
      <div class="section" style="padding:0;overflow:hidden">
        <table class="table"><thead><tr><th>Item</th><th>Issues</th><th style="width:140px">Space</th><th style="width:160px">Last changed</th><th style="width:100px"></th></tr></thead><tbody>
          ${r('N', 'Notion', pb('Weak', 'chip--danger') + pb('Reused ×3', 'chip--warn'), 'General', 'Mar 14, 2024')}
          ${r('S', 'Slack', pb('Reused ×3', 'chip--warn') + pb('Over a year', 'chip--warn'), 'General', 'Feb 2, 2025')}
          ${r('R', 'Office router', pb('Weak', 'chip--danger') + pb('Over a year', 'chip--warn'), 'General', 'Sep 9, 2023')}
          ${r('D', 'Dropbox', pb('Reused ×3', 'chip--warn'), 'General', 'Jun 30, 2025')}
          ${r('A', 'Adobe Creative Cloud', pb('No 2FA', ''), 'Design', 'Aug 12, 2026')}
          ${r('M', 'Microsoft 365', pb('Weak', 'chip--danger') + pb('No 2FA', ''), 'Accounting', 'Jan 4, 2025')}
          ${r('W', 'Webflow', pb('No 2FA', ''), 'Design', 'Today')}
        </tbody></table>
      </div>
    </div></div>`, { primary: null });
}

// ---------- 13. Backups ----------
function sauvegardes() {
  const h = (d, where, size, ok) => `<tr><td>${d}</td><td class="muted">${where}</td><td class="num muted">${size}</td><td>${ok ? '<span class="chip chip--ok" title="Test restore succeeded"><span class="dot"></span>Verified</span>' : '<span class="chip">Not verified</span>'}</td><td style="text-align:right"><button class="btn btn--ghost btn--sm">Verify</button><button class="btn btn--sm">Restore</button></td></tr>`;
  return settings('Sync & backups', `
    <div class="page-head"><h1 style="flex:none">Sync & backups</h1><div class="segmented" role="radiogroup" aria-label="Whose vault" title="Your vault, or the organization's (administrators)"><button role="radio" aria-checked="false">Mine</button><button role="radio" aria-checked="true">Example Studio</button></div><span style="flex:1"></span><button class="btn btn--primary">${i('upload')}Back up now</button></div>
    <div class="section">${sectionTitle('GitHub', 'Encrypted file pushed to a private repository.')}
      <div class="row"><span class="site-ico">${i('repo')}</span><div class="member row-label"><div class="meta"><span class="code">example-studio/vault-backups</span><span class="sub">Connected as example-studio</span></div></div><span class="chip">Private</span><button class="btn btn--ghost btn--sm">Disconnect</button></div>
      <div class="row"><span class="row-label">Frequency</span><div class="input select" style="width:160px"><span class="grow">Daily</span></div><span class="switch on"></span></div>
    </div>
    <div class="section">${sectionTitle('SFTP', 'Encrypted file server (SSH). Plain FTP is not offered.')}
      <div class="grid-3" style="grid-template-columns:2fr 80px 1fr">
        <div class="field"><span class="label">Host</span><div class="input"><span class="grow">nas.example-studio.lan</span></div></div>
        <div class="field"><span class="label">Port</span><div class="input"><span class="grow num">22</span></div></div>
        <div class="field"><span class="label">User</span><div class="input"><span class="grow">backup</span></div></div>
      </div>
      <div class="grid-2">
        <div class="field"><span class="label">Folder</span><div class="input"><span class="grow">/backups/cleis</span></div></div>
        <div class="field"><span class="label">Authentication</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="false">Password</button><button role="radio" aria-checked="true">SSH key</button></div></div>
      </div>
      <div class="row"><span class="row-label">Frequency</span><div class="input select" style="width:160px"><span class="grow">Weekly</span></div><button class="btn">Test connection</button><span class="switch"></span></div>
    </div>
    <div class="section">${sectionTitle('Backup key', 'Without it, no backup can be restored. Keep it outside Cleis.')}
      <div class="row"><span class="row-label" style="color:var(--warning-text)">${i('warn')}Not downloaded yet</span><button class="btn">${i('download')}Download</button></div>
    </div>
    <div class="section" style="padding-bottom:var(--s-2)">${sectionTitle('History')}
      <table class="table"><thead><tr><th>Date</th><th>Destination</th><th class="num">Size</th><th>Status</th><th></th></tr></thead><tbody>
        ${h('Today, 3:00 AM', 'GitHub', '184 KB', true)}${h('Yesterday, 3:00 AM', 'GitHub', '183 KB', true)}${h('Nov 22, 3:00 AM', 'GitHub', '183 KB', false)}
      </tbody></table>
    </div>`);
}

// ---------- 14. Import & export ----------
function importCsv() {
  const map = (src, sample, dst) => `<div class="map-row"><div class="src">${i('columns')}<span>${src}</span></div><span class="arrow">${i('arrowR')}</span><div class="input select"><span class="grow" ${dst === 'Skip' ? 'style="color:var(--text-faint)"' : ''}>${dst}</span></div><span class="sample">${sample}</span></div>`;
  return settings('Import & export', `
    <div class="page-head"><h1>Import</h1>
      <div class="steps"><span class="step done"><b>${i('check')}</b>File</span><span class="line"></span><span class="step is-active"><b>2</b>Columns</span><span class="line"></span><span class="step"><b>3</b>Review</span></div></div>
    <div class="dropzone">${i('file')}<div class="member row-label"><div class="meta"><span>notion-export.csv</span><span class="sub">86 rows · 9 columns</span></div></div><div class="field" style="width:200px"><span class="label">Into space</span><div class="input select"><span class="grow">General</span></div></div><button class="btn btn--ghost btn--sm">Change</button></div>
    <div class="section">
      <div class="map-row" style="font-size:var(--text-xs);color:var(--text-muted);font-weight:var(--weight-medium)"><span>File column</span><span></span><span>Cleis column</span><span>Sample</span></div>
      ${map('Site/app', 'Figma', 'Name')}
      ${map('ID', 'design@example-studio.com', 'Username')}
      ${map('MDP', '••••••••', 'Password')}
      ${map('URL', 'https://figma.com', 'Website')}
      ${map('Abonnement', 'actif, gratuit, off, en cours', 'Subscription')}
      ${map('Prix (mensuel)', '15 €', 'Price')}
      ${map('N° de licences', '4', 'Licenses')}
      ${map('Cout total', '60 €', 'Skip')}
      ${map('En +', 'Compte équipe…', 'Notes')}
    </div>
    <div class="row" style="padding:0;gap:var(--s-2)"><span class="chip chip--auto" title="Matched automatically, please review">8 columns matched</span><span class="chip" title="Yearly cost is computed by Cleis">1 skipped</span></div>
    <div style="display:flex;justify-content:flex-end;gap:var(--s-2)"><button class="btn btn--ghost">Back</button><button class="btn btn--primary">Continue</button></div>`);
}


// ---------- 14b. Import: choose a file ----------
const importSteps = (n) => {
  const st = (k, label) => `<span class="step ${k < n ? 'done' : k === n ? 'is-active' : ''}"><b>${k < n ? i('check') : k}</b>${label}</span>`;
  return `<div class="steps">${st(1, 'File')}<span class="line"></span>${st(2, 'Columns')}<span class="line"></span>${st(3, 'Review')}</div>`;
};
function importFichier() {
  return settings('Import & export', `
    <div class="page-head"><h1>Import</h1>${importSteps(1)}</div>
    <div class="dropzone" style="padding:var(--s-6) var(--s-4);flex-direction:column;justify-content:center">
      ${i('upload')}
      <span style="color:var(--text-muted)" title="Notion, Excel, Google Sheets, Bitwarden, 1Password, Chrome, Firefox (CSV)">Drop a CSV file here</span>
      <button class="btn">Choose a file</button>
    </div>
    <div class="section">${sectionTitle('Export')}
      <div class="row"><span class="row-label" title="Readable only with the password you choose">Encrypted export</span><button class="btn btn--sm">${i('download')}Export</button></div>
    </div>`);
}

// ---------- 14c. Import: review ----------
function importRevue() {
  const w = (line, msg) => `<tr><td class="num muted" style="width:64px">${line}</td><td>${msg}</td></tr>`;
  return settings('Import & export', `
    <div class="page-head"><h1>Import</h1>${importSteps(3)}</div>
    <div class="section">
      <div class="row" style="border:0;padding-top:0"><span class="row-label">${i('file')}notion-export.csv</span><span class="muted" style="font-size:var(--text-sm)">into General</span></div>
      <div class="grid-3">
        <div class="field"><span class="label">New items</span><span style="font-size:var(--text-xl);font-weight:var(--weight-strong)">84</span></div>
        <div class="field"><span class="label" title="Same name and username as an item already in the space">Already in the vault</span><span style="font-size:var(--text-xl);font-weight:var(--weight-strong)">2</span></div>
        <div class="field"><span class="label">Warnings</span><span style="font-size:var(--text-xl);font-weight:var(--weight-strong);color:var(--warning-text)">3</span></div>
      </div>
      <div class="row"><span class="row-label" title="Skipped items are not imported">Skip items already in the vault</span><span class="switch on"></span></div>
    </div>
    <div class="section" style="padding-bottom:var(--s-2)">${sectionTitle('Warnings', 'The rest of each row is imported.')}
      <table class="table"><thead><tr><th class="num">Row</th><th>Message</th></tr></thead><tbody>
        ${w(12, 'Price “quinze euros” not understood')}${w(31, 'Renewal date “fin mars” not understood')}${w(57, 'No name: the website is used')}
      </tbody></table>
    </div>
    <div style="display:flex;justify-content:flex-end;gap:var(--s-2)"><button class="btn btn--ghost">Back</button><button class="btn btn--primary">Import 84 items</button></div>`);
}

// ---------- 16. Autofill (my account) ----------
function remplissage(scroll = 0) {
  const lk = (c, app, prog, item, sp) => `<tr><td><span class="site"><span class="site-ico">${c}</span><span class="site-name">${app}</span></span></td><td class="muted"><span class="code">${prog}</span></td><td>${item}</td><td class="muted">${sp}</td><td style="text-align:right"><button class="icon-btn icon-btn--sm" aria-label="Unlink" title="Unlink">${i('x')}</button></td></tr>`;
  const br = (n, state, act = '') => `<div class="row"><span class="row-label">${i('globe')}${n}</span>${state}${act}</div>`;
  return settings('Autofill', `
    <div class="page-head"><h1>Autofill</h1><span class="switch on" title="Autofill on this computer"></span></div>
    <div class="section">${sectionTitle('Shortcuts', 'Work in any app, even when Cleis is in the background.')}
      <div class="row"><span class="row-label">Quick Access</span>${keys('Ctrl', 'Shift', 'Space')}<button class="btn btn--ghost btn--sm">Change</button><span class="switch on"></span></div>
      <div class="row"><span class="row-label" title="Fills the item linked to the window in front, without opening Quick Access">Fill best match</span>${keys('Ctrl', 'Alt', 'A')}<button class="btn btn--ghost btn--sm">Change</button><span class="switch on"></span></div>
    </div>
    <div class="section">${sectionTitle('Filling')}
      <div class="row"><span class="row-label">Method</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="true" title="Writes into the field through the system accessibility interface; types the keys when the app does not allow it">Automatic</button><button role="radio" aria-checked="false" title="Simulates the keyboard">Type</button><button role="radio" aria-checked="false" title="Clipboard, cleared right after">Paste</button></div></div>
      <div class="row"><span class="row-label" title="When the app asks for it after the password">Then fill the 2FA code</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Press Enter after filling</span><span class="switch"></span></div>
      <div class="row"><span class="row-label" title="Slower typing for apps that miss keys">Typing speed</span><div class="input select" style="width:160px"><span class="grow">Normal</span></div></div>
    </div>
    <div class="section">${sectionTitle('Detection')}
      <div class="row"><span class="row-label" title="Reads the address of the active tab through accessibility, without the extension. May slow Chrome and Edge slightly.">Match websites in browsers</span>${inherited('Allowed')}<span class="switch on"></span></div>
      <div class="row"><span class="row-label">Ask before filling an app that is not linked</span><span class="switch on"></span></div>
    </div>
    <div class="section" style="padding-bottom:var(--s-2)">${sectionTitle('Linked apps', 'Recognized by their program. Linked from Quick Access the first time you fill.')}
      <table class="table"><thead><tr><th>App</th><th>Program</th><th>Item</th><th style="width:120px">Space</th><th style="width:40px"></th></tr></thead><tbody>
        ${lk('E', 'Example Books', 'ExampleBooks.exe', 'Example Books', 'Accounting')}
        ${lk('F', 'Figma', 'Figma.exe', 'Figma', 'Design')}
        ${lk('G', 'GitHub Desktop', 'GitHubDesktop.exe', 'GitHub', 'Dev')}
        ${lk('R', 'Remote Desktop', 'mstsc.exe', 'Office router', 'General')}
      </tbody></table>
    </div>
    <div class="section">${sectionTitle('System permissions')}
      <div class="row"><span class="row-label" title="macOS · System Settings › Privacy & Security › Accessibility. Not needed on Windows.">Accessibility</span><span class="chip chip--ok"><span class="dot"></span>Allowed</span></div>
    </div>`, '', scroll);
}
function remplissageSuite() { return remplissage(560); }

// ---------- 17. Quick Access over another app ----------
function accesRapide() {
  const it = (c, n, sub, sel = false, hint = '') => `<div class="qa-item ${sel ? 'is-sel' : ''}"><span class="site-ico">${c}</span><div class="meta"><span class="site-name">${n}</span><span class="sub">${sub}</span></div>${hint}</div>`;
  return `<div class="desk">
    <div class="win" style="left:420px;top:478px;width:600px">
      <div class="win-bar"><span class="win-dots"><i></i><i></i><i></i></span><span>Example Books — Sign in</span></div>
      <div class="win-body">
        <div style="font-size:var(--text-lg);font-weight:var(--weight-medium)">Sign in to Example Books</div>
        <div class="field"><span class="label">Email</span><div class="input"><span class="grow ph">name@company.com</span></div></div>
        <div class="field"><span class="label">Password</span><div class="input is-focus"><span class="grow"></span></div></div>
        <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary">Sign in</button></div>
      </div>
    </div>
    <div class="qa" role="dialog" aria-label="Quick Access">
      <div class="qa-search">${i('search')}<span class="grow ph">Search</span><span class="chip chip--auto" title="Detected from the window in front: ExampleBooks.exe">${i('window')}Example Books</span></div>
      <div class="qa-list">
        <div class="menu-head">Linked to this app</div>
        ${it('E', 'Example Books', 'accounting@example-studio.com · Accounting', true, `<span class="qa-hint">${keys('↵')}Fill</span>`)}
        ${it('E', 'Example Books — test', 'ines@example-studio.com · My vault')}
        <div class="menu-head">Other</div>
        ${it('M', 'Microsoft 365', 'accounting@example-studio.com · Accounting')}
        ${it('B', 'Bank — business portal', '0042 1187 · Accounting')}
      </div>
      <div class="qa-foot">
        <span class="qa-hint">${keys('Shift', '↵')}Type</span>
        <span class="qa-hint">${keys('Ctrl', 'C')}Password</span>
        <span class="qa-hint">${keys('Ctrl', 'Alt', 'C')}2FA code</span>
        <span style="flex:1"></span>
        <span class="totp" title="2FA code for Example Books"><span class="ring" style="--p:70%"></span>518 204</span>
      </div>
    </div>
  </div>`;
}

// ---------- 20. Contacts ----------
const PEOPLE = [
  { n: 'Camille Dubois', co: 'Example Print', t: 'Account manager', e: 'camille@example-print.com', ph: '+33 1 99 00 12 34', lb: ['Supplier'], city: 'Lyon', fav: true },
  { n: 'Julien Moreau', co: 'Example Legal', t: 'Lawyer', e: 'j.moreau@example-legal.com', ph: '+33 1 99 00 45 10', lb: ['Advisor'], city: 'Paris' },
  { n: 'Nadia Benali', co: 'Example Hosting', t: 'Support lead', e: 'support@example-hosting.com', ph: '+33 1 99 00 77 02', lb: ['Supplier', 'Tech'], city: 'Lille' },
  { n: 'Paul Lefèvre', co: 'Example Bakery', t: 'Owner', e: 'paul@example-bakery.com', ph: '+33 6 99 00 21 88', lb: ['Client'], city: 'Annecy', fav: true },
  { n: 'Sofia Rossi', co: 'Example Films', t: 'Producer', e: 'sofia@example-films.com', ph: '+39 02 9900 1122', lb: ['Client', 'VIP'], city: 'Milan' },
  { n: 'Marc Girard', co: 'Example Bank', t: 'Business adviser', e: 'marc.girard@example-bank.com', ph: '+33 1 99 00 63 40', lb: ['Bank'], city: 'Paris' },
  { n: 'Léa Fontaine', co: '—', t: 'Photographer', e: 'lea.fontaine@example.com', ph: '+33 6 99 00 54 17', lb: ['Freelancer'], city: 'Bordeaux' },
  { n: 'Omar Haddad', co: 'Example Studio', t: 'Accountant', e: 'omar@example-accounting.com', ph: '+33 1 99 00 30 25', lb: ['Advisor'], city: 'Paris' },
  { n: 'Emma Laurent', co: 'Example Retail', t: 'Marketing director', e: 'emma.laurent@example-retail.com', ph: '+33 1 99 00 88 61', lb: ['Prospect'], city: 'Nantes' },
  { n: 'Hugo Bernard', co: 'Example Studio', t: 'Developer', e: 'hugo@example-studio.com', ph: '+33 6 99 00 10 03', lb: ['Team'], city: 'Paris' },
  { n: 'Ines Petit', co: 'Example Studio', t: 'Office manager', e: 'ines@example-studio.com', ph: '+33 6 99 00 10 04', lb: ['Team'], city: 'Paris' },
  { n: 'Thomas Weber', co: 'Example Print', t: 'Prepress', e: 'thomas@example-print.com', ph: '+49 30 9900 4410', lb: ['Supplier'], city: 'Berlin' },
];
const LB = { Supplier: '', Client: 'chip--set', VIP: 'chip--warn', Prospect: 'chip--auto', Team: 'chip--ok', Advisor: '', Bank: '', Freelancer: '', Tech: '' };
const initials = (n) => n.split(' ').map((x) => x[0]).join('').slice(0, 2);
function contactsSidebar() {
  const it = (ico, label, count, active = false, tip = '') =>
    `<div class="side-item ${active ? 'is-active' : ''}" ${tip ? `title="${tip}"` : ''}>${i(ico)}<span>${label}</span>${count !== undefined ? `<span class="count">${count}</span>` : ''}</div>`;
  const lbl = (n, c) => `<div class="side-item"><span class="lbl-dot ${LB[n]}"></span><span>${n}</span><span class="count">${c}</span></div>`;
  return `<aside class="sidebar pane">
    ${it('users', 'All contacts', 148, true)}${it('star', 'Favorites', 12)}${it('calendar', 'Birthdays soon', 2, false, 'Next 30 days')}
    <div class="side-group"><span>Personal</span></div>
    ${it('user', 'My contacts', 37)}
    <div class="side-group"><span>Example Studio</span><button class="icon-btn icon-btn--sm" aria-label="New folder" title="New folder">${i('plus')}</button></div>
    ${it('folder', 'Clients', 42)}${it('folder', 'Suppliers', 31)}${it('folder', 'Advisors', 9)}${it('folder', 'Team', 7)}
    <div class="side-group"><span>Labels</span><button class="icon-btn icon-btn--sm" aria-label="New label" title="New label">${i('plus')}</button></div>
    ${lbl('VIP', 6)}${lbl('Prospect', 14)}${lbl('Freelancer', 11)}
    <div style="flex:1"></div>
    <div class="side-item" title="Possible duplicates">${i('users')}<span>Merge & fix</span><span class="count badge">3</span></div>
    <div class="side-item" title="vCard, CSV">${i('upload')}<span>Import</span></div>
  </aside>`;
}
function contactsTable(sel = 'Camille Dubois') {
  const rows = PEOPLE.map((c) => `<tr class="${c.n === sel ? 'is-selected' : ''}">
    <td><span class="site"><span class="avatar avatar--sm">${initials(c.n)}</span><span class="site-name">${c.n}</span>${c.fav ? `<span class="fav">${i('star')}</span>` : ''}</span></td>
    <td>${c.co}</td>
    <td class="muted"><span class="cellflex"><span class="ell">${c.e}</span>${c.n === sel ? `<span class="cell-acts"><button class="icon-btn icon-btn--sm" aria-label="Copy" title="Copy">${i('copy')}</button></span>` : ''}</span></td>
    <td class="muted num" style="text-align:left">${c.ph}</td>
    <td>${c.lb.map((l) => `<span class="chip ${LB[l]}" style="margin-right:4px">${l}</span>`).join('')}</td>
</tr>`).join('');
  return `<table class="table table--fixed"><thead><tr>
    <th style="width:210px" class="sorted">Name <span class="sort">↑</span></th><th style="width:150px">Company</th><th>Email</th><th style="width:150px">Phone</th><th style="width:160px">Labels</th>
  </tr></thead><tbody>${rows}<tr><td colspan="5" style="color:var(--text-muted);font-size:var(--text-sm);border-bottom:0">148 contacts</td></tr></tbody></table>`;
}
function contactInspector() {
  const multi = (label, val, kind, acts) => `<div class="kv-row"><span class="kv-type">${kind}</span><span class="kv-val">${val}</span><span class="act">${acts}</span></div>`;
  const cp = `<button class="icon-btn icon-btn--sm" aria-label="Copy" title="Copy">${i('copy')}</button>`;
  return `<aside class="inspector pane">
    <div class="inspector-head">
      <span class="avatar avatar--lg">CD</span>
      <div style="flex:1;min-width:0;display:flex;flex-direction:column">
        <span style="font-weight:var(--weight-medium);font-size:var(--text-lg)">Camille Dubois</span>
        <span style="color:var(--text-muted);font-size:var(--text-sm)">Account manager · Example Print</span>
      </div>
      <button class="icon-btn" aria-label="Favorite" title="Favorite" style="color:var(--warning-text)">${i('star')}</button>
      <button class="icon-btn" aria-label="More" title="Share vCard, merge, delete">${i('more')}</button>
    </div>
    <div class="quick-acts">
      <button class="btn btn--sm" title="New email">${i('mail')}Email</button>
      <button class="btn btn--sm" title="Call with the default app">${i('phone')}Call</button>
      <button class="btn btn--sm" title="Copy as text">${i('copy')}Copy</button>
    </div>
    <div class="inspector-tabs"><span class="is-active">Details</span><span>Linked</span><span>History</span></div>
    <div class="inspector-body" style="padding-top:var(--s-3)">
      <div class="field"><span class="label">Email</span>
        ${multi('Email', 'camille@example-print.com', 'Work', cp)}${multi('Email', 'camille.d@example.com', 'Personal', cp)}</div>
      <div class="field"><span class="label">Phone</span>
        ${multi('Phone', '+33 1 99 00 12 34', 'Work', cp)}${multi('Phone', '+33 6 99 00 12 35', 'Mobile', cp)}</div>
      <div class="field"><span class="label">Address</span>
        ${multi('Address', '12 rue de l’Exemple, 69002 Lyon', 'Work', `<button class="icon-btn icon-btn--sm" aria-label="Open map" title="Open in maps">${i('external')}</button>`)}</div>
      <div class="grid-2" style="gap:var(--s-3)">
        <div class="field"><span class="label">Birthday</span><div class="input"><span class="grow">Mar 12</span></div></div>
        <div class="field"><span class="label" title="Custom column">VAT number</span><div class="input"><span class="grow code">FR00 999999999</span></div></div>
      </div>
      <div class="field"><span class="label">Labels</span><div class="chips"><span class="chip">Supplier</span><button class="icon-btn icon-btn--sm" aria-label="Add label" title="Add label">${i('plus')}</button></div></div>
      <div class="field"><span class="label" title="Vault items for this contact">Linked items</span>
        <div class="linked"><span class="site-ico">E</span><span style="flex:1">Example Print — client portal</span><button class="icon-btn icon-btn--sm" aria-label="Copy password" title="Copy password">${i('key')}</button></div></div>
      <div class="field"><span class="label">Notes</span>
        <div class="input" style="height:auto;min-height:44px;align-items:flex-start;padding:6px 8px;color:var(--text-muted)">Quotes within 48 h · prefers email</div></div>
    </div>
  </aside>`;
}
function contacts() {
  return shell('Contacts', `
    <div class="work">${contactsSidebar()}
      <section class="center">
        <div class="toolbar">
          <div class="input search">${i('search')}<span class="grow ph">Search name, company, phone</span><span style="color:var(--text-faint);font-size:var(--text-sm)">Ctrl+K</span></div>
          <button class="btn btn--ghost">${i('filter')}Filter</button>
          <button class="btn btn--ghost">${i('columns')}Columns</button>
          <div style="flex:1"></div>
          <div class="segmented" role="radiogroup" aria-label="View"><button role="radio" aria-checked="true" title="Table">${i('list')}</button><button role="radio" aria-checked="false" title="Cards">${i('columns')}</button></div>
        </div>
        <div class="table-wrap">${contactsTable()}</div>
      </section>
      ${contactInspector()}
    </div>
    <footer class="statusbar"><span>Suppliers › Camille Dubois</span><span class="sp"></span><span class="sb-item" title="Contacts are encrypted like passwords">${i('shield')}Standard encryption</span><span>Saved</span></footer>`,
    { primary: `${i('plus')}New contact` });
}

// ---------- 21. Merge & fix ----------
function fusion() {
  const side = (who, rows, keep) => `<div class="merge-card ${keep ? 'is-keep' : ''}">
    <div class="member"><span class="avatar">${initials(who)}</span><div class="meta"><span style="font-weight:var(--weight-medium)">${who}</span><span class="sub">${keep ? 'Kept' : 'Merged into the other'}</span></div></div>
    <dl class="kv">${rows.map(([k, v, diff]) => `<dt>${k}</dt><dd ${diff ? 'class="diff"' : ''}>${v}</dd>`).join('')}</dl></div>`;
  return shell('Contacts', `
    <div class="work work--no-inspector">${contactsSidebar().replace('side-item is-active', 'side-item').replace('<span>Merge & fix</span>', '<span style="color:var(--text)">Merge & fix</span>').replace('title="Possible duplicates"', 'title="Possible duplicates" style="background:var(--surface-active)"')}
      <section class="center"><div class="settings-body" style="padding-top:var(--s-6)"><div class="page">
        <div class="page-head"><h1>Merge & fix</h1><span class="chip chip--auto" title="Same email, phone or name">3 possible duplicates</span><button class="btn btn--primary">Merge all</button></div>
        <div class="section">
          <div class="row" style="padding:0;border:0"><span class="row-label" style="font-weight:var(--weight-medium)">Camille Dubois</span><span class="chip" title="Same email">Same email</span><button class="btn btn--ghost btn--sm">Not a duplicate</button><button class="btn btn--sm">Merge</button></div>
          <div class="grid-2">
            ${side('Camille Dubois', [['Company', 'Example Print'], ['Email', 'camille@example-print.com'], ['Phone', '+33 1 99 00 12 34'], ['Mobile', '+33 6 99 00 12 35', true], ['Folder', 'Suppliers']], true)}
            ${side('C. Dubois', [['Company', 'Example Print'], ['Email', 'camille@example-print.com'], ['Phone', '—'], ['Birthday', 'Mar 12', true], ['Folder', 'My contacts']], false)}
          </div>
        </div>
        <div class="section"><div class="row" style="padding:0;border:0"><span class="row-label" style="font-weight:var(--weight-medium)">Nadia Benali</span><span class="chip">Same phone</span><span class="sub" style="color:var(--text-muted);font-size:var(--text-sm)">2 contacts</span><button class="btn btn--ghost btn--sm">Not a duplicate</button><button class="btn btn--sm">Review</button></div></div>
        <div class="section"><div class="row" style="padding:0;border:0"><span class="row-label" style="font-weight:var(--weight-medium)">Paul Lefèvre</span><span class="chip">Similar name</span><span class="sub" style="color:var(--text-muted);font-size:var(--text-sm)">2 contacts</span><button class="btn btn--ghost btn--sm">Not a duplicate</button><button class="btn btn--sm">Review</button></div></div>
      </div></div></section>
    </div>`, { primary: `${i('plus')}New contact` });
}

// ---------- 22. Restore my vault (new device) ----------
function restaurer() {
  const tile = (ico, label, on = false, tip = '') => `<button class="tile ${on ? 'is-active' : ''}" ${tip ? `title="${tip}"` : ''}>${i(ico)}<span>${label}</span></button>`;
  return `<div class="app"><main class="main" style="margin:6px"><div class="center-stage">
    <div class="card" style="width:520px">
      <div class="card-head"><span class="logo logo--lg">${i('key')}</span><h1>Restore my vault</h1></div>
      <div class="section">${sectionTitle('Where is it?', 'Cleis opens your encrypted vault there. The provider never sees your passwords.')}
        <div class="tiles">${tile('repo', 'GitHub', true, 'Private repository')}${tile('cloud', 'OneDrive', false, 'Microsoft account')}${tile('cloud', 'Google Drive', false, 'Google account')}${tile('server', 'SFTP', false, 'Your own server')}${tile('qr', 'Setup code', false, 'Code shown on another device or in your emergency kit')}</div>
        <div class="row plain"><span class="chip chip--ok"><span class="dot"></span>Signed in as lea-example</span><span style="flex:1"></span><button class="btn btn--ghost btn--sm">Switch account</button></div>
        <div class="field"><span class="label">Vault</span><div class="input select"><span class="site-ico" style="width:18px;height:18px">${i('repo')}</span><span class="grow code">lea-example/cleis-vault</span><span class="muted" style="font-size:var(--text-sm)">2 min ago</span></div></div>
      </div>
      <div class="section">
        <div class="field"><span class="label">Master password</span><div class="input is-focus"><span class="grow dots">••••••••••••</span><span class="act"><button class="icon-btn icon-btn--sm" aria-label="Show" title="Show">${i('eye')}</button></span></div></div>
      </div>
      <div style="display:flex;justify-content:space-between"><button class="btn btn--ghost">Back</button><button class="btn btn--primary">Restore</button></div>
    </div></div></main></div>`;
}

// ---------- 23. Sync & backups (my account) ----------
function synchro() {
  // Audit U-8: as the app is (25/11/2026): places by folder, GitHub and SFTP; no account sign-in.
  const place = (ico, name, sub, action) => `<div class="row"><span class="site-ico">${i(ico)}</span><div class="member row-label"><div class="meta"><span>${name}</span><span class="sub">${sub}</span></div></div>${action}</div>`;
  return settings('Sync & backups', `
    <div class="page-head"><h1 style="flex:none">Sync & backups</h1><div class="segmented" role="radiogroup" aria-label="Whose vault" title="Your vault, or the organization's (administrators)"><button role="radio" aria-checked="true">Mine</button><button role="radio" aria-checked="false">Example Studio</button></div><span style="flex:1"></span><button class="btn btn--primary">${i('sync')}Sync now</button></div>
    <div class="section">${sectionTitle('Sync', 'Your encrypted vault, kept the same on every device that uses this place. Your master password stays on your devices.')}
      <div class="row"><span class="site-ico">${i('repo')}</span><div class="member row-label"><div class="meta"><span>GitHub</span><span class="sub code">lea-example/cleis-vault</span></div></div><span class="chip chip--ok" title="Last sync today at 10:42"><span class="dot"></span>Up to date</span><button class="btn btn--ghost btn--sm">Disconnect</button></div>
    </div>
    <div class="section">${sectionTitle('Places', 'Where the encrypted copy is kept.')}
      ${place('cloud', 'OneDrive', 'Folder kept by the OneDrive app', '<button class="btn btn--sm">Choose folder</button>')}
      ${place('cloud', 'Google Drive', 'Folder kept by Google Drive for desktop', '<button class="btn btn--sm">Choose folder</button>')}
      ${place('folder', 'Folder', 'Network share, Dropbox, USB drive', '<button class="btn btn--sm">Choose folder</button>')}
      ${place('repo', 'GitHub', 'Private repository', '<button class="btn btn--sm">Connect</button>')}
      ${place('server', 'SFTP', 'Your own server', '<button class="btn btn--sm">Set up</button>')}
    </div>
    <div class="section">${sectionTitle('New device', 'What a new computer needs besides your master password.')}
      <div class="row"><span class="row-label" title="PDF with where your vault is and a space to write your master password. Print it and keep it safe.">${i('file')}Emergency kit</span><button class="btn btn--sm">${i('download')}Download</button></div>
    </div>
    <div class="section">${sectionTitle('Backups', 'Dated copies you can go back to.')}
      <div class="row"><span class="row-label">Keep</span><div class="input select" style="width:160px"><span class="grow">30 versions</span></div><button class="btn btn--sm">${i('upload')}Back up now</button></div>
      <div class="row"><span class="row-label">Encrypted export</span><button class="btn btn--sm">${i('download')}Export</button></div>
    </div>`);
}


// ---------- Organization dialogs and pages (added with the server, 25/11/2026) ----------
const dlg = (label, body) => `<div class="overlay"><div class="dialog" role="dialog" aria-label="${label}"><h2>${label}</h2>${body}</div></div>`;
const fld = (label, value, focus = false) => `<div class="field"><span class="label">${label}</span><div class="input ${focus ? 'is-focus' : ''}"><span class="grow">${value}</span></div></div>`;
const vaultWith = (overlay) => shell('Vault', `
    <div class="work">${sidebar()}
      <section class="center">${vaultToolbar()}<div class="table-wrap">${vaultTable()}</div></section>
      ${inspector()}
    </div>${statusbar()}`, { overlay });
function inviter() {
  return settings('Members', membres().match(/<div class="settings-body"><div class="page"[^>]*>([\s\S]*)<\/div><\/div><\/div>/)?.[1] ?? '', dlg('Invite', `
    ${fld('E-mail', 'noa@example-studio.com', true)}${fld('Name', 'Noa Lambert')}
    <div class="field"><span class="label">Role</span><div class="input select" style="width:200px"><span class="grow">Member</span></div></div>
    <div class="field"><span class="label">Spaces</span><div style="display:flex;gap:6px;flex-wrap:wrap"><span class="chip chip--set"><span class="dot"></span>General</span><span class="chip">Design</span><span class="chip">Accounting</span><span class="chip">Dev</span><span class="chip">Interns</span></div></div>
    <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Create code</button></div>`));
}
function codeInvitation() {
  return settings('Members', '', dlg('Invitation', `
    <span style="color:var(--text-muted)">Send this code with the server address. Valid 7 days, once.</span>
    <div class="input"><span class="grow code">K7Q2-9MXA-4TRE-LB2C-WQ7N-3HJD-PXYV-6FKA</span><button class="icon-btn icon-btn--sm" title="Copy">${i('copy')}</button></div>
    <div class="dialog-actions"><button class="btn btn--primary">Done</button></div>`));
}
function nouveauRole() {
  return settings('Members', '', dlg('New role', `${fld('Name', 'Contractor', true)}
    <span style="color:var(--text-muted);font-size:var(--text-sm)">Starts with Copy, fill; tick the rest in the table.</span>
    <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Create</button></div>`));
}
function nouvelEspace() {
  return vaultWith(dlg('New space', `${fld('Name', 'Marketing', true)}
    <div class="field"><span class="label">Encryption</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="true">Standard</button><button role="radio" aria-checked="false">End-to-end</button></div></div>
    <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Create</button></div>`));
}
function codeServeur() {
  return vaultWith(dlg('Example Studio · verification code', `
    <span style="color:var(--text-muted)">Required by your organization. Scan, then type the code.</span>
    <div style="display:flex;justify-content:center"><div style="width:140px;height:140px;background:#fff;border-radius:6px;display:grid;place-items:center;color:#000">${i('qr')}</div></div>
    <div class="input"><span class="grow code">JBSW Y3DP EHPK 3PXP JBSW Y3DP EHPK 3PXP</span><button class="icon-btn icon-btn--sm" title="Copy the key">${i('copy')}</button></div>
    ${fld('Code', '123 456', true)}
    <div class="dialog-actions"><button class="btn">Later</button><button class="btn btn--primary">Continue</button></div>`));
}
function serveur() {
  return settings('Server', `
    <div class="page-head"><h1>Server</h1></div>
    <div class="section">${sectionTitle('Example Studio')}
      <div class="row"><span class="row-label">Address</span><span class="code" style="color:var(--text-muted)">vault.example-studio.lan:8443</span><span class="chip chip--ok"><span class="dot"></span>Synced Today, 11:34 AM</span></div>
      <div class="row"><span class="row-label">Certificate</span><span class="code" style="color:var(--text-muted)">SHA256:4f1c·9a2e·77b0·d31f</span></div>
      <div class="row"><span class="row-label">Signed in as</span><span class="avatar">LM</span><span>Lea Martin · Admin</span></div>
      <div class="row"><span class="row-label"></span><button class="btn">${i('sync')}Sync now</button><button class="btn btn--danger-ghost" title="Forgets the organization's spaces on this vault">Leave</button></div>
    </div>`);
}
function rejoindre() {
  return settings('Organization', `
    <div class="page-head"><h1>Organization</h1></div>
    <div class="section">${sectionTitle('Join an organization', 'Shared spaces, on top of your personal vault')}
      <div class="field"><span class="label" title="Cleis server, reached through your VPN">Server address</span><div class="input"><span class="grow">vault.example-studio.lan</span><span class="chip chip--ok"><span class="dot"></span>Example Studio</span></div></div>
      ${fld('Invitation code', 'K7Q2-9MXA-4TRE-LB2C-WQ7N-3HJD-PXYV-6FKA', true)}
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary">Join</button></div>
    </div>`);
}

function superAdmin() { return membres('HB'); }
function profil() {
  return settings('Profile', `
    <div class="page-head"><h1>Profile</h1></div>
    <div class="section">${sectionTitle('Example Studio', 'Shown to the other members.')}
      <div class="row" style="gap:var(--s-3)"><span class="avatar avatar--lg">LM</span><div class="member row-label"><div class="meta"><span>Lea Martin</span><span class="sub">Super Admin</span></div></div></div>
      <div class="grid-2">${fld('Name', 'Lea Martin')}${fld('E-mail', 'lea@example-studio.com')}</div>
    </div>
    <div class="section">${sectionTitle('Account key', 'Signs you in to the server; never leaves your vault.')}
      <div class="row"><span class="row-label">Fingerprint</span><span class="code" style="color:var(--text-muted)">SHA256:9be0·41c7·e2d5·0a63</span><button class="icon-btn" title="Copy">${i('copy')}</button></div>
    </div>`);
}
function appareils() {
  const d = (ico, n, last, chip, cur = false) => `<tr><td><span class="site">${i(ico)}<span>${n}</span></span></td><td class="muted">${last}</td><td>${chip}</td><td style="text-align:right">${cur ? '' : '<button class="btn btn--ghost btn--sm">Sign out</button>'}</td></tr>`;
  return settings('Devices', `
    <div class="page-head"><h1>Devices</h1></div>
    <div class="section" style="padding:0;overflow:hidden">
      <table class="table table--roomy"><thead><tr><th>Device</th><th>Last active</th><th>Verification code</th><th style="width:120px"></th></tr></thead><tbody>
        ${d('laptop', 'MAC-LEA · macOS 15', 'Now', '<span class="chip chip--ok"><span class="dot"></span>This device</span>', true)}
        ${d('laptop', 'PC-STUDIO · Windows 11', 'Yesterday, 6:02 PM', '<span class="chip" title="Code not asked until Dec 24">Remembered</span>')}
        ${d('laptop', 'MAC-HOME · macOS 14', 'Nov 18, 2026', '<span class="chip">Asked</span>')}
      </tbody></table>
    </div>`);
}
function espaces(overlay = '') {
  const av = (xs) => `<span style="display:inline-flex">${xs.map((x, k) => `<span class="avatar avatar--sm" style="margin-left:${k ? -6 : 0}px">${x}</span>`).join('')}</span>`;
  const r = (n, items, ms, mode) => `<tr><td><span class="site">${i('folder')}<span>${n}</span></span></td><td class="num muted">${items}</td><td>${av(ms)} <span class="muted" style="margin-left:6px">${ms.length}</span></td><td>${mode === 'e' ? '<span class="chip chip--ok">End-to-end</span>' : '<span class="chip">Standard</span>'}</td>
    <td style="text-align:right"><button class="icon-btn icon-btn--sm" title="Rename">${i('text')}</button><button class="icon-btn icon-btn--sm" title="Delete">${i('trash')}</button></td></tr>`;
  return settings('Spaces', `
    <div class="page-head"><h1>Spaces</h1><button class="btn btn--primary">${i('plus')}New space</button></div>
    <div class="section" style="padding:0;overflow:hidden">
      <table class="table table--roomy"><thead><tr><th>Space</th><th class="num" style="width:80px">Items</th><th>Members</th><th style="width:140px">Encryption</th><th style="width:90px"></th></tr></thead><tbody>
        ${r('General', 41, ['LM', 'HB', 'IP', 'SR'], 's')}${r('Design', 18, ['LM', 'HB'], 's')}${r('Accounting', 12, ['LM', 'IP'], 'e')}${r('Dev', 9, ['LM', 'HB'], 's')}${r('Interns', 6, ['LM', 'TD'], 's')}
      </tbody></table>
    </div>`, overlay);
}
function renommerEspace() {
  return espaces(dlg('Rename “Design”', `${fld('Name', 'Design & brand', true)}
    <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Rename</button></div>`));
}
function supprimerEspace() {
  return espaces(dlg('Delete “Interns”?', `<span style="color:var(--text-muted)">Its 6 items are erased for every member. A backup of the organization keeps them.</span>
    <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--danger">Delete</button></div>`));
}

function nouveauDossier() {
  return vaultWith(dlg('New folder', `${fld('Name', 'Banking', true)}
    <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Create</button></div>`));
}
function supprimerDossier() {
  return vaultWith(dlg('Delete “Banking”?', `<span style="color:var(--text-muted)">Its 4 items move to My vault.</span>
    <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--danger">Delete</button></div>`));
}

function creerOrganisation() {
  return settings('Organization', `
    <div class="page-head"><h1>Organization</h1></div>
    <div class="section">${sectionTitle('Create an organization', 'Shared spaces for a team: Cleis runs its server for you.')}
      ${fld('Name', 'Example Studio', true)}
      <div class="field"><span class="label">Server</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="true" title="Runs while Cleis runs; can start with the system">This computer</button><button role="radio" aria-checked="false" title="Linux, Windows, macOS or Docker">Another machine</button></div></div>
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary">Create</button></div>
    </div>
    <div class="section">${sectionTitle('Join an organization', 'Shared spaces, on top of your personal vault')}
      <div class="field"><span class="label">Server address</span><div class="input"><span class="grow muted">vault.example-studio.lan</span></div></div>
      <div class="field"><span class="label">Invitation code</span><div class="input"><span class="grow muted">XXXX-XXXX-…</span></div></div>
      <div style="display:flex;justify-content:flex-end"><button class="btn">Join</button></div>
    </div>`);
}
function serveurHeberge() {
  return settings('Server', `
    <div class="page-head"><h1>Server</h1></div>
    <div class="section">${sectionTitle('Example Studio')}
      <div class="row"><span class="row-label">Hosted</span><span>This computer</span><span class="chip chip--ok"><span class="dot"></span>Running</span></div>
      <div class="row"><span class="row-label" title="Give one of these with each invitation code">Addresses for members</span><span class="code" style="color:var(--text-muted)">192.168.1.24:8443 · 10.8.0.3:8443</span><button class="icon-btn" title="Copy">${i('copy')}</button></div>
      <div class="row"><span class="row-label" title="Otherwise it runs while Cleis runs">Start with the system</span><button class="btn">Install</button></div>
      <div class="row"><span class="row-label">Certificate</span><span class="code" style="color:var(--text-muted)">SHA256:4f1c·9a2e·77b0·d31f</span></div>
      <div class="row"><span class="row-label"></span><button class="btn">${i('sync')}Sync now</button><button class="btn btn--danger-ghost">Leave</button></div>
    </div>`);
}

function placeGithub() {
  return settings('Sync & backups', '', dlg('GitHub', `
    ${fld('Repository', 'lea-example/cleis-vault', true)}
    <div style="display:flex;gap:12px"><div style="flex:1">${fld('File', 'vault.cleis')}</div><div style="width:140px">${fld('Branch', 'main')}</div></div>
    <div class="field" title="Fine-grained token: this repository only, Contents read and write"><span class="label">Access token</span><div class="input"><span class="grow code">github_pat_••••••••••••••</span><button class="btn btn--ghost btn--sm">Create one</button></div></div>
    <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Connect</button></div>`));
}
function placeSftp() {
  return settings('Sync & backups', '', dlg('SFTP', `
    <div style="display:flex;gap:12px"><div style="flex:1">${fld('Server', 'files.example-studio.com', true)}</div><div style="width:90px">${fld('Port', '22')}</div></div>
    <div style="display:flex;gap:12px"><div style="flex:1">${fld('User', 'lea')}</div><div style="flex:1">${fld('File', 'cleis/vault.cleis')}</div></div>
    <div class="field"><span class="label">Sign-in</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="true">Password</button><button role="radio" aria-checked="false" title="OpenSSH private key (ed25519, RSA)">Private key</button></div></div>
    <div class="input"><span class="grow">••••••••••••</span></div>
    <div class="row" title="Checked at each connection; a change stops the sync"><span class="row-label">Server key</span><span class="code muted">SHA256:pT4b…9QcE</span><span class="chip chip--ok"><span class="dot"></span>Trusted</span></div>
    <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Connect</button></div>`));
}

// ---------- Added 25/11/2026 (after the full audit) ----------
// Vault tabs (32): the personal vault in its own tab, one tab per organization.
const VTABS = ['My vault', 'Example Studio', 'Acme Partners', 'Contacts', 'Health', 'Activity', 'Settings'];
function orgSidebar() {
  const it = (ico, label, count, tip = '', active = false) =>
    `<div class="side-item ${active ? 'is-active' : ''}" ${tip ? `title="${tip}"` : ''}>${i(ico)}<span>${label}</span>${count !== undefined ? `<span class="count">${count}</span>` : ''}</div>`;
  return `<aside class="sidebar pane">
    ${it('list', 'All items', 72, '', true)}${it('star', 'Favorites', 6)}${it('clock', 'Renew soon', 2)}
    <div class="side-group"><span>Spaces</span><button class="icon-btn icon-btn--sm" aria-label="New space" title="New space">${i('plus')}</button></div>
    ${it('folder', 'General', 41)}${it('folder', 'Design', 18)}
    <div class="side-item" title="Reserved · end-to-end encrypted">${i('lock')}<span>Accounting</span><span class="count">12</span></div>
    ${it('folder', 'Dev', 9)}${it('folder', 'Interns', 6)}
    <div style="flex:1"></div>
    ${it('upload', 'Import')}${it('trash', 'Trash')}
  </aside>`;
}
function personalSidebar() {
  const it = (ico, label, count, active = false) =>
    `<div class="side-item ${active ? 'is-active' : ''}">${i(ico)}<span>${label}</span>${count !== undefined ? `<span class="count">${count}</span>` : ''}</div>`;
  return `<aside class="sidebar pane">
    ${it('list', 'All items', 14, true)}${it('star', 'Favorites', 3)}${it('clock', 'Renew soon', 1)}
    <div class="side-group"><span>Folders</span><button class="icon-btn icon-btn--sm" aria-label="New folder" title="New folder">${i('plus')}</button></div>
    ${it('folder', 'Banking', 4)}${it('folder', 'Home', 6)}${it('folder', 'Shopping', 4)}
    <div style="flex:1"></div>
    ${it('upload', 'Import')}${it('trash', 'Trash')}
  </aside>`;
}
const vaultTabsShell = (active, side, overlay = '') => shell(active, `
    <div class="work">${side}
      <section class="center">${vaultToolbar()}<div class="table-wrap">${vaultTable({ hover: 'Notion' })}</div></section>
      ${inspector()}
    </div>${statusbar()}`, { tabs: VTABS, doc: 'Lea Martin', overlay });
function ongletsOrg() { return vaultTabsShell('Example Studio', orgSidebar()); }
function ongletsPerso() {
  return vaultTabsShell('My vault', personalSidebar()).replace('General › Figma', 'Banking › Bank — business portal').replace(/Standard encryption/, 'End-to-end encryption');
}
// Settings with two organizations (32c): one group each, "Add an organization" at the end.
function reglagesDeuxOrgs() {
  const it = (ico, n, active = false) => `<div class="side-item ${active ? 'is-active' : ''}">${i(ico)}<span>${n}</span></div>`;
  const nav = `<nav class="settings-nav">
    <div class="side-group" style="padding-top:var(--s-1)">My account</div>
    ${it('cloud', 'Account')}${it('user', 'Profile')}${it('lock', 'Security')}${it('laptop', 'Devices')}${it('gear', 'Preferences', true)}${it('fill', 'Autofill')}${it('sync', 'Sync & backups')}
    <div class="side-group">Example Studio</div>
    ${it('users', 'Members')}${it('folder', 'Spaces')}${it('shield', 'Security policy')}${it('key', 'Encryption')}${it('server', 'Server')}${it('upload', 'Import & export')}
    <div class="side-group">Acme Partners</div>
    ${it('folder', 'Spaces')}${it('server', 'Server')}
    <div style="height:var(--s-2)"></div>
    <div class="side-item" title="Join or create another organization" style="color:var(--text-muted)">${i('plus')}<span>Add an organization</span></div>
  </nav>`;
  const body = preferences().match(/<div class="settings-body">([\s\S]*)<\/div><\/div><\/main>/)?.[1] ?? '';
  const vaults = `<div class="section">${sectionTitle('Vaults', 'How your vaults appear at the top of the window')}
      <div class="row"><span class="row-label" title="Off: one Vault tab, every vault in its sidebar">Personal vault in its own tab</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Personal vault name</span><div class="input is-focus" style="width:200px"><span class="grow">My vault</span></div></div>
    </div>`;
  const page = body.replace('<div class="section">', vaults + '<div class="section">');
  return shell('Settings', `<div class="settings">${nav}<div class="settings-body">${page}</div></div>`, { primary: null, tabs: VTABS, doc: 'Lea Martin' });
}
// Activity with two organizations (32d): the organization is the first filter.
function journalDeuxOrgs() {
  return journal().replace('<div class="input select" style="width:150px"><span class="grow">All members</span></div>',
    '<div class="input select" style="width:170px" title="Organization"><span class="grow">Example Studio</span></div><div class="input select" style="width:150px"><span class="grow">All members</span></div>')
    .replace(/<nav class="tabs" role="tablist">[\s\S]*?<\/nav>/, `<nav class="tabs" role="tablist">${VTABS.map((t) => `<span class="tab ${t === 'Activity' ? 'is-active' : ''}" role="tab">${t}</span>`).join('')}</nav>`);
}

// Health with two organizations (32e): the vault is the first filter, then its spaces.
function santeDeuxOrgs() {
  const menu = `<div class="menu" style="left:826px;top:102px;width:200px">${['All vaults', 'My vault', 'Example Studio', 'Acme Partners'].map((n, k) => `<div class="menu-item ${k === 2 ? 'is-hover' : ''}">${k === 0 ? '<span style="width:16px"></span>' : i(k === 1 ? 'user' : 'folder')}<span>${n}</span>${k === 2 ? `<span class="kbd">${i('check')}</span>` : ''}</div>`).join('')}</div>`;
  return sante()
    .replace('<div class="input select" style="width:170px"><span class="grow">All spaces</span></div>',
      '<div class="input select is-focus" style="width:170px" title="Vault"><span class="grow">Example Studio</span></div><div class="input select" style="width:170px"><span class="grow">All spaces</span></div>')
    .replace(/<nav class="tabs" role="tablist">[\s\S]*?<\/nav>/, `<nav class="tabs" role="tablist">${VTABS.map((t) => `<span class="tab ${t === 'Health' ? 'is-active' : ''}" role="tab">${t}</span>`).join('')}</nav>`)
    .replace('</div></div></main>', `</div></div>${menu}</main>`);
}

// Context menus (33): right-click on any object of a list.
const ctx = (x, y, items, w = 240) => `<div class="menu" style="left:${x}px;top:${y}px;width:${w}px">${items}</div>`;
const mi = (ico, label, kbd = '', { danger = false, sub = false, hover = false } = {}) =>
  `<div class="menu-item ${hover ? 'is-hover' : ''}" ${danger ? 'style="color:var(--danger-text)"' : ''}>${ico ? i(ico) : '<span style="width:16px"></span>'}<span>${label}</span>${kbd ? `<span class="kbd">${kbd}</span>` : ''}${sub ? `<span class="kbd" style="display:inline-grid;transform:rotate(-90deg)">${i('chevron')}</span>` : ''}</div>`;
const sep = '<div class="menu-sep"></div>';
function menuElement() {
  const m = ctx(520, 236, `${mi('copy', 'Copy username', 'Ctrl+C')}${mi('key', 'Copy password', 'Ctrl+Shift+C')}${mi('totp', 'Copy 2FA code', 'Ctrl+Alt+C')}${mi('external', 'Open website')}${sep}${mi('star', 'Add to favorites')}${mi('copy', 'Duplicate', 'Ctrl+D')}${mi('folder', 'Move to', '', { sub: true, hover: true })}${sep}${mi('trash', 'Move to trash', 'Del', { danger: true })}`)
    + ctx(764, 418, `<div class="menu-head">Example Studio</div>${mi('folder', 'General')}${mi('folder', 'Dev')}${mi('folder', 'Interns')}${sep}<div class="menu-item" style="color:var(--text-faint)" title="An organization's items stay in the organization">${i('user')}<span>My vault</span></div>`, 200);
  return shell('Vault', `
    <div class="work">${sidebar()}
      <section class="center" style="position:relative">${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: 'Figma', hover: 'Google Workspace' })}</div>${m}</section>
      ${inspector()}
    </div>${statusbar()}`);
}
function menuEspace() {
  const m = ctx(150, 360, `${mi('plus', 'New item here')}${mi('text', 'Rename')}${mi('users', 'Members…')}${mi('shield', 'Encryption…')}${sep}${mi('trash', 'Delete space', '', { danger: true })}`, 220);
  return shell('Vault', `
    <div class="work">${sidebar('Design')}
      <section class="center">${vaultToolbar()}<div class="table-wrap">${vaultTable()}</div></section>
      ${inspector()}
    </div>${statusbar()}${m}`);
}
function menuContact() {
  const m = ctx(470, 196, `${mi('mail', 'Send e-mail')}${mi('phone', 'Call')}${mi('copy', 'Copy', '', { sub: true })}${sep}${mi('star', 'Add to favorites')}${mi('download', 'Export vCard')}${mi('folder', 'Move to', '', { sub: true })}${sep}${mi('trash', 'Delete', 'Del', { danger: true })}`, 230);
  return contacts().replace('</main>', `${m}</main>`);
}
function menuMembre() {
  const m = ctx(720, 250, `${mi('users', 'Role', '', { sub: true })}${mi('folder', 'Spaces', '', { sub: true })}${mi('refresh', 'Reset two-step verification')}${mi('logout', 'Sign out everywhere')}${sep}${mi('shield', 'Make Super Admin')}${mi('refresh', 'Transfer Super Admin…')}${sep}${mi('trash', 'Remove…', '', { danger: true })}`, 250);
  return membres().replace('</main>', `${m}</main>`).replace('>2FA<', '>2-step<');
}

// Admin delegations (07f): the Admin column, set by a Super Admin.
function rolesDelegation() {
  const c = (on, fixed, tip = '') => `<td style="text-align:center" ${tip ? `title="${tip}"` : ''}>${on ? `<span class="check on" style="display:inline-grid;${fixed ? 'opacity:.45' : ''}">${i('check')}</span>` : `<span class="check" style="display:inline-grid;${fixed ? 'opacity:.45' : ''}"></span>`}</td>`;
  const row = (n, tip, cells) => `<tr><td ${tip ? `title="${tip}"` : ''}>${n}${tip ? ' <i class="info" style="display:inline-grid">i</i>' : ''}</td>${cells.join('')}</tr>`;
  const F = (v) => c(v, true), E = (v, tip) => c(v, false, tip);
  const body = membres().match(/<div class="settings-body"><div class="page"[^>]*>([\s\S]*)<\/div><\/div><\/div>/)?.[1] ?? '';
  const roles = `<div class="section">${sectionTitle('Roles', 'What each role can do. Only a Super Admin changes the Admin column.')}
      <table class="table" style="margin-top:-4px"><thead><tr><th></th><th style="text-align:center">Super Admin</th><th style="text-align:center;color:var(--set-text)" title="Set by a Super Admin">Admin</th><th style="text-align:center">Member</th><th style="text-align:center">Read only</th><th style="text-align:center">Intern</th></tr></thead><tbody>
        ${row('See passwords', '', [F(1), F(1), E(1), E(1), E(0)])}
        ${row('Copy, fill', '', [F(1), F(1), E(1), E(1), E(1)])}
        ${row('Add, edit', '', [F(1), F(1), E(1), E(0), E(0)])}
        ${row('Delete', '', [F(1), F(1), E(1), E(0), E(0)])}
        ${row('Manage members and spaces', '', [F(1), F(1), F(0), F(0), F(0)])}
        ${row('Activity log, health report', 'Read, verify and export the activity log', [F(1), E(1, 'Admins read the activity log'), F(0), F(0), F(0)])}
        ${row('Server backups', 'Download the server file: it also holds the server key and the members’ authenticator keys', [F(1), E(0, 'Admins download server backups'), F(0), F(0), F(0)])}
        ${row('Recovery and server keys', 'Recovery key, rotate the server key', [F(1), E(0), F(0), F(0), F(0)])}
        ${row('Sign-in and log rules', 'Verification code, methods, remembered devices, what the log records and keeps', [F(1), E(0), F(0), F(0), F(0)])}
        ${row('Admins, roles', '', [F(1), F(0), F(0), F(0), F(0)])}
      </tbody></table>
      <div><button class="btn btn--ghost btn--sm">${i('plus')}New role</button></div>
    </div>`;
  const page = body.replace(/<div class="section">\$\{sectionTitle\('Roles'[\s\S]*$/, '').replace(/<div class="section"><div class="section-title"><h2>Roles<\/h2>[\s\S]*$/, '') + roles;
  return settings('Members', page.replace('>2FA<', '>2-step<'));
}
// Security policy seen by an Admin without "Sign-in and log rules" (08b).
function politiqueAdmin() {
  return politique().replace(/<div class="section"><div class="section-title"><h2>Signing in to the server<\/h2>/, '<div class="section" title="Set by a Super Admin"><div class="section-title"><h2>Signing in to the server</h2><span class="chip chip--warn" style="margin-left:var(--s-2)">' + i('lock') + 'Super Admin</span>')
    .replace('<div class="section"><div class="section-title"><h2>Allowed methods</h2>', '<div class="section" style="opacity:.55" title="Set by a Super Admin"><div class="section-title"><h2>Allowed methods</h2>');
}
// Confirmations (34): actions that cannot be undone.
const membersBody = () => membres().match(/<div class="settings-body"><div class="page"[^>]*>([\s\S]*)<\/div><\/div><\/div>/)?.[1] ?? '';
function confirmTransfert() {
  return settings('Members', membersBody(), dlg('Transfer Super Admin to Hugo Bernard?', `<span style="color:var(--text-muted)">You become Admin.</span>
    <div class="dialog-actions"><button class="btn" style="outline:2px solid var(--focus);outline-offset:1px">Cancel</button><button class="btn btn--danger">Transfer</button></div>`));
}
function confirmRetrait() {
  return settings('Members', membersBody(), dlg('Remove Sarah Roux?', `<span style="color:var(--text-muted)" title="Her devices lose the organization's spaces at their next sync">Her access ends now. Re-encrypt her end-to-end spaces after.</span>
    <div class="dialog-actions"><button class="btn" style="outline:2px solid var(--focus);outline-offset:1px">Cancel</button><button class="btn btn--danger">Remove</button></div>`));
}


// ---------- 35. Lot B (26/11/2026): void menus, drag and drop, columns, customize, calendar, unlocking ----------
// The current sidebar: shortcuts, the personal section and its folders, the organization, Import and Trash.
function sidebarB({ active = 'All items', drop = null, line = null, edit = false, hidden = [] } = {}) {
  const grip = edit ? `<span class="grip-h" title="Drag to reorder">${i('grip')}</span>` : '';
  const rm = (on) => (edit && on ? `<button class="rm-badge" aria-label="Delete" title="Delete">${i('x')}</button>` : '');
  const eye = (on) => (edit ? `<button class="icon-btn icon-btn--sm" style="margin-left:auto" title="${on ? 'Hide' : 'Show'}">${i('eye')}</button>` : '');
  const it = (ico, label, count, { folder = false, cls = '' } = {}) => {
    const off = hidden.includes(label);
    return `<div class="side-item ${label === active ? 'is-active' : ''} ${label === drop ? 'is-drop' : ''} ${edit && folder ? 'wiggle' : ''} ${off ? 'is-off' : ''} ${cls}">${folder ? grip : ''}${i(ico)}<span>${label}</span>${edit && !folder ? eye(!off) : count !== undefined ? `<span class="count">${count}</span>` : ''}${rm(folder)}</div>${line === label ? '<div class="drop-line"></div>' : ''}`;
  };
  // A vault is a section: clicking its name lists all of it, its folders below.
  const section = (ico, label, count, add) => `<div class="side-item side-section ${label === active ? 'is-active' : ''}">${i(ico)}<span>${label}</span><span class="count">${count}</span><button class="icon-btn icon-btn--sm" aria-label="${add}" title="${add}">${i('plus')}</button></div>`;
  return `<aside class="sidebar pane ${edit ? 'is-editing' : ''}">
    ${it('list', 'All items', 15)}${it('star', 'Favorites', 3)}${it('clock', 'Renew soon', 1)}
    <div style="height:var(--s-3)"></div>
    ${section('user', 'My vault', 3, 'New folder')}
    ${it('folder', 'Banking', 1, { folder: true, cls: 'sub' })}${it('folder', 'Shopping', 1, { folder: true, cls: 'sub' })}
    <div style="height:var(--s-2)"></div>
    ${section('users', 'Example Studio', 12, 'New space')}
    ${it('folder', 'General', 4, { folder: true, cls: 'sub' })}${it('folder', 'Design', 3, { folder: true, cls: 'sub' })}${it('lock', 'Accounting', 2, { folder: true, cls: 'sub' })}${it('folder', 'Dev', 3, { folder: true, cls: 'sub' })}
    <div style="flex:1"></div>
    <div class="side-item">${i('upload')}<span>Import</span></div>
    <div class="side-row"><div class="side-item" style="flex:1">${i('trash')}<span>Trash</span></div><button class="icon-btn icon-btn--sm ${edit ? 'is-on' : ''}" aria-label="Customize" title="Customize: reorder and delete">${i('grip')}</button></div>
  </aside>`;
}
const frameB = (center, { inspector: insp = false, overlay = '', side = sidebarB() } = {}) => shell('Vault', `
    <div class="work ${insp ? '' : 'work--no-inspector'}">${side}
      <section class="center" style="position:relative">${center}</section>
      ${insp ? inspector() : ''}
    </div>${statusbar()}`, { overlay });

// 35. Right-click in the void of the list (no item selected: the panel on the right is closed).
function menuVideListe() {
  const m = ctx(560, 560, `${mi('plus', 'New item', 'Ctrl+N')}${mi('folder', 'New folder')}${sep}${mi('list', 'Sort by', '', { sub: true, hover: true })}${mi('columns', 'Columns…')}${mi('eye', 'Show as cards')}${sep}${mi('upload', 'Import…')}`, 220)
    + ctx(784, 642, `${mi('check', 'Name')}${mi('', 'Last edited')}${mi('', 'Price')}${mi('', 'Renewal')}${sep}${mi('grip', 'My order', '', {})}`, 190);
  return frameB(`${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div>${m}`);
}

// 35b. Right-click in the void of the sidebar.
function menuVideLaterale() {
  const opt = (label, on) => `<div class="menu-item"><span class="check ${on ? 'on' : ''}">${on ? i('check') : ''}</span><span>${label}</span></div>`;
  const m = ctx(150, 520, `${mi('folder', 'New folder')}${sep}<div class="menu-head" title="For this vault; Preferences › Vaults makes it the same everywhere">Shortcuts</div>${opt('All items', true)}${opt('Favorites', true)}${opt('Renew soon', false)}${sep}${mi('grip', 'Customize')}`, 220);
  return frameB(`${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div>`, { overlay: m });
}

// 35c. Drag an item onto a folder.
function glisserVersDossier() {
  const ghost = `<div class="drag-ghost" style="left:132px;top:214px"><span class="site-ico">F</span>Figma</div>`;
  return frameB(`${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: 'Figma' }).replace('<tr class="is-selected', '<tr class="is-dragging is-selected')}</div>`,
    { side: sidebarB({ drop: 'Banking' }), overlay: ghost });
}

// 35d. Reorder: folders in the sidebar, items in "My order".
function reorganiser() {
  const ghost = `<div class="drag-ghost" style="left:70px;top:318px">${i('folder')}Dev</div>`;
  return frameB(`${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null }).replace('<th style="width:190px" class="sorted">Name <span class="sort">↑</span></th>', '<th style="width:190px">Name</th>')}</div>`,
    { side: sidebarB({ line: 'General' }), overlay: ghost });
}

// 35e. Column width: the edge between two headings.
function largeurColonnes() {
  const t = vaultTable({ selected: null }).replace('<th>Username</th>', '<th class="is-resizing">Username<span class="col-grip"></span></th>');
  const tip = `<div class="tooltip" style="left:640px;top:126px">Drag to resize · double-click to fit</div>`;
  return frameB(`${vaultToolbar()}<div class="table-wrap">${t}</div>${tip}`);
}

// 35f. Customize: handles to reorder, quick delete, shortcuts shown or hidden.
function personnaliser() {
  const t = vaultTable({ selected: null })
    .replace(/<td><span class="site">/g, `<td><span class="site"><span class="grip-h">${i('grip')}</span>`)
    .replace(/<tr class=" "/g, '<tr class="wiggle-row"');
  const bar = `<div class="edit-bar"><span style="color:var(--text-muted)" title="Drag the handles; the cross moves to the trash">${i('grip')}</span><span class="sp"></span><button class="btn btn--primary btn--sm">Done</button></div>`;
  return frameB(`${bar}<div class="table-wrap">${t.replace(/<span class="site-name">/g, '<button class="rm-badge rm-badge--row" aria-label="Move to trash" title="Move to trash">' + i('x') + '</button><span class="site-name">')}</div>`,
    { side: sidebarB({ edit: true, hidden: ['Renew soon'] }) });
}

// 35g. Dates: type it, or pick it in the calendar.
function calendrier() {
  const days = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'].map((d) => `<span class="cal-dow">${d}</span>`).join('');
  let cells = '';
  for (let k = 0; k < 1; k++) cells += '<span class="cal-day is-out"></span>';
  for (let d = 1; d <= 31; d++) cells += `<span class="cal-day ${d === 3 ? 'is-on' : ''}">${d}</span>`;
  const pop = `<div class="cal" style="left:1170px;top:286px">
    <div class="cal-head"><button class="icon-btn icon-btn--sm" aria-label="Previous month" title="Previous month" style="transform:rotate(90deg)">${i('chevron')}</button><span class="sp" style="text-align:center;font-weight:var(--weight-medium)">December 2026</span><button class="icon-btn icon-btn--sm" aria-label="Next month" title="Next month" style="transform:rotate(-90deg)">${i('chevron')}</button></div>
    <div class="cal-grid">${days}${cells}</div>
    <div class="cal-foot"><button class="btn btn--ghost btn--sm" title="One month from today">+1 month</button><button class="btn btn--ghost btn--sm" title="One year from today">+1 year</button><span class="sp"></span><button class="btn btn--ghost btn--sm">Clear</button></div>
  </div>`;
  const insp = inspector().replace(/<div class="inspector-body"[\s\S]*<\/aside>/, `<div class="inspector-body" style="padding-top:var(--s-3)">
      <div class="sub-box">
        <div class="kv"><span>Price</span><div class="input input--sm"><span class="grow">€15.00 / month</span></div></div>
        <div class="kv"><span>Licenses</span><div class="input input--sm"><span class="grow">4</span></div></div>
        <div class="kv"><span>Yearly cost</span><b style="margin-left:auto">€720.00</b></div>
        <div class="kv"><span>Renewal</span><div class="input input--sm is-focus"><span class="grow">2026-12-03</span><span class="act" title="Pick a date">${i('calendar')}</span></div></div>
      </div></div></aside>`);
  return shell('Vault', `<div class="work">${sidebarB({ active: 'Design' })}<section class="center">${vaultToolbar()}<div class="table-wrap">${vaultTable()}</div></section>${insp}</div>${statusbar()}`, { overlay: pop });
}

// 35h. Opening Cleis: master password, Windows Hello, or nothing on this computer.
function ouverture() {
  return settings('Security', `
    <div class="page-head"><h1>Security</h1></div>
    <div class="section">${sectionTitle('Opening Cleis', 'How this computer opens your vault. On another computer, or after reinstalling Windows: the master password or the recovery key.')}
      <div class="row"><span class="row-label">Unlock with</span>
        <div class="segmented" role="radiogroup"><button role="radio" aria-checked="false">${i('key')}Master password</button><button role="radio" aria-checked="true" title="Face, fingerprint or PIN of this Windows account">${i('fingerprint')}Windows Hello</button><button role="radio" aria-checked="false" title="Anyone using this Windows account opens Cleis">${i('laptop')}Nothing</button></div></div>
      <div class="row"><span class="row-label" title="Opens your vault anywhere; keep it outside Cleis">Recovery key</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Saved Nov 26, 2026</span><button class="btn">Show</button></div>
      <div class="row"><span class="row-label">Master password</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Changed Oct 2, 2026</span><button class="btn">Change</button><button class="btn btn--ghost" title="The recovery key then opens your vault on another computer">Remove</button></div>
    </div>
    <div class="section">${sectionTitle('Example Studio', 'Set by the organization')}
      <div class="row"><span class="row-label" style="color:var(--inherit-text)">Unlock with</span>${inherited('Master password or Windows Hello')}</div>
    </div>`);
}

// 35i. First launch: how this computer opens the vault.
function premierLancementProtection() {
  return `<div class="app"><main class="main" style="margin:6px"><div class="center-stage">
    <div class="card" style="width:460px">
      <div class="card-head"><span class="logo logo--lg">${i('key')}</span><h1>Welcome to Cleis</h1></div>
      <div class="choice is-active"><span class="radio on"></span><div class="txt"><b style="font-weight:var(--weight-medium)">This computer only</b><span>Personal vault</span></div><span class="ico">${i('laptop')}</span></div>
      <div class="choice"><span class="radio"></span><div class="txt"><b style="font-weight:var(--weight-medium)">Join an organization</b><span>Shared spaces, on top of your personal vault</span></div><span class="ico">${i('server')}</span></div>
      <div class="section" style="gap:var(--s-3)">
        <div class="row" style="padding:0"><span class="row-label">Unlock Cleis with</span>
          <div class="segmented" role="radiogroup"><button role="radio" aria-checked="false">Password</button><button role="radio" aria-checked="true">Windows Hello</button><button role="radio" aria-checked="false" title="Anyone using this Windows account opens Cleis">Nothing</button></div></div>
      </div>
      <div style="display:flex;justify-content:space-between;align-items:center">
        <button class="btn btn--ghost">${i('sync')}Restore my vault</button>
        <button class="btn btn--primary" title="A recovery key is shown next: it opens the vault on another computer">Create vault</button>
      </div>
    </div></div></main></div>`;
}

// 35j. Preferences › Vaults: the shortcuts of each vault, or the same everywhere.
function raccourcisCoffres() {
  const tg = (label, on) => `<button class="tag-toggle ${on ? 'on' : ''}">${on ? i('check') : ''}${label}</button>`;
  return settings('Preferences', `
    <div class="page-head"><h1>Preferences</h1></div>
    <div class="section">${sectionTitle('Vaults')}
      <div class="row"><span class="row-label">Personal vault in its own tab</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label" title="All items, Favorites, Renew soon at the top of the sidebar">Shortcuts</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Same for every vault</span><span class="switch"></span></div>
      <div class="row"><span class="row-label" style="padding-left:var(--s-4)">${i('user')}My vault</span>${tg('All items', true)}${tg('Favorites', true)}${tg('Renew soon', false)}</div>
      <div class="row"><span class="row-label" style="padding-left:var(--s-4)">${i('users')}Example Studio</span>${tg('All items', true)}${tg('Favorites', false)}${tg('Renew soon', true)}</div>
    </div>`);
}

// 35k. Security policy: how members may open Cleis.
function politiqueOuverture() {
  return settings('Security policy', `
    <div class="page-head"><h1>Security policy</h1></div>
    <div class="section">${sectionTitle('Opening Cleis', 'Applies to every member whose vault holds Example Studio')}
      <div class="row"><span class="row-label">Unlock with</span><div class="input select" style="width:280px"><span class="grow">Master password or Windows Hello</span></div></div>
    </div>`, `<div class="menu" style="left:906px;top:236px;width:280px">${mi('key', 'Master password only')}${mi('check', 'Master password or Windows Hello')}${mi('', 'Anything, also nothing')}</div>`);
}


// ---------- 36. Version 0.2.2 (26/11/2026): colour wheel, account, contact photo, organization storage, PIN ----------
// 36. A colour of one's own: "+" after the swatches opens the wheel.
function roueCouleur() {
  const sw = (c, on = false, label) => `<button class="swatch ${on ? 'on' : ''}" style="${c ? `background:${c}` : ''}" aria-label="${label}" title="${label}">${c ? '' : i('x')}</button>`;
  const wheel = `<div class="cal" style="left:930px;top:318px;width:248px">
    <div style="position:relative;width:216px;height:216px;margin:4px auto;border-radius:50%;background:conic-gradient(red,#ff0,lime,cyan,blue,#f0f,red)">
      <div style="position:absolute;inset:0;border-radius:50%;background:radial-gradient(circle,#fff 0,rgba(255,255,255,0) 70%)"></div>
      <span style="position:absolute;left:150px;top:56px;width:14px;height:14px;border-radius:50%;border:2px solid #fff;box-shadow:0 0 0 1px rgb(0 0 0 / .5)"></span>
    </div>
    <div title="Brightness" style="height:10px;border-radius:5px;background:linear-gradient(90deg,#000,#e06b9a)"></div>
    <div style="display:flex;gap:8px;align-items:center"><span class="swatch on" style="background:#e06b9a"></span><div class="input input--sm" style="flex:1"><span class="grow code">#E06B9A</span></div><button class="btn btn--primary btn--sm">Use</button></div>
  </div>`;
  return settings('Preferences', `
    <div class="page-head"><h1>Preferences</h1></div>
    <div class="section">${sectionTitle('Appearance')}
      <div class="row"><span class="row-label">Theme</span><div class="segmented"><button role="radio" aria-checked="true">Dark</button><button role="radio" aria-checked="false">Light</button><button role="radio" aria-checked="false">System</button></div></div>
      <div class="row"><span class="row-label">Logo</span><div class="segmented"><button role="radio" aria-checked="true">Light</button><button role="radio" aria-checked="false">Dark</button></div></div>
      <div class="row"><span class="row-label">Accent color</span><div class="swatches">${sw('', false, 'None')}${sw('var(--accent)', false, 'Ochre (default)')}${sw('#3f5a75', false, 'Blue')}${sw('#494a6a', false, 'Violet')}${sw('#48664f', false, 'Green')}${sw('#3f6766', false, 'Teal')}${sw('#7a4646', false, 'Red')}<button class="swatch on" style="background:conic-gradient(red,#ff0,lime,cyan,blue,#f0f,red)" title="Your own color"></button></div></div>
      <div class="row"><span class="row-label">Accent scope</span><div class="segmented"><button role="radio" aria-checked="false">Minimal</button><button role="radio" aria-checked="false">Standard</button><button role="radio" aria-checked="true">Extended</button></div></div>
    </div>`, wheel);
}

// 36b. Account: sign in once, everything follows to the other computers (encrypted before it leaves).
function compte() {
  const prov = (ico, name, sub, on = false) => `<div class="choice ${on ? 'is-active' : ''}" style="padding:var(--s-3)"><span class="radio ${on ? 'on' : ''}"></span><div class="txt"><b style="font-weight:var(--weight-medium)">${name}</b><span>${sub}</span></div><span class="ico">${i(ico)}</span></div>`;
  return settings('Account', `
    <div class="page-head"><h1>Account</h1></div>
    <div class="section">${sectionTitle('Sign in', 'Your vault, settings, backups and organizations follow you to your other computers. Everything is encrypted on this computer before it leaves: the provider sees only sealed files.')}
      ${prov('laptop', 'Local', 'This computer only', true)}
      ${prov('window', 'Microsoft', 'Personal, work or school account · OneDrive')}
      ${prov('globe', 'Google', 'Google Drive')}
      ${prov('repo', 'GitHub', 'A private repository')}
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary">Sign in</button></div>
    </div>`);
}
// 36c. Signed in: what follows, and the computers.
function compteConnecte() {
  const sync = (label, on, tip) => `<div class="row"><span class="row-label" ${tip ? `title="${tip}"` : ''}>${label}</span><span class="switch ${on ? 'on' : ''}"></span></div>`;
  return settings('Account', `
    <div class="page-head"><h1>Account</h1></div>
    <div class="section">${sectionTitle('Microsoft')}
      <div class="member"><span class="avatar" style="width:32px;height:32px">LM</span><div class="meta"><span style="font-weight:var(--weight-medium)">Lea Martin</span><span class="sub">lea@example-studio.com</span></div><span class="chip chip--ok" style="margin-left:auto" title="OneDrive › Apps › Cleis"><span class="dot"></span>Synced 2 min ago</span><button class="btn btn--ghost btn--sm">Sign out</button></div>
    </div>
    <div class="section">${sectionTitle('What follows you')}
      ${sync('Vault', true, 'Items, folders, contacts')}${sync('Settings', true, 'Preferences, columns, shortcuts')}${sync('Backups', true, 'The dated copies')}${sync('Organizations', true, 'Your memberships; each server still asks for your PIN')}
    </div>
    <div class="section">${sectionTitle('Computers')}
      <div class="row"><span class="row-label">${i('laptop')}MATV-PC</span><span style="color:var(--text-muted);font-size:var(--text-sm)">This computer</span></div>
      <div class="row"><span class="row-label">${i('laptop')}MATV-LAPTOP</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Today 09:12</span><button class="btn btn--ghost btn--sm" title="It stops following; its copy stays sealed">Remove</button></div>
    </div>`);
}
// 36d. A new computer: sign in, then the master password or the recovery key opens what came.
function nouvelOrdinateur() {
  return `<div class="app"><main class="main" style="margin:6px"><div class="center-stage">
    <div class="card" style="width:460px">
      <div class="card-head"><span class="logo logo--lg">${i('key')}</span><h1>Welcome to Cleis</h1></div>
      <div class="choice"><span class="radio"></span><div class="txt"><b style="font-weight:var(--weight-medium)">This computer only</b><span>Personal vault</span></div><span class="ico">${i('laptop')}</span></div>
      <div class="choice is-active"><span class="radio on"></span><div class="txt"><b style="font-weight:var(--weight-medium)">Sign in</b><span>My vault from my other computers</span>
        <div style="display:flex;gap:8px;margin-top:var(--s-3)"><button class="btn btn--sm">${i('window')}Microsoft</button><button class="btn btn--sm">${i('globe')}Google</button><button class="btn btn--sm">${i('repo')}GitHub</button></div>
      </div><span class="ico">${i('user')}</span></div>
      <div class="choice"><span class="radio"></span><div class="txt"><b style="font-weight:var(--weight-medium)">Join an organization</b><span>Shared spaces, on top of your personal vault</span></div><span class="ico">${i('server')}</span></div>
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary" title="Then the master password or the recovery key opens it">Continue</button></div>
    </div></div></main></div>`;
}

// 36e. A contact's photo: hover the avatar, or its menu.
function photoContact() {
  const insp = contactInspector().replace('<span class="avatar avatar--lg">CD</span>',
    `<span class="avatar avatar--lg" style="position:relative;overflow:hidden" title="Change photo">CD<span style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center;background:rgb(0 0 0 / .55);color:#fff">${i('upload')}</span></span>`);
  const m = ctx(1186, 96, `${mi('upload', 'Choose a photo…')}${mi('x', 'Remove photo')}`, 200);
  return shell('Contacts', `<div class="work">${contactsSidebar()}<section class="center"><div class="table-wrap">${contactsTable()}</div></section>${insp}</div>${statusbar()}`, { primary: `${i('plus')}New contact`, overlay: m });
}

// 36f. Creating an organization: where its data lives (D-78).
function stockageOrganisation() {
  const ch = (ico, name, sub, on, extra = '') => `<div class="choice ${on ? 'is-active' : ''}"><span class="radio ${on ? 'on' : ''}"></span><div class="txt"><b style="font-weight:var(--weight-medium)">${name}</b><span>${sub}</span>${extra}</div><span class="ico">${i(ico)}</span></div>`;
  const nas = `<div style="display:flex;gap:12px;margin-top:var(--s-3)"><div style="flex:1">${fld('NAS address', 'nas.example-studio.lan', true)}</div><div style="width:90px">${fld('SSH port', '22')}</div></div>
    <div style="display:flex;gap:12px;margin-top:var(--s-2)"><div style="flex:1">${fld('Administrator', 'admin')}</div><div style="flex:1"><div class="field"><span class="label">Password</span><div class="input"><span class="grow dots">••••••••</span></div></div></div></div>
    <div class="row" style="padding:var(--s-2) 0 0" title="Docker (Synology Container Manager, QNAP Container Station) or a Linux service"><span class="row-label" style="color:var(--text-muted)">Found</span><span class="chip chip--ok"><span class="dot"></span>Synology · Container Manager</span></div>`;
  return settings('Organization', `
    <div class="page-head"><h1>Organization</h1></div>
    <div class="section">${sectionTitle('Create an organization')}
      ${fld('Name', 'Example Studio')}
      ${ch('server', 'Server on the NAS', 'Cleis installs it there · immediate access changes · reachable through your VPN', true, nas)}
      ${ch('laptop', 'Server on this computer', 'Only while this computer is on')}
      ${ch('folder', 'Shared folder', 'SharePoint, Google shared drive, NAS share or SFTP · no server · access changes at the next sync')}
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary">Install and create</button></div>
    </div>`);
}
// 36i. Creating an organization in a shared folder (26/11/2026, user's note: SharePoint, SFTP and
// Google's equivalent too): where the shared folder is.
function dossierOrganisation() {
  const ch = (ico, name, sub, on, extra = '') => `<div class="choice ${on ? 'is-active' : ''}"><span class="radio ${on ? 'on' : ''}"></span><div class="txt"><b style="font-weight:var(--weight-medium)">${name}</b><span>${sub}</span>${extra}</div><span class="ico">${i(ico)}</span></div>`;
  const place = (ico, name, sub, on, action) => `<div class="row" style="padding:var(--s-2) 0;${on ? '' : ''}"><span class="radio ${on ? 'on' : ''}"></span><span class="site-ico">${i(ico)}</span><div class="member row-label"><div class="meta"><span>${name}</span><span class="sub ${on ? 'code' : ''}">${sub}</span></div></div>${action}</div>`;
  const places = `<div style="margin-top:var(--s-3);border:1px solid var(--line-strong);border-radius:var(--radius-md);padding:0 var(--s-3);background:var(--bg)">
      ${place('cloud', 'SharePoint', 'C:\\Users\\lea\\Example Studio\\Cleis', true, '<button class="btn btn--sm">Change</button>')}
      ${place('cloud', 'Google shared drive', 'Folder kept by Google Drive for desktop', false, '<button class="btn btn--sm">Choose folder</button>')}
      ${place('folder', 'NAS share', 'Network folder (\\\\nas\\cleis)', false, '<button class="btn btn--sm">Choose folder</button>')}
      ${place('server', 'SFTP', 'An SSH server', false, '<button class="btn btn--sm">Set up</button>')}
    </div>`;
  return settings('Organization', `
    <div class="page-head"><h1>Organization</h1></div>
    <div class="section">${sectionTitle('Create an organization')}
      ${fld('Name', 'Example Studio')}
      ${ch('server', 'Server on the NAS', 'Cleis installs it there · immediate access changes · reachable through your VPN', false)}
      ${ch('laptop', 'Server on this computer', 'Only while this computer is on')}
      ${ch('folder', 'Shared folder', 'No server · access changes at the next sync', true, places)}
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary">Create</button></div>
    </div>`);
}
// 36g. Security policy: the organization's PIN.
function politiquePin() {
  return settings('Security policy', `
    <div class="page-head"><h1>Security policy</h1></div>
    <div class="section">${sectionTitle('Organization PIN', 'Asked before the organization’s spaces open, on top of the vault’s own unlock')}
      <div class="row"><span class="row-label">Ask a PIN</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Length</span><div class="segmented"><button role="radio" aria-checked="false">4</button><button role="radio" aria-checked="true">6</button><button role="radio" aria-checked="false">8</button></div></div>
      <div class="row"><span class="row-label" title="Then the PIN is asked again">Ask again after</span><div class="input select" style="width:160px"><span class="grow">8 hours</span></div></div>
      <div class="row"><span class="row-label" title="The member's access is suspended until an admin resets it">Wrong tries before locking</span><div class="input select" style="width:160px"><span class="grow">5</span></div></div>
      <div class="row"><span class="row-label" title="Give it to the members; every device is asked the new one">PIN</span><button class="btn">Change</button></div>
    </div>`);
}
// 36h. A member opens the organization: its PIN.
function saisiePin() {
  const side = sidebarB({ active: 'Example Studio' }).replace(/(<div class="side-item side-section[^"]*">\s*<svg[\s\S]*?<\/svg><span>Example Studio)/, '$1');
  const center = `<div class="center-stage" style="height:100%"><div class="card" style="width:340px">
      <div class="card-head"><span class="logo logo--lg">${i('lock')}</span><h1>Example Studio</h1><span style="color:var(--text-muted)" title="Set by your organization">Organization PIN</span></div>
      <div class="otp"><span>•</span><span>•</span><span>•</span><span class="is-focus"></span><span></span><span></span></div>
      <button class="btn btn--primary" style="justify-content:center;height:32px">Open</button>
    </div></div>`;
  return frameB(center, { side });
}

// ---------- 37 (27/11/2026): user's notes after trying 0.2.2 ----------
// 37a. "+" next to My vault: a folder without a name appears in place, its field has the keyboard.
// Esc cancels; leaving the field empty cancels; leaving it with a name keeps the folder.
function dossierEnLigne() {
  const side = sidebarB({ active: 'My vault' }).replace('<span>Shopping</span><span class="count">1</span></div>',
    `<span>Shopping</span><span class="count">1</span></div><div class="side-item sub is-editing" title="Enter or click elsewhere: keep · Esc: cancel">${i('folder')}<span class="caret"></span></div>`);
  return frameB(`${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div>`, { side });
}
// 37b. Double-click a folder's name: renamed in place, the name selected.
function dossierRenomme() {
  const side = sidebarB({ active: 'My vault' }).replace(/<div class="side-item[^"]*sub">(<svg(?:(?!<\/svg>)[\s\S])*<\/svg>)<span>Banking<\/span><span class="count">1<\/span><\/div>/,
    '<div class="side-item sub is-editing" title="Enter or click elsewhere: keep · Esc: cancel">$1<span class="sel">Banking</span></div>');
  return frameB(`${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div>`, { side });
}
// 37c. A 1 px grey bar between headings: where the width handle is.
function separateursColonnes() {
  // Every heading but the last gets the bar on its right edge.
  const t = vaultTable({ selected: null }).replace(/<th([^>]*)>/g, (m, attrs) => /class="/.test(attrs) ? `<th${attrs.replace('class="', 'class="sep ')}>` : `<th${attrs} class="sep">`)
    .replace(/<th([^>]*)class="sep ([^"]*)"([^>]*)>Yearly cost/, '<th$1class="$2"$3>Yearly cost').replace(/<th([^>]*) class="sep">Yearly cost/, '<th$1>Yearly cost');
  return frameB(`${vaultToolbar()}<div class="table-wrap">${t}</div>`);
}
// 37d. Top bar: the green dot alone; its tooltip names the connection.
function pastilleConnexion() {
  const html = frameB(`${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div>`,
    { overlay: '<div class="tooltip" style="right:236px;top:40px">Local</div>' });
  return html.replace(/<span class="sync" title="[^"]*"><span class="dot"><\/span>Synced<\/span>/, '<span class="sync" title="Local"><span class="dot"></span></span>')
    .replace(/<span class="avatar" title="Lea Martin">LM<\/span>/, '');
}
// 37e. Security › Opening Cleis: a PIN for the personal vault, as for the organization (36g).
function ouverturePin() {
  return settings('Security', `
    <div class="page-head"><h1>Security</h1></div>
    <div class="section">${sectionTitle('Opening Cleis', 'How this computer opens your vault. On another computer: the master password or the recovery key.')}
      <div class="row"><span class="row-label">Unlock Cleis with</span>
        <div class="segmented" role="radiogroup"><button role="radio" aria-checked="false">${i('key')}Master password</button><button role="radio" aria-checked="true" title="Digits only, on this computer; the master password after too many wrong tries">${i('hash')}PIN</button><button role="radio" aria-checked="false" title="Face, fingerprint or PIN of this Windows account">${i('fingerprint')}Windows Hello</button><button role="radio" aria-checked="false" title="Anyone using this Windows account opens Cleis">${i('laptop')}Nothing</button></div></div>
      <div class="row"><span class="row-label">Length</span><div class="segmented"><button role="radio" aria-checked="false">4</button><button role="radio" aria-checked="true">6</button><button role="radio" aria-checked="false">8</button></div></div>
      <div class="row"><span class="row-label" title="Then the master password or the recovery key">Wrong tries before locking</span><div class="input select" style="width:160px"><span class="grow">5</span></div></div>
      <div class="row"><span class="row-label">PIN</span><button class="btn">Change</button></div>
      <div class="row"><span class="row-label" title="Opens your vault anywhere; keep it outside Cleis">Recovery key</span><button class="btn">Show</button></div>
      <div class="row"><span class="row-label">Master password</span><button class="btn">Change</button><button class="btn btn--ghost" title="The recovery key then opens your vault on another computer">Remove</button></div>
    </div>`);
}
// 37f. Choosing PIN: typed twice in the same boxes as the organization PIN (36h).
function choisirPin() {
  return settings('Security', ouverturePin().match(/<div class="page"[^>]*>([\s\S]*)<\/div><\/div><\/div>/)[1],
    dlg('New PIN', `<div class="otp"><span>•</span><span>•</span><span>•</span><span>•</span><span class="is-focus"></span><span></span></div>
      <span style="color:var(--text-muted);text-align:center" title="Typed a second time to confirm">Confirm</span>
      <div class="otp"><span></span><span></span><span></span><span></span><span></span><span></span></div>
      <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Save</button></div>`));
}
// 37g. Opening Cleis with the PIN: the unlock screen takes the boxes of 36h.
function deverrouillerPin() {
  return `<div class="app"><main class="main" style="margin:6px"><div class="center-stage">
    <div class="card" style="width:340px">
      <div class="card-head"><span class="logo logo--lg">${i('key')}</span><h1>My vault</h1><span style="color:var(--text-muted)">PIN</span></div>
      <div class="otp"><span>•</span><span>•</span><span class="is-focus"></span><span></span><span></span><span></span></div>
      <button class="btn btn--primary" style="justify-content:center;height:32px">Open</button>
      <button class="btn btn--ghost" style="justify-content:center" title="Also after too many wrong tries">Use the master password</button>
    </div></div></main></div>`;
}
// 37h. First launch: PIN among the ways to open Cleis.
function premierLancementPin() {
  return premierLancementProtection()
    .replace('<button role="radio" aria-checked="false">Password</button><button role="radio" aria-checked="true">Windows Hello</button>',
      '<button role="radio" aria-checked="false">Password</button><button role="radio" aria-checked="true">PIN</button><button role="radio" aria-checked="false">Windows Hello</button>')
    .replace('</div>\n      </div>\n      <div style="display:flex;justify-content:space-between',
      `</div>
        <div class="otp"><span>•</span><span>•</span><span>•</span><span>•</span><span class="is-focus"></span><span></span></div>
      </div>
      <div style="display:flex;justify-content:space-between`);
}
// 37i. Preferences › Vaults: one Vault tab, shortcuts still per vault.
function raccourcisUnOnglet() {
  const tg = (label, on) => `<button class="tag-toggle ${on ? 'on' : ''}">${on ? i('check') : ''}${label}</button>`;
  return settings('Preferences', `
    <div class="page-head"><h1>Preferences</h1></div>
    <div class="section">${sectionTitle('Vaults')}
      <div class="row"><span class="row-label" title="Off: one Vault tab, every vault in its sidebar">Personal vault in its own tab</span><span class="switch"></span></div>
      <div class="row"><span class="row-label" title="All items, Favorites, Renew soon, at the top of each vault">Shortcuts</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Same for every vault</span><span class="switch"></span></div>
      <div class="row"><span class="row-label" style="padding-left:var(--s-4)">${i('user')}My vault</span>${tg('All items', true)}${tg('Favorites', true)}${tg('Renew soon', false)}</div>
      <div class="row"><span class="row-label" style="padding-left:var(--s-4)">${i('users')}Example Studio</span>${tg('All items', true)}${tg('Favorites', false)}${tg('Renew soon', true)}</div>
    </div>`);
}
// 37j. The single Vault tab with shortcuts per vault: each section carries its own, under its name.
function laterale37() {
  const sc = (ico, label, n) => `<div class="side-item side-shortcut">${i(ico)}<span>${label}</span><span class="count">${n}</span></div>`;
  const sec = (ico, label, count, add, act = false) => `<div class="side-item side-section ${act ? 'is-active' : ''}">${i(ico)}<span>${label}</span><span class="count">${count}</span><button class="icon-btn icon-btn--sm" aria-label="${add}" title="${add}">${i('plus')}</button></div>`;
  const f = (ico, label, n) => `<div class="side-item sub">${i(ico)}<span>${label}</span><span class="count">${n}</span></div>`;
  const side = `<aside class="sidebar pane">
    ${sec('user', 'My vault', 3, 'New folder', true)}${sc('star', 'Favorites', 1)}
    ${f('folder', 'Banking', 1)}${f('folder', 'Shopping', 1)}
    <div style="height:var(--s-2)"></div>
    ${sec('users', 'Example Studio', 12, 'New space')}${sc('star', 'Favorites', 2)}${sc('clock', 'Renew soon', 1)}
    ${f('folder', 'General', 4)}${f('folder', 'Design', 3)}${f('lock', 'Accounting', 2)}${f('folder', 'Dev', 3)}
    <div style="flex:1"></div>
    <div class="side-item">${i('upload')}<span>Import</span></div>
    <div class="side-row"><div class="side-item" style="flex:1">${i('trash')}<span>Trash</span></div><button class="icon-btn icon-btn--sm" aria-label="Customize" title="Customize: reorder and delete">${i('grip')}</button></div>
  </aside>`;
  return frameB(`${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div>`, { side });
}
// 37k. Preferences › Start in: each vault tab when vaults have their own tabs.
function demarrerDans() {
  return settings('Preferences', `
    <div class="page-head"><h1>Preferences</h1></div>
    <div class="section">${sectionTitle('Vaults')}
      <div class="row"><span class="row-label">Personal vault in its own tab</span><span class="switch on"></span></div>
    </div>
    <div class="section">${sectionTitle('Start')}
      <div class="row"><span class="row-label">Start in</span><div class="input select" style="width:200px"><span class="grow">My vault</span></div></div>
    </div>`, `<div class="menu" style="left:1089px;top:312px;width:200px">${mi('check', 'My vault')}${mi('', 'Example Studio')}${mi('', 'Acme Partners')}${sep}${mi('', 'All items')}${mi('', 'Favorites')}${mi('', 'Contacts')}</div>`);
}
// 37l. Sync & backups: no sentence under the titles (tooltips); at the very bottom, the deletions.
function synchro37() {
  return settings('Sync & backups', `
    <div class="page-head"><h1 style="flex:none">Sync & backups</h1><span style="flex:1"></span><button class="btn btn--primary">${i('sync')}Sync now</button></div>
    <div class="section">${sectionTitle('Sync', 'Your encrypted vault, kept the same on every computer that uses this place')}
      <div class="row"><span class="site-ico">${i('laptop')}</span><div class="member row-label"><div class="meta"><span>Local</span></div></div><button class="btn btn--sm">Choose folder</button></div>
    </div>
    <div class="section">${sectionTitle('New device', 'What a new computer needs besides your master password')}
      <div class="row"><span class="row-label" title="Where your vault is, a space to write your master password">${i('file')}Emergency kit</span><button class="btn btn--sm">${i('download')}Download</button></div>
    </div>
    <div class="section">${sectionTitle('Backups', 'Dated copies on this computer you can go back to')}
      <div class="row"><span class="row-label">Keep</span><div class="input select" style="width:160px"><span class="grow">30 versions</span></div><button class="btn btn--sm">${i('upload')}Back up now</button></div>
      <div class="row"><span class="row-label">Encrypted export</span><button class="btn btn--sm">${i('download')}Export</button></div>
    </div>
    <div class="section">${sectionTitle('Delete')}
      <div class="row"><span class="row-label" title="The 12 dated copies on this computer; the vault stays">All backups</span><button class="btn btn--danger-ghost btn--sm">${i('trash')}Delete all backups</button></div>
      <div class="row"><span class="row-label" title="Vault, backups, settings and this computer's unlock; Cleis starts again from the first launch">Account and its data</span><button class="btn btn--danger btn--sm">${i('trash')}Delete account</button></div>
    </div>`, '', 120);
}
// 37m. Deleting the account: the master password confirms; the copy at the sync place can go too.
function supprimerCompte() {
  return settings('Sync & backups', synchro37().match(/<div class="page"[^>]*>([\s\S]*)<\/div><\/div><\/div>/)[1],
    dlg('Delete your account?', `<span style="color:var(--text-muted)" title="Organizations keep their own spaces">Vault, 12 backups, settings. It cannot be undone.</span>
      <label class="row" style="padding:0"><span class="check on">${i('check')}</span><span class="row-label">Also the copy on GitHub</span></label>
      <div class="field"><span class="label">Master password</span><div class="input is-focus"><span class="grow dots">••••••</span></div></div>
      <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--danger">Delete account</button></div>`), 120);
}
// 37n. Account › GitHub, like Revel: a code to type on github.com, no token to make.
function connexionGithub() {
  return settings('Account', compte().match(/<div class="page"[^>]*>([\s\S]*)<\/div><\/div><\/div>/)[1],
    dlg('Sign in with GitHub', `<div class="code-box"><span class="code code--lg">WDJB-MJHT</span><button class="icon-btn icon-btn--sm" aria-label="Copy" title="Copy">${i('copy')}</button></div>
      <div style="display:flex;align-items:center;gap:var(--s-2);color:var(--text-muted)" title="A private repository cleis-vault is made in your account"><span class="spinner"></span>Waiting for GitHub…</div>
      <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary" title="github.com/login/device, the code copied">${i('external')}Open GitHub</button></div>`));
}

// 37o. Right-click on the password field of the panel: its own menu (the row's menu is 33).
function menuChampMotDePasse() {
  const m = ctx(1186, 300, `${mi('copy', 'Copy', 'Ctrl+Shift+C')}${mi('', 'Paste', 'Ctrl+V')}${mi('eye', 'Show')}${sep}${mi('refresh', 'Generate a password')}${mi('clock', 'Previous passwords')}`, 220);
  return shell('Vault', `
    <div class="work">${sidebarB({ active: 'Design' })}
      <section class="center" style="position:relative">${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: 'Figma' })}</div></section>
      ${inspector().replace('<div class="input"><span class="grow dots">', '<div class="input is-focus"><span class="grow dots">')}
    </div>${statusbar()}`, { overlay: m });
}

// 37p. Right-click on a row: its menu opens and the row is outlined; the selection and the panel
// stay as they were (only a left click opens an item). The menu's keys act on the outlined row.
function clicDroitLigne() {
  const m = ctx(560, 300, `${mi('copy', 'Copy username', 'Ctrl+C')}${mi('key', 'Copy password', 'Ctrl+Shift+C')}${mi('totp', 'Copy 2FA code', 'Ctrl+Alt+C')}${mi('external', 'Open website')}${sep}${mi('star', 'Add to favorites')}${mi('copy', 'Duplicate', 'Ctrl+D')}${mi('folder', 'Move to', '', { sub: true })}${sep}${mi('trash', 'Move to trash', 'Del', { danger: true })}`);
  return shell('Vault', `
    <div class="work">${sidebarB({ active: 'Design' })}
      <section class="center" style="position:relative">${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null, aimed: 'Notion' })}</div>${m}</section>
    </div>${statusbar()}`);
}
// 37q. Security policy › Organization PIN › Change: the boxes of 37f, the policy's length.
function choisirPinOrganisation() {
  return settings('Security policy', politiquePin().match(/<div class="page"[^>]*>([\s\S]*)<\/div><\/div><\/div>/)[1],
    dlg('Organization PIN', `<div class="otp"><span>•</span><span>•</span><span class="is-focus"></span><span></span><span></span><span></span></div>
      <span style="color:var(--text-muted);text-align:center" title="Typed a second time to confirm">Confirm</span>
      <div class="otp"><span></span><span></span><span></span><span></span><span></span><span></span></div>
      <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Save</button></div>`));
}

// 37r. First launch › Sign in › GitHub, the account has no vault yet: the account named, then
// "Create my vault" (made here, then sent to a private repository cleis-vault on the account,
// once its recovery key is put away), another account, or cancel.
function githubSansCoffre() {
  return restaurer().replace('</main></div>', `${dlg('No vault on this account', `<div class="code-box" style="justify-content:flex-start" title="No private repository cleis-vault on this account">${i('repo')}<span style="flex:1">@lea-example</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Signed in</span></div>
      <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--ghost" title="Sign out of github.com first, then sign in with the other account">Other account</button><button class="btn btn--primary" title="A new vault on this computer, saved to the private repository cleis-vault of this account">Create my vault</button></div>`)}</main></div>`);
}

// ---------- 39 (27/11/2026): Account and Security in one page ----------
// 39a. Settings › Account: the account (GitHub, signed in) with Sign out and Revoke access, then
// what was Security: opening Cleis, two-factor authentication (was "Verification code"), locking,
// clipboard; Delete account at the bottom, as in Sync & backups. No "Security" page any more.
function navCompte(active) {
  return settingsNav(active).replace(/<div class="side-item[^"]*">(<svg(?:(?!<\/svg>)[\s\S])*<\/svg>)<span>(Security|Profile)<\/span><\/div>/g, '');
}
const settingsCompte = (body, overlay = '', scroll = 0) => shell('Settings', `<div class="settings">${navCompte('Account')}<div class="settings-body"><div class="page" ${scroll ? `style="margin-top:-${scroll}px"` : ''}>${body}</div></div></div>`, { primary: null, overlay });
function compteFusion() {
  return `
    <div class="page-head"><h1>Account</h1></div>
    <div class="section">${sectionTitle('GitHub')}
      <div class="row"><span class="site-ico">${i('repo')}</span><span class="row-label" style="flex:1"><span>@lea-example</span><span class="code" style="color:var(--text-muted);font-size:var(--text-sm);margin-left:var(--s-2)">cleis-vault</span></span><span class="chip chip--ok" title="Last sync today at 10:42"><span class="dot"></span>Synced 2 min ago</span><button class="btn btn--ghost btn--sm" title="This computer stops following; the copy there stays sealed">Sign out</button><button class="btn btn--danger-ghost btn--sm" title="Removes Cleis' access to your GitHub repositories, on every computer">Revoke access</button></div>
    </div>
    <div class="section">${sectionTitle('Opening Cleis', 'How this computer opens your vault. On another computer: the password or the recovery key.')}
      <div class="row"><span class="row-label">Unlock Cleis with</span>
        <div class="segmented" role="radiogroup"><button role="radio" aria-checked="false">${i('key')}Password</button><button role="radio" aria-checked="true" title="Digits only, on this computer; the password after too many wrong tries">${i('hash')}PIN</button><button role="radio" aria-checked="false" title="Face, fingerprint or PIN of this Windows account">${i('fingerprint')}Windows Hello</button><button role="radio" aria-checked="false" title="Anyone using this Windows account opens Cleis">${i('laptop')}Nothing</button></div></div>
      <div class="row"><span class="row-label">Length</span><div class="segmented"><button role="radio" aria-checked="false">4</button><button role="radio" aria-checked="true">6</button><button role="radio" aria-checked="false">8</button></div></div>
      <div class="row"><span class="row-label" title="Then the password or the recovery key">Wrong tries before locking</span><div class="input select" style="width:160px"><span class="grow">5</span></div></div>
      <div class="row"><span class="row-label">PIN</span><button class="btn">Change</button></div>
      <div class="row"><span class="row-label" title="Opens your vault anywhere; keep it outside Cleis">Recovery key</span><button class="btn">Show</button></div>
      <div class="row"><span class="row-label">Password</span><button class="btn">Change</button></div>
    </div>
    <div class="section">${sectionTitle('Two-factor authentication', 'A code from your authenticator app, asked each time Cleis opens, on top of the password or PIN. This computer only: an organization’s two-step verification is in Profile.')}
      <div class="row"><span class="row-label">2FA when Cleis opens</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Method</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="true">${i('phone')}App</button><button role="radio" aria-checked="false">${i('mail')}Email</button><button role="radio" aria-checked="false">SMS</button></div></div>
      <div class="row"><span class="row-label" title="Each one opens Cleis once without the app">Recovery codes</span><span style="color:var(--text-muted);font-size:var(--text-sm)">8 left</span><button class="btn">Show</button></div>
    </div>
    <div class="section">${sectionTitle('Locking')}
      <div class="row"><span class="row-label">After inactivity</span><div class="input select" style="width:140px"><span class="grow">15 min</span></div></div>
    </div>
    <div class="section">${sectionTitle('Clipboard')}
      <div class="row"><span class="row-label">Clear after copying</span><div class="input select" style="width:140px"><span class="grow">30 s</span></div></div>
    </div>
    <div class="section">${sectionTitle('Example Studio', 'What the organization knows of you; was Profile')}
      <div class="row"><span class="row-label" title="Shown to the other members">Name</span><div class="input" style="width:220px"><span class="grow">Lea Martin</span></div></div>
      <div class="row"><span class="row-label" title="Signs you in to the server; never leaves your vault">Account key</span><span class="code" style="color:var(--text-muted);font-size:var(--text-sm)">7F3A · 91C2 · 0B6E · D44F</span></div>
      <div class="row"><span class="row-label" title="Asked by the server at sign-in">Two-step verification</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Authenticator app</span></div>
    </div>
    <div class="section">${sectionTitle('Delete')}
      <div class="row"><span class="row-label" title="Vault, backups, settings and this computer's unlock; Cleis starts again from the first launch">Account and its data</span><button class="btn btn--danger btn--sm">${i('trash')}Delete account</button></div>
    </div>`;
}
function compteFusionHaut() { return settingsCompte(compteFusion()); }
// 39b. The bottom of the same page.
function compteFusionBas() { return settingsCompte(compteFusion(), '', 720); }
// 39c. Revoke access: Cleis forgets its GitHub access here, then github.com opens on Cleis' page
// (Settings › Applications › Cleis) where "Revoke access" removes it everywhere (GitHub lets only
// the account holder do it from the app's side without a secret).
function revoquerGithub() {
  return settingsCompte(compteFusion(), dlg('Revoke GitHub access?', `<span style="color:var(--text-muted)" title="Your vault stays on this computer and in lea-example/cleis-vault">Every computer stops syncing with @lea-example.</span>
      <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--danger" title="Then click Revoke access on the page GitHub opens">${i('external')}Revoke on GitHub</button></div>`));
}
// 39d. Signed out (Local): the GitHub access stays revocable from here.
function compteLocalRevoquer() {
  const prov = (ico, name, on = false, tip = '') => `<div class="choice ${on ? 'is-active' : ''}" style="padding:var(--s-3)" ${tip ? `title="${tip}"` : ''}><span class="radio ${on ? 'on' : ''}"></span><div class="txt"><b style="font-weight:var(--weight-medium)">${name}</b></div><span class="ico">${i(ico)}</span></div>`;
  return settingsCompte(`
    <div class="page-head"><h1>Account</h1></div>
    <div class="section">${sectionTitle('Sign in', 'Your vault, settings, backups and organizations follow you to your other computers. Everything is encrypted on this computer before it leaves.')}
      ${prov('laptop', 'Local', true, 'This computer only')}${prov('window', 'Microsoft', false, 'Available once Cleis is registered with Microsoft')}${prov('globe', 'Google', false, 'Available once Cleis is registered with Google')}${prov('repo', 'GitHub', false, 'A private repository')}
      <div style="display:flex;justify-content:space-between;align-items:center"><button class="btn btn--ghost btn--sm" title="Opens Cleis' page on github.com, to remove its access to your repositories">${i('external')}Revoke GitHub access</button><button class="btn btn--primary">Sign in</button></div>
    </div>
    <div class="section">${sectionTitle('Opening Cleis')}
      <div class="row"><span class="row-label">Unlock Cleis with</span>
        <div class="segmented" role="radiogroup"><button role="radio" aria-checked="true">${i('key')}Master password</button><button role="radio" aria-checked="false">${i('hash')}PIN</button><button role="radio" aria-checked="false">${i('fingerprint')}Windows Hello</button><button role="radio" aria-checked="false">${i('laptop')}Nothing</button></div></div>
    </div>`);
}

// ---------- 40 (27/11/2026): one journey from the first screen, the account in the top bar ----------
// The first screen keeps one card: choosing GitHub (Microsoft, Google) selects "Sign in", the code
// is typed on the provider's page, then the card goes on by itself: no vault on the account →
// the vault is made here and saved there (40b); a vault there → it is opened here (40c). No other
// screen, no "Restore" detour.
const welcomeCard = (inner, { org = false } = {}) => `<div class="app"><main class="main" style="margin:6px"><div class="center-stage">
    <div class="card" style="width:460px">
      <div class="card-head"><span class="logo logo--lg">${i('key')}</span><h1>Welcome to Cleis</h1></div>${inner}</div></div></main></div>`;
const chLocal = (on) => `<div class="choice ${on ? 'is-active' : ''}"><span class="radio ${on ? 'on' : ''}"></span><div class="txt"><b style="font-weight:var(--weight-medium)">Local account</b></div><span class="ico">${i('laptop')}</span></div>`;
const chOrg = `<div class="choice"><span class="radio"></span><div class="txt"><b style="font-weight:var(--weight-medium)">Join an organization</b></div><span class="ico">${i('server')}</span></div>`;
const chSignIn = (state) => {
  const prov = (ico, n, on) => `<button class="btn btn--sm" ${on ? 'style="border-color:var(--focus);color:var(--text)"' : ''}>${i(ico)}${n}</button>`;
  const body = state === 'none'
    ? `<div style="display:flex;gap:8px;margin-top:var(--s-3)">${prov('window', 'Microsoft')}${prov('globe', 'Google')}${prov('repo', 'GitHub')}</div>`
    : `<div style="display:flex;align-items:center;gap:8px;margin-top:var(--s-3)"><span class="avatar" style="width:20px;height:20px;font-size:10px">L</span><span>@lea-example</span><span class="chip chip--ok" title="GitHub"><span class="dot"></span>GitHub</span><span style="flex:1"></span><button class="btn btn--ghost btn--sm" title="Another account or provider">Change</button></div>`;
  const on = state !== 'none' || state === 'pick';
  return `<div class="choice ${on ? 'is-active' : ''}" ${state === 'none' ? 'title="Your vault follows you to your other computers"' : ''}><span class="radio ${on ? 'on' : ''}"></span><div class="txt" style="flex:1"><b style="font-weight:var(--weight-medium)">Sign in</b>${body}</div><span class="ico">${i('user')}</span></div>`;
};
const unlockWith = `<div class="row" style="padding:0;flex-wrap:wrap;row-gap:10px"><span class="row-label">Unlock Cleis with</span>
          <div class="segmented" role="radiogroup"><button role="radio" aria-checked="true">Password</button><button role="radio" aria-checked="false">PIN</button><button role="radio" aria-checked="false">Windows Hello</button><button role="radio" aria-checked="false" title="Anyone using this Windows account opens Cleis">Nothing</button></div></div>`;
const pwFields = `<div class="field"><span class="label">Password</span><div class="input is-focus"><span class="grow dots">••••••••••••</span><span class="act">${i('eye')}</span></div></div>
        <div class="field"><span class="label">Confirm</span><div class="input"><span class="grow"></span><span class="act">${i('eye')}</span></div></div>`;
// 40a. GitHub clicked: "Sign in" is the choice, the code waits on github.com.
function parcoursCode() {
  return welcomeCard(`${chLocal(false)}${chSignIn('pick').replace('<div style="display:flex;gap:8px;margin-top:var(--s-3)">', '<div style="display:flex;gap:8px;margin-top:var(--s-3)">').replace(/(<button class="btn btn--sm")(>[\s\S]*?GitHub<\/button>)/, '$1 style="border-color:var(--focus);color:var(--text)"$2')}${chOrg}`)
    .replace('</main></div>', `${dlg('Sign in with GitHub', `<div class="code-box"><span class="code code--lg">WDJB-MJHT</span><button class="icon-btn icon-btn--sm" aria-label="Copy" title="Copy">${i('copy')}</button></div>
      <div style="display:flex;align-items:center;gap:var(--s-2);color:var(--text-muted)"><span class="spinner"></span>Waiting for GitHub…</div>
      <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary" title="github.com/login/device, the code copied">${i('external')}Open GitHub</button></div>`)}</main></div>`);
}
// 40b. Signed in, no vault on the account: the card goes on to the creation, the account kept.
function parcoursCreer() {
  return welcomeCard(`${chLocal(false)}${chSignIn('signed')}${chOrg}
      <div class="section" style="gap:var(--s-3)">${unlockWith}${pwFields}</div>
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary" title="Then saved to a private repository cleis-vault on @lea-example">Create vault</button></div>`);
}
// 40c. Signed in, a vault already on the account: opened here with its password or recovery key.
function parcoursOuvrir() {
  return welcomeCard(`${chLocal(false)}${chSignIn('signed')}${chOrg}
      <div class="section" style="gap:var(--s-3)">
        <div class="row" style="padding:0"><span class="row-label">${i('repo')}<span class="code">lea-example/cleis-vault</span></span><span style="color:var(--text-muted);font-size:var(--text-sm)">Synced 2 min ago</span></div>
        <div class="field"><span class="label">Password or recovery key</span><div class="input is-focus"><span class="grow dots">••••••••••••</span><span class="act">${i('eye')}</span></div></div>
      </div>
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary">Open vault</button></div>`);
}
// 40f. First screen, nothing chosen: the three options alone in the middle. Choosing one moves
// them up and shows how the vault opens (40b); a click on an option brings them back here.
function parcoursDebut() {
  return welcomeCard(`${chLocal(false)}${chSignIn('none')}${chOrg}
      <div style="display:flex"><button class="btn btn--ghost">${i('sync')}Restore my vault</button></div>`);
}
// 40d. Top bar: the account's picture at the right; its menu.
function profilCompte() {
  const avatar = '<span class="avatar" style="background:linear-gradient(135deg,#6b8f71,#3d5a80);color:#fff" title="@lea-example · GitHub">L</span>';
  const menu = `<div class="menu" style="right:132px;top:44px;width:260px">
      <div class="member" style="padding:var(--s-2) var(--s-2) var(--s-3)"><span class="avatar" style="width:32px;height:32px;background:linear-gradient(135deg,#6b8f71,#3d5a80);color:#fff">L</span><div class="meta"><span style="font-weight:var(--weight-medium)">@lea-example</span><span class="sub">GitHub · Synced 2 min ago</span></div></div>
      ${sep}${mi('sync', 'Sync now')}${mi('gear', 'Account')}${sep}${mi('logout', 'Sign out')}</div>`;
  const html = shell('Vault', `<div class="work">${sidebar()}<section class="center">${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div></section></div>${statusbar()}`, { overlay: menu });
  return html.replace(/<span class="avatar" title="Lea Martin">LM<\/span>/, avatar);
}
// 40e. Settings › Account, signed out: no "Revoke"; the providers to sign in with.
function compteLocal40() {
  const prov = (ico, name, on = false, tip = '') => `<div class="choice ${on ? 'is-active' : ''}" style="padding:var(--s-3)" ${tip ? `title="${tip}"` : ''}><span class="radio ${on ? 'on' : ''}"></span><div class="txt"><b style="font-weight:var(--weight-medium)">${name}</b></div><span class="ico">${i(ico)}</span></div>`;
  return settingsCompte(`
    <div class="page-head"><h1>Account</h1></div>
    <div class="section">${sectionTitle('Sign in', 'Your vault, settings, backups and organizations follow you to your other computers. Everything is encrypted on this computer before it leaves.')}
      ${prov('laptop', 'Local', true, 'This computer only')}${prov('window', 'Microsoft', false, 'OneDrive, in a folder only Cleis sees')}${prov('globe', 'Google', false, 'Google Drive, in a folder only Cleis sees')}${prov('repo', 'GitHub', false, 'A private repository')}
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary">Sign in</button></div>
    </div>
    <div class="section">${sectionTitle('Opening Cleis')}
      <div class="row"><span class="row-label">Unlock Cleis with</span>
        <div class="segmented" role="radiogroup"><button role="radio" aria-checked="true">${i('key')}Password</button><button role="radio" aria-checked="false">${i('hash')}PIN</button><button role="radio" aria-checked="false">${i('fingerprint')}Windows Hello</button><button role="radio" aria-checked="false">${i('laptop')}Nothing</button></div></div>
    </div>`);
}

// ---------- 41 (27/11/2026): GitHub App (D-97) — Cleis reaches only cleis-vault ----------
// After the code (40a), GitHub has to let Cleis into one repository. Two steps in the same dialog,
// each one a page of GitHub opened by its button; Cleis sees by itself when each is done.
const ghStep = (n, title, state, btn, tip) => `<div class="row" style="padding:var(--s-2) 0">${state === 'done' ? `<span style="width:20px;height:20px;border-radius:999px;display:inline-grid;place-items:center;background:var(--accent);color:#000">${i('check')}</span>` : ''}<span class="row-label" ${tip ? `title="${tip}"` : ''}>${title}</span>${state === 'wait' ? '<span style="display:flex;align-items:center;gap:6px;color:var(--text-muted);font-size:var(--text-sm)"><span class="spinner"></span>Waiting for GitHub…</span>' : ''}${btn ? `<button class="btn ${state === 'next' ? 'btn--primary' : ''} btn--sm">${i('external')}${btn}</button>` : ''}</div>`;
function ghAppCreer() {
  return parcoursCode().replace(/<div class="overlay">[\s\S]*<\/div><\/div><\/main>/, `${dlg('Your vault on GitHub', `<div class="code-box" style="justify-content:flex-start"><span class="avatar" style="width:20px;height:20px;font-size:10px">L</span><span style="flex:1">@lea-example</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Signed in</span></div>
      ${ghStep(1, 'Create the private repository cleis-vault', 'next', 'Create on GitHub', 'The page opens filled in: Private, cleis-vault. Click Create repository.')}
      <div class="dialog-actions"><button class="btn">Cancel</button></div>`)}</main>`);
}
function ghAppInstaller() {
  return parcoursCode().replace(/<div class="overlay">[\s\S]*<\/div><\/div><\/main>/, `${dlg('Your vault on GitHub', `<div class="code-box" style="justify-content:flex-start"><span class="avatar" style="width:20px;height:20px;font-size:10px">L</span><span style="flex:1">@lea-example</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Signed in</span></div>
      ${ghStep(1, 'Create the private repository cleis-vault', 'done', '', '')}
      ${ghStep(2, 'Install Cleis', 'next', 'Install on GitHub', 'Only select repositories › cleis-vault › Install')}
      <div style="display:flex;align-items:center;gap:6px;color:var(--text-muted);font-size:var(--text-sm)"><span class="spinner"></span>Waiting for GitHub…</div>
      <div class="dialog-actions"><button class="btn">Cancel</button></div>`)}</main>`);
}

// ---------- 42 (27/11/2026): an organization rules access to itself, never how Cleis opens ----------
// Security policy: "Opening Cleis" and "Organization PIN" become one section, "Accessing <org>".
// The admin decides whether the organization's spaces ask for something before they show, and
// what: a password or a PIN set by the admin, or Windows Hello (Touch ID on a Mac) of each member's
// own computer. How a member opens Cleis itself (and the personal vault) stays theirs alone.
// "Locking" goes into it: the inactivity delay and the offline access concern the organization only.
const navPol = (active) => settingsNav(active).replace(/<div class="side-item[^"]*">(<svg(?:(?!<\/svg>)[\s\S])*<\/svg>)<span>(Security|Profile)<\/span><\/div>/g, '');
const settingsPol = (active, body, overlay = '') => shell('Settings', `<div class="settings">${navPol(active)}<div class="settings-body"><div class="page">${body}</div></div></div>`, { primary: null, overlay });
const sel160 = (v, w = 160) => `<div class="input select" style="width:${w}px"><span class="grow">${v}</span></div>`;
function accesOrg({ on = true, how = 'Password', set = true } = {}) {
  const secret = how === 'Windows Hello' ? '' : how === 'PIN'
    ? `<div class="row"><span class="row-label" title="Another length asks for a new PIN">Length</span><div class="segmented"><button role="radio" aria-checked="false">4</button><button role="radio" aria-checked="true">6</button><button role="radio" aria-checked="false">8</button></div></div>
      <div class="row"><span class="row-label" title="Give it to the members; every device is asked the new one">PIN</span>${set ? '' : '<span class="chip chip--warn"><span class="dot"></span>Not set yet</span>'}<button class="btn">${set ? 'Change' : 'Set'}</button></div>`
    : `<div class="row"><span class="row-label" title="Give it to the members; every device is asked the new one">Password</span>${set ? '' : '<span class="chip chip--warn"><span class="dot"></span>Not set yet</span>'}<button class="btn">${set ? 'Change' : 'Set'}</button></div>`;
  const tries = how === 'Windows Hello' ? '' : `<div class="row"><span class="row-label" title="The member's access is suspended until an admin resets it">Wrong tries before locking</span>${sel160('5')}</div>`;
  const more = on ? `
      <div class="row"><span class="row-label">With</span>${sel160(how === 'Windows Hello' ? 'Windows Hello' : how, 220)}</div>
      ${secret}
      <div class="row"><span class="row-label" title="Since it was last given on the device">Ask again after</span>${sel160('8 hours')}</div>
      <div class="row"><span class="row-label" title="Without using Cleis">After inactivity</span>${sel160('15 minutes')}</div>
      ${tries}` : '';
  return `<div class="section">${sectionTitle('Accessing Example Studio', 'Asked before the organization’s spaces show. How each member opens Cleis and their own vault stays their choice.')}
      <div class="row"><span class="row-label" title="How long the organization's spaces stay readable without reaching the server">Offline access</span>${sel160('14 days')}</div>
      <div class="row"><span class="row-label">Ask for authentication</span><span class="switch ${on ? 'on' : ''}"></span></div>${more}
    </div>`;
}
const politique42 = (acc) => `
    <div class="page-head"><h1>Security policy</h1></div>
    ${acc}
    <div class="section">${sectionTitle('Signing in to the server', 'Applies to every Example Studio member.')}
      <div class="row"><span class="row-label">Two-step verification</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="false">Optional</button><button role="radio" aria-checked="true">Required</button></div></div>
      <div class="row"><span class="row-label">Remember device</span>${sel160('30 days')}</div>
    </div>`;
// 42a. Asking nothing: one switch (and the offline access, always there).
function accesOrgOff() { return settingsPol('Security policy', politique42(accesOrg({ on: false }))); }
// 42b. A password set by the admin: asked again after 8 hours, or 15 minutes without use.
function accesOrgMotDePasse() { return settingsPol('Security policy', politique42(accesOrg())); }
// 42c. "With": password, PIN, or the member's Windows Hello / Touch ID. In an organization kept in a
// shared folder, each computer counts the wrong tries.
function accesOrgMenu() {
  const m = `<div class="menu" style="left:1069px;top:296px;width:220px">${mi('check', 'Password')}${mi('', 'PIN')}<div class="menu-item" title="Touch ID on a Mac"><span style="width:16px"></span><span>Windows Hello</span></div></div>`;
  return settingsPol('Security policy', politique42(accesOrg()), m);
}
// 42d. PIN: its length, then the PIN typed twice (37q's dialog).
function accesOrgPin() { return settingsPol('Security policy', politique42(accesOrg({ how: 'PIN', set: false }))); }
// 42e. Windows Hello / Touch ID: nothing to set or give; each member's own computer checks them.
function accesOrgHello() { return settingsPol('Security policy', politique42(accesOrg({ how: 'Windows Hello' }))); }
// 42f. Password › Set: typed twice; at least 10 characters.
function accesOrgDefinir() {
  return settingsPol('Security policy', politique42(accesOrg({ set: false })), dlg('Example Studio password', `<div class="field"><span class="label" title="At least 10 characters">Password</span><div class="input is-focus"><span class="grow dots">••••••••••••</span><span class="act">${i('eye')}</span></div></div>
      <div class="field"><span class="label">Confirm</span><div class="input"><span class="grow"></span><span class="act">${i('eye')}</span></div></div>
      <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--primary">Save</button></div>`));
}
// A fictional organization logo (Example Studio), as an admin would upload it.
const orgLogo = (size = 48) => `<span title="Example Studio" style="width:${size}px;height:${size}px;border-radius:var(--radius-lg);display:inline-grid;place-items:center;background:#1f3b4d;flex:none"><svg width="${size * 0.6}" height="${size * 0.6}" viewBox="0 0 24 24"><circle cx="9" cy="12" r="6" fill="#4fb3bf"/><rect x="11" y="6" width="10" height="12" rx="2" fill="#f2c14e" opacity=".9"/></svg></span>`;
// 42g. A member: the organization's section shows its lock and asks its password; the personal
// vault and everything else stay open.
function accesOrgMembre() {
  const center = `<div class="center-stage" style="height:100%"><div class="card" style="width:340px">
      <div class="card-head"><span style="position:relative;display:inline-grid">${orgLogo()}<span style="position:absolute;right:-6px;bottom:-6px;width:20px;height:20px;border-radius:999px;display:grid;place-items:center;background:var(--surface-raised);border:1px solid var(--line-strong)">${i('lock')}</span></span><h1>Example Studio</h1><span style="color:var(--text-muted)" title="Set by Example Studio's admins">Organization password</span></div>
      <div class="input is-focus"><span class="grow dots">•••••</span><span class="act">${i('eye')}</span></div>
      <button class="btn btn--primary" style="justify-content:center;height:32px">Open</button>
    </div></div>`;
  return frameB(center, { side: sidebarB({ active: 'Example Studio' }) });
}
// 42h. A member, Windows Hello: one button (Touch ID on a Mac). Not set up on the computer: the
// button opens Windows' settings, its tooltip says so.
function accesOrgMembreHello() {
  const center = `<div class="center-stage" style="height:100%"><div class="card" style="width:340px">
      <div class="card-head"><span style="position:relative;display:inline-grid">${orgLogo()}<span style="position:absolute;right:-6px;bottom:-6px;width:20px;height:20px;border-radius:999px;display:grid;place-items:center;background:var(--surface-raised);border:1px solid var(--line-strong)">${i('lock')}</span></span><h1>Example Studio</h1></div>
      <button class="btn btn--primary" style="justify-content:center;height:32px" title="Face, fingerprint or PIN of this Windows account">${i('fingerprint')}Open with Windows Hello</button>
    </div></div>`;
  return frameB(center, { side: sidebarB({ active: 'Example Studio' }) });
}

// Server: the Super Admin deletes it. Shown to the Super Admin only, at the bottom.
const serveur42 = (logoRow = `<span title="Change" style="position:relative;display:inline-grid;cursor:pointer">${orgLogo(32)}</span><button class="icon-btn icon-btn--sm" aria-label="Remove" title="Remove">${i('x')}</button>`) => `
    <div class="page-head"><h1>Server</h1></div>
    <div class="section">${sectionTitle('Example Studio')}
      <div class="row"><span class="row-label" title="Shown to the members when Example Studio asks for authentication. PNG or JPEG, square.">Logo</span>${logoRow}</div>
      <div class="row"><span class="row-label">Hosted</span><span>This computer</span><span class="chip chip--ok"><span class="dot"></span>Running</span></div>
      <div class="row"><span class="row-label" title="Give one of these with each invitation code">Addresses for members</span><span class="code" style="color:var(--text-muted)">192.168.1.24:8443 · 10.8.0.3:8443</span><button class="icon-btn" title="Copy">${i('copy')}</button></div>
      <div class="row"><span class="row-label" title="Otherwise it runs while Cleis runs">Start with the system</span><button class="btn">Install</button></div>
      <div class="row"><span class="row-label">Certificate</span><span class="code" style="color:var(--text-muted)">SHA256:4f1c·9a2e·77b0·d31f</span></div>
      <div class="row"><span class="row-label"></span><button class="btn">${i('sync')}Sync now</button><button class="btn btn--danger-ghost">Leave</button></div>
    </div>
    <div class="section">${sectionTitle('Delete')}
      <div class="row"><span class="row-label" title="Its spaces, members, log and backups. Members' copies go at their next sync.">Server and its data</span><button class="btn btn--danger btn--sm">${i('trash')}Delete server</button></div>
    </div>`;
// 42i. Server › Delete (Super Admin only).
function serveurSupprimer() { return settingsPol('Server', serveur42()); }
// 42k. Server › Logo: none yet ("Add"), or the logo itself as the button (hover: outlined, an upload
// sign over it) and ×. Admins with the Rules right.
function serveurLogoSurvol() {
  const hover = `<span title="Change" style="position:relative;display:inline-grid;cursor:pointer">${orgLogo(32)}<span style="position:absolute;inset:0;border-radius:var(--radius-lg);border:1px solid var(--focus);background:rgb(0 0 0 / .55);display:grid;place-items:center;color:#fff">${i('upload')}</span></span><button class="icon-btn icon-btn--sm" aria-label="Remove" title="Remove">${i('x')}</button>`;
  return settingsPol('Server', serveur42(hover));
}
function serveurLogo() { return settingsPol('Server', serveur42(`<button class="btn btn--sm">${i('plus')}Add</button>`)); }
// 42j. Confirmed by typing the organization's name; an encrypted export first, ticked.
function serveurSupprimerConfirmer() {
  return settingsPol('Server', serveur42(), dlg('Delete the server?', `<span style="color:var(--text-muted)" title="Server on another machine: its data is erased, the program stays installed there">Example Studio, 14 members, 6 spaces. It cannot be undone.</span>
      <label class="row" style="padding:0"><span class="check on">${i('check')}</span><span class="row-label" title="Encrypted, opened by the recovery key of the organization">Export the spaces first</span></label>
      <div class="field"><span class="label">Type Example Studio to confirm</span><div class="input is-focus"><span class="grow">Example Stu</span></div></div>
      <div class="dialog-actions"><button class="btn">Cancel</button><button class="btn btn--danger" disabled>Delete server</button></div>`));
}

// ---------- 43 (27/11/2026): Health › Change asks first ----------
// "Change" no longer replaces the password at once: a floating card opens under the button, its
// right edge on the list's card. In it a generated password, editable by hand, a shuffle button
// (another one) and Apply. Esc or a click elsewhere closes it, nothing changed.
function santeChanger() {
  const pb = (t, cls) => `<span class="chip ${cls}" style="margin-right:4px">${t}</span>`;
  const r = (c, n, probs, sp, ch, on = false) => `<tr class="${on ? 'is-aimed' : ''}"><td><span class="site"><span class="site-ico">${c}</span><span class="site-name">${n}</span></span></td><td>${probs}</td><td class="muted">${sp}</td><td class="muted">${ch}</td><td><button class="btn btn--sm" ${on ? 'style="border-color:var(--focus)"' : ''}>Change</button></td></tr>`;
  const pop = `<div class="menu" style="right:0;left:auto;top:212px;width:380px;padding:var(--s-3);display:flex;flex-direction:column;gap:var(--s-3)">
      <div style="display:flex;align-items:center;gap:var(--s-2)"><span class="site-ico">N</span><span style="font-weight:var(--weight-medium)">Notion</span><span style="flex:1"></span><span style="color:var(--text-muted);font-size:var(--text-sm)" title="Settings › Preferences › Password generator">20 characters</span></div>
      <div class="input is-focus" title="Edit it by hand if you want"><span class="grow code">k7#Vq2!mWz9@pLx4&amp;Rt8</span><button class="icon-btn icon-btn--sm" aria-label="Shuffle" title="Another one">${i('refresh')}</button></div>
      <div style="display:flex;justify-content:flex-end;gap:var(--s-2)"><button class="btn btn--sm">Cancel</button><button class="btn btn--primary btn--sm" title="The current password goes to the item's previous passwords">Apply</button></div>
    </div>`;
  return shell('Health', `
    <div class="settings-body" style="padding-top:var(--s-6)"><div class="page" style="max-width:1100px;position:relative">
      <div class="page-head"><h1>Health report</h1><div class="input select" style="width:170px"><span class="grow">All spaces</span></div><button class="btn">${i('refresh')}Scan</button></div>
      <div class="grid-3" style="grid-template-columns:repeat(5,1fr)">
        <div class="section stat danger"><b>4</b><span>Weak</span></div>
        <div class="section stat warn"><b>6</b><span>Reused</span></div>
        <div class="section stat warn"><b>9</b><span>Over a year old</span></div>
        <div class="section stat"><b>5</b><span title="The service offers two-step verification">No 2FA</span></div>
        <div class="section stat ok"><b>62</b><span>No issue</span></div>
      </div>
      <div class="section" style="padding:0;overflow:hidden">
        <table class="table"><thead><tr><th>Item</th><th>Issues</th><th style="width:140px">Space</th><th style="width:160px">Last changed</th><th style="width:100px"></th></tr></thead><tbody>
          ${r('N', 'Notion', pb('Weak', 'chip--danger') + pb('Reused ×3', 'chip--warn'), 'General', 'Mar 14, 2024', true)}
          ${r('S', 'Slack', pb('Reused ×3', 'chip--warn') + pb('Over a year', 'chip--warn'), 'General', 'Feb 2, 2025')}
          ${r('R', 'Office router', pb('Weak', 'chip--danger') + pb('Over a year', 'chip--warn'), 'General', 'Sep 9, 2023')}
          ${r('D', 'Dropbox', pb('Reused ×3', 'chip--warn'), 'General', 'Jun 30, 2025')}
        </tbody></table>
      </div>
      ${pop}
    </div></div>`, { primary: null });
}

// ---------- 44 (27/11/2026): Contacts as the Vault ----------
// The sidebar of Contacts follows the Vault's: "My contacts" and the organization are sections
// (a click lists all of it, + adds a folder), their folders under them. In an organization a
// contact can stay outside any folder: "Uncategorized" lists those. New contact goes where the
// list is (the organization shown, its folder); at the top right, the organization's logo.
function contactsSidebar44(active) {
  const it = (ico, label, count, { on = false, sub = false, tip = '' } = {}) =>
    `<div class="side-item ${on ? 'is-active' : ''} ${sub ? 'sub' : ''}" ${tip ? `title="${tip}"` : ''}>${i(ico)}<span>${label}</span>${count !== undefined ? `<span class="count">${count}</span>` : ''}</div>`;
  const section = (ico, label, count, add, on) => `<div class="side-item side-section ${on ? 'is-active' : ''}">${i(ico)}<span>${label}</span><span class="count">${count}</span><button class="icon-btn icon-btn--sm" aria-label="${add}" title="${add}">${i('plus')}</button></div>`;
  const lbl = (n, c) => `<div class="side-item"><span class="lbl-dot ${LB[n]}"></span><span>${n}</span><span class="count">${c}</span></div>`;
  return `<aside class="sidebar pane">
    ${it('users', 'All contacts', 148, { on: active === 'All' })}${it('star', 'Favorites', 12)}${it('calendar', 'Birthdays soon', 2, { tip: 'Next 30 days' })}
    <div style="height:var(--s-3)"></div>
    ${section('user', 'My contacts', 37, 'New folder', active === 'Mine')}
    ${it('folder', 'Family', 12, { sub: true })}
    <div style="height:var(--s-2)"></div>
    ${section('users', 'Example Studio', 111, 'New folder', active === 'Org')}
    ${it('list', 'Uncategorized', 22, { sub: true, on: active === 'Uncategorized', tip: 'Contacts of Example Studio outside any folder' })}
    ${it('folder', 'Clients', 42, { sub: true })}${it('folder', 'Suppliers', 31, { sub: true, on: active === 'Suppliers' })}${it('folder', 'Advisors', 9, { sub: true })}${it('folder', 'Team', 7, { sub: true })}
    <div class="side-group"><span>Labels</span><button class="icon-btn icon-btn--sm" aria-label="New label" title="New label">${i('plus')}</button></div>
    ${lbl('VIP', 6)}${lbl('Prospect', 14)}${lbl('Freelancer', 11)}
    <div style="flex:1"></div>
    <div class="side-item" title="Possible duplicates">${i('users')}<span>Merge & fix</span><span class="count badge">3</span></div>
    <div class="side-item" title="vCard, CSV">${i('upload')}<span>Import</span></div>
  </aside>`;
}
function contacts44(active) {
  const logo = active === 'All' || active === 'Mine' ? '' : orgLogo(24);
  let html = shell('Contacts', `
    <div class="work">${contactsSidebar44(active)}
      <section class="center">
        <div class="toolbar">
          <div class="input search">${i('search')}<span class="grow ph">Search name, company, phone</span><span style="color:var(--text-faint);font-size:var(--text-sm)">Ctrl+K</span></div>
          <button class="btn btn--ghost">${i('filter')}Filter</button>
          <button class="btn btn--ghost">${i('columns')}Columns</button>
          <div style="flex:1"></div>
          <div class="segmented" role="radiogroup" aria-label="View"><button role="radio" aria-checked="true" title="Table">${i('list')}</button><button role="radio" aria-checked="false" title="Cards">${i('columns')}</button></div>
        </div>
        <div class="table-wrap">${contactsTable(null)}</div>
      </section>
    </div>
    <footer class="statusbar"><span>${active === 'Uncategorized' ? 'Example Studio › Uncategorized' : active === 'Org' ? 'Example Studio' : 'All contacts'}</span><span class="sp"></span><span class="sb-item">${i('shield')}Standard encryption</span></footer>`,
    { primary: `${i('plus')}New contact` });
  if (logo) html = html.replace(/<span class="avatar"[^>]*>LM<\/span>/, logo);
  return html;
}
function contactsOrg() { return contacts44('Org'); }
function contactsUncategorized() { return contacts44('Uncategorized'); }

// ---------- 45 (27/11/2026): sidebar shortcuts as a grid, not pills ----------
// One row per shortcut (a new one is a new row), one column per vault (or "Every vault" when the
// same set serves all), a check box in each cell. Contacts get their own block, the same way.
function raccourcisGrille(per = true) {
  const ck = (on) => `<span class="check ${on ? 'on' : ''}" style="margin:auto">${on ? i('check') : ''}</span>`;
  const cols = per ? [['user', 'My vault'], ['users', 'Example Studio']] : [['list', 'Every vault']];
  const head = (first) => `<div style="display:grid;grid-template-columns:1fr repeat(${cols.length},128px);align-items:center;height:32px;color:var(--text-muted);font-size:var(--text-xs);font-weight:var(--weight-medium);border-bottom:1px solid var(--line)"><span>${first}</span>${cols.map(([ic, n]) => `<span style="display:flex;align-items:center;justify-content:center;gap:6px">${i(ic)}${n}</span>`).join('')}</div>`;
  const line = (ic, label, vals, tip = '') => `<div style="display:grid;grid-template-columns:1fr repeat(${vals.length},128px);align-items:center;height:36px;border-bottom:1px solid var(--line)" ${tip ? `title="${tip}"` : ''}><span style="display:flex;align-items:center;gap:8px">${i(ic)}${label}</span>${vals.map(ck).join('')}</div>`;
  const v = (a, b) => (per ? [a, b] : [a]);
  const headC = `<div style="display:grid;grid-template-columns:1fr 128px;align-items:center;height:32px;color:var(--text-muted);font-size:var(--text-xs);font-weight:var(--weight-medium);border-bottom:1px solid var(--line)"><span>Contacts</span><span style="display:flex;justify-content:center">Shown</span></div>`;
  return settings('Preferences', `
    <div class="page-head"><h1>Preferences</h1></div>
    <div class="section">${sectionTitle('Vaults')}
      <div class="row"><span class="row-label">Personal vault in its own tab</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label" title="At the top of the sidebar">Shortcuts</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Same for every vault</span><span class="switch ${per ? '' : 'on'}"></span></div>
      <div style="padding-left:var(--s-4)">
        ${head('Vault')}
        ${line('list', 'All items', v(true, true))}
        ${line('star', 'Favorites', v(true, false))}
        ${line('clock', 'Renew soon', v(false, true), 'Subscriptions renewing in the next 30 days')}
        <div style="height:var(--s-3)"></div>
        ${headC}
        ${line('users', 'All contacts', [true])}
        ${line('star', 'Favorites', [true])}
        ${line('calendar', 'Birthdays soon', [true], 'Next 30 days')}
        ${line('list', 'Uncategorized', [true], 'Contacts outside any folder')}
      </div>
    </div>`);
}
function raccourcisGrilleMeme() { return raccourcisGrille(false); }

// 38. Settings › Shortcuts (27/11/2026): every keyboard shortcut in one place, each one changed by
// pressing the new keys. The global ones move here from Autofill (one place, not two).
function raccourcisClavier() {
  const row = (label, k, { tip = '', changed = false, rec = false, clash = '', sw = false } = {}) => `<div class="row"><span class="row-label" ${tip ? `title="${tip}"` : ''}>${label}</span>
    ${clash ? `<span class="chip chip--warn" title="${clash}"><span class="dot"></span>In use</span>` : ''}
    ${rec ? `<span class="input input--sm is-focus" style="width:150px;justify-content:center;color:var(--text-muted)" title="Esc: cancel · Backspace: none">Press the keys…</span>` : keys(...k)}
    ${changed ? `<button class="icon-btn icon-btn--sm" aria-label="Reset" title="Back to ${changed}">${i('refresh')}</button>` : '<span style="width:24px"></span>'}
    <button class="btn btn--ghost btn--sm" style="width:64px;justify-content:center">${rec ? 'Cancel' : 'Change'}</button>${sw ? '<span class="switch on"></span>' : '<span style="width:26px;flex:none"></span>'}</div>`;
  const nav = settingsNav('Shortcuts').replace(`<span>Sync & backups</span></div>`, `<span>Sync & backups</span></div><div class="side-item is-active">${i('keyboard')}<span>Shortcuts</span></div>`);
  const body = `
    <div class="page-head"><h1>Shortcuts</h1><span style="flex:1"></span><button class="btn btn--ghost btn--sm" title="Every shortcut back to Cleis's">Reset all</button></div>
    <div class="section">${sectionTitle('Everywhere', 'Work in any app, even when Cleis is in the background')}
      ${row('Quick Access', ['Ctrl', 'Shift', 'Space'], { sw: true })}
      ${row('Fill best match', ['Ctrl', 'Alt', 'A'], { tip: 'Fills the item linked to the window in front', sw: true })}
    </div>
    <div class="section">${sectionTitle('In Cleis')}
      ${row('Lock', ['Ctrl', 'L'])}
      ${row('New password', ['Ctrl', 'N'], { tip: 'New contact in Contacts' })}
      ${row('Search', ['Ctrl', 'K'], { changed: 'Ctrl+F' })}
    </div>
    <div class="section">${sectionTitle('Selected item')}
      ${row('Copy username', ['Ctrl', 'C'])}
      ${row('Copy password', [], { rec: true })}
      ${row('Copy 2FA code', ['Ctrl', 'Alt', 'C'])}
      ${row('Duplicate', ['Ctrl', 'D'])}
      ${row('Move to trash', ['Del'])}
    </div>`;
  return shell('Settings', `<div class="settings">${nav}<div class="settings-body"><div class="page">${body}</div></div></div>`, { primary: null });
}
// 38b. A shortcut already taken: both rows say so, and the one just set waits until the other changes.
function raccourciConflit() {
  return raccourcisClavier()
    .replace(/(<span class="row-label" >Lock<\/span>\s*)/, '$1<span class="chip chip--warn" title="Also Copy password"><span class="dot"></span>In use</span>')
    .replace(/(<span class="row-label" >Copy password<\/span>\s*)/, '$1<span class="chip chip--warn" title="Also Lock: change one of them"><span class="dot"></span>In use</span>')
    .replace(/<span class="input input--sm is-focus"[^>]*>Press the keys…<\/span>/, keys('Ctrl', 'L'))
    .replace('>Cancel</button>', '>Change</button>');
}

// ---------- 46 (27/11/2026): profile from the avatar, website link, shortcut boxes, space members ----------
// 46a. The avatar always opens its menu, signed in or not: the profile is Cleis's own (name, e-mail,
// photo), the one every organization shows of you and writes in its activity log.
const meAv = (size = 24, photo = true) => `<span class="avatar" style="width:${size}px;height:${size}px;${photo ? 'background:linear-gradient(135deg,#6b8f71,#3d5a80);color:#fff' : ''}">LM</span>`;
function profilMenu({ github = false } = {}) {
  const head = `<div class="member" style="padding:var(--s-2) var(--s-2) var(--s-3)">${meAv(32)}<div class="meta"><span style="font-weight:var(--weight-medium)">Lea Martin</span><span class="sub">${github ? '@lea-example · Synced 2 min ago' : 'Local account'}</span></div></div>`;
  const menu = `<div class="menu" style="right:132px;top:44px;width:260px">${head}${sep}
      ${mi('user', 'Edit profile', '', { hover: true })}${github ? mi('sync', 'Sync now') : mi('cloud', 'Sign in to sync…')}${mi('gear', 'Account')}${sep}${github ? mi('logout', 'Sign out') : mi('lock', 'Lock', 'Ctrl+L')}</div>`;
  const html = shell('Vault', `<div class="work">${sidebar()}<section class="center">${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div></section></div>${statusbar()}`, { overlay: menu });
  return html.replace(/<span class="avatar" title="Lea Martin">LM<\/span>/, `<span style="border-radius:99px;outline:1px solid var(--focus);outline-offset:1px;display:inline-grid">${meAv()}</span>`);
}
function profilMenuGithub() { return profilMenu({ github: true }); }
// 46b. Edit profile: a card under the avatar. The photo is the button (hover: upload sign). Saved
// once for Cleis and sent to each organization; their logos show where it goes.
function profilEditer() {
  const orgs = `<span style="display:inline-flex;gap:4px" title="Example Studio, Atelier Nord">${orgLogo(20)}<span style="width:20px;height:20px;border-radius:var(--radius-md);display:inline-grid;place-items:center;background:#4a3f35;color:#f0d9b5;font-size:10px;font-weight:600">AN</span></span>`;
  const card = `<div class="menu" style="right:132px;top:44px;width:320px;padding:var(--s-4);display:flex;flex-direction:column;gap:var(--s-3)">
      <div style="display:flex;align-items:center;gap:var(--s-3)">
        <span style="position:relative;display:inline-grid" title="Change the photo">${meAv(56)}<span style="position:absolute;inset:0;border-radius:99px;background:rgba(0,0,0,.45);display:grid;place-items:center;color:#fff">${i('upload')}</span></span>
        <div style="display:flex;flex-direction:column;gap:2px;flex:1"><span style="font-weight:var(--weight-medium)">Profile</span><span style="display:flex;align-items:center;gap:6px;color:var(--text-muted);font-size:var(--text-sm)">Shown to ${orgs}</span></div>
      </div>
      ${fld('Name', 'Lea Martin', true)}${fld('E-mail', 'lea@example-studio.com')}
      <div style="display:flex;justify-content:flex-end;gap:var(--s-2)"><button class="btn btn--sm">Cancel</button><button class="btn btn--primary btn--sm">Save</button></div></div>`;
  const html = shell('Vault', `<div class="work">${sidebar()}<section class="center">${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div></section></div>${statusbar()}`, { overlay: card });
  return html.replace(/<span class="avatar" title="Lea Martin">LM<\/span>/, `<span style="border-radius:99px;outline:1px solid var(--focus);outline-offset:1px;display:inline-grid">${meAv()}</span>`);
}
// 46c. Website column: the open-link button next to the address, like copy next to the password.
// Both cell buttons get a stronger hover (a darker-than-the-row fill, the icon brightened; no
// outline, no colour) to read as buttons on a highlighted row; the icon stays centred.
const strong = 'background:var(--surface-active);color:var(--text)';
function siteColonne() {
  const sites = { OVHcloud: 'example-ovh.com', Cloudflare: 'example-cloudflare.com', Figma: 'example-figma.com', 'Adobe Creative Cloud': 'example-adobe.com', Notion: 'example-notion.com', 'Google Workspace': 'example-google.com', 'Microsoft 365': 'example-office.com', 'Bank — business portal': 'example-bank.com', GitHub: 'example-github.com', Webflow: 'example-webflow.com' };
  const rows = ROWS.filter((r) => sites[r.n]).map((r) => {
    const hov = r.n === 'Figma', sel = r.n === 'Notion';
    const b = (ico, tip, on = false) => `<button class="icon-btn icon-btn--sm" style="${on ? strong : ''};${hov || sel ? '' : 'visibility:hidden'}" title="${tip}">${i(ico)}</button>`;
    const totp = r.totp ? `<span class="totp"><span class="ring ${r.p < 25 ? 'low' : ''}" style="--p:${r.p}%"></span>${r.totp}</span>` : '<span style="color:var(--text-faint)">—</span>';
    return `<tr class="${hov ? 'is-hover' : ''} ${sel ? 'is-selected' : ''}"><td><span class="site"><span class="site-ico">${r.c}</span><span class="site-name">${r.n}</span></span></td>
      <td class="muted"><span class="cellflex"><span class="ell">${r.u}</span>${b('copy', 'Copy username')}</span></td>
      <td><span class="cellflex"><span class="dots ell">••••••</span>${b('copy', 'Copy password', hov)}</span></td>
      <td class="muted"><span class="cellflex"><span class="ell">${sites[r.n]}</span>${b('external', `Open ${sites[r.n]}`)}</span></td>
      <td>${totp}</td></tr>`;
  }).join('');
  const table = `<table class="table table--fixed"><thead><tr><th style="width:200px" class="sorted">Name <span class="sort">↑</span></th><th>Username</th><th style="width:116px">Password</th><th style="width:200px">Website</th><th style="width:100px">2FA code</th></tr></thead><tbody>${rows}</tbody></table>`;
  const zoom = `<div style="position:absolute;right:24px;bottom:56px;display:flex;gap:var(--s-4);align-items:center;padding:var(--s-3) var(--s-4);background:var(--surface);border:1px solid var(--line);border-radius:var(--radius-lg);font-size:var(--text-sm);color:var(--text-muted)">
      <span>Rest</span><button class="icon-btn icon-btn--sm">${i('external')}</button>
      <span>Hover</span><button class="icon-btn icon-btn--sm" style="${strong}">${i('external')}</button></div>`;
  return shell('Vault', `<div class="work">${sidebar()}<section class="center">${vaultToolbar()}<div class="table-wrap">${table}</div></section></div>${statusbar()}`, { overlay: zoom });
}
// 46d. Shortcuts: the keys sit in a box that reads as a field; a click on it records, Esc cancels,
// Backspace clears. No "Change" button.
function raccourcisCases() {
  const box = (k, { hover = false, rec = false, none = false } = {}) => `<span class="input input--sm ${rec ? 'is-focus' : ''}" style="width:168px;height:28px;justify-content:center;cursor:pointer;${hover ? 'border-color:var(--text-faint);background:var(--surface-hover)' : ''}" title="${rec ? 'Esc: cancel · Backspace: none' : 'Click, then press the new keys'}">${rec ? '<span style="color:var(--text-muted);font-size:var(--text-sm)">Press the keys…</span>' : none ? '<span class="ph" style="font-size:var(--text-sm)">None</span>' : keys(...k)}</span>`;
  const row = (label, k, { tip = '', changed = '', sw = null, ...o } = {}) => `<div class="row"><span class="row-label" ${tip ? `title="${tip}"` : ''}>${label}</span>
    ${changed ? `<button class="icon-btn icon-btn--sm" title="Back to ${changed}">${i('refresh')}</button>` : '<span style="width:24px"></span>'}
    ${box(k, o)}${sw === null ? '<span style="width:26px;flex:none"></span>' : `<span class="switch ${sw ? 'on' : ''}"></span>`}</div>`;
  const nav = settingsNav('Shortcuts').replace(`<span>Sync & backups</span></div>`, `<span>Sync & backups</span></div><div class="side-item is-active">${i('keyboard')}<span>Shortcuts</span></div>`);
  const body = `
    <div class="page-head"><h1>Shortcuts</h1><span style="flex:1"></span><button class="btn btn--ghost btn--sm" title="Every shortcut back to Cleis's">Reset all</button></div>
    <div class="section">${sectionTitle('Everywhere', 'Work in any app, even when Cleis is in the background')}
      ${row('Quick Access', ['Ctrl', 'Shift', 'Space'], { sw: true })}
      ${row('Fill best match', ['Ctrl', 'Alt', 'A'], { tip: 'Fills the item linked to the window in front', sw: true })}
    </div>
    <div class="section">${sectionTitle('In Cleis')}
      ${row('Lock', ['Ctrl', 'L'])}
      ${row('New password', ['Ctrl', 'N'], { tip: 'New contact in Contacts', hover: true })}
      ${row('Search', ['Ctrl', 'K'], { changed: 'Ctrl+F' })}
    </div>
    <div class="section">${sectionTitle('Selected item')}
      ${row('Copy username', ['Ctrl', 'C'])}
      ${row('Copy password', [], { rec: true })}
      ${row('Copy 2FA code', [], { none: true, changed: 'Ctrl+Alt+C' })}
      ${row('Duplicate', ['Ctrl', 'D'])}
      ${row('Move to trash', ['Del'])}
    </div>`;
  return shell('Settings', `<div class="settings">${nav}<div class="settings-body"><div class="page">${body}</div></div></div>`, { primary: null });
}
// 46f. Spaces: a click on a space unfolds its members and what each may do in it: Access, Edit
// (add, change, trash), Show passwords (without: copy and fill only). The role set on the server
// is the ceiling: a space narrows it, never widens it (a box the role does not allow is greyed,
// its tip says why). Administrators: everything, greyed. Reserved space: Super Admin only.
function espacesMembres() {
  const av = (xs) => `<span style="display:inline-flex">${xs.map((x, k) => `<span class="avatar avatar--sm" style="margin-left:${k ? -6 : 0}px">${x}</span>`).join('')}</span>`;
  const r = (n, items, ms, mode, open = false) => `<tr ${open ? 'style="background:var(--surface-hover)"' : ''}><td><span class="site"><span style="display:inline-grid;transform:rotate(${open ? 0 : -90}deg);color:var(--text-muted)">${i('chevron')}</span>${i('folder')}<span>${n}</span></span></td><td class="num muted">${items}</td><td>${av(ms)} <span class="muted" style="margin-left:6px">${ms.length}</span></td><td>${mode === 'e' ? '<span class="chip chip--ok">End-to-end</span>' : '<span class="chip">Standard</span>'}</td>
    <td style="text-align:right"><button class="icon-btn icon-btn--sm" title="Rename">${i('text')}</button><button class="icon-btn icon-btn--sm" title="Delete">${i('trash')}</button></td></tr>`;
  const grid = 'display:grid;grid-template-columns:28px 1fr 96px 96px 128px;align-items:center;gap:var(--s-3);padding:0 var(--s-4) 0 56px';
  // v: [access, edit, show]; each true / false / 'x' (not allowed by the role: greyed, empty).
  const ck = (v, tip = '', fixed = false) => `<span style="display:flex;justify-content:center" ${tip ? `title="${tip}"` : ''}><span class="check ${v === true ? 'on' : ''}" style="${fixed || v === 'x' ? 'opacity:.4' : 'cursor:pointer'}">${v === true ? i('check') : ''}</span></span>`;
  const m = (ini, name, v, { admin = false, role = '', hover = false } = {}) => `<div style="${grid};height:40px;${hover ? 'background:var(--surface-hover)' : ''}">
      <span class="avatar">${ini}</span><span>${name}</span>
      ${ck(v[0], admin ? 'Administrators see every space' : '', admin)}
      ${ck(v[0] === false ? 'x' : v[1], admin ? '' : v[1] === 'x' ? `Not allowed by the role ${role}` : v[0] === false ? 'No access' : '', admin)}
      ${ck(v[0] === false ? 'x' : v[2], admin ? '' : v[2] === 'x' ? `Not allowed by the role ${role}` : v[0] === false ? 'No access' : 'Without: copy and fill only', admin)}</div>`;
  const open = `<tr><td colspan="5" style="padding:0;border-bottom:1px solid var(--line)">
      <div style="${grid};height:28px;color:var(--text-muted);font-size:var(--text-xs);font-weight:var(--weight-medium)"><span></span><span>Member</span><span style="text-align:center">Access</span><span style="text-align:center" title="Add, change, move to the trash">Edit</span><span style="text-align:center" title="Without: copy and fill only">Show passwords</span></div>
      ${m('LM', 'Lea Martin', [true, true, true], { admin: true })}
      ${m('IP', 'Ines Petit', [true, true, true], { admin: true })}
      ${m('HB', 'Hugo Bernard', [true, true, false])}
      ${m('SR', 'Sam Roux', [true, false, true], { hover: true })}
      ${m('TD', 'Tom Dubois', [true, 'x', 'x'], { role: 'Intern' })}
      ${m('ZM', 'Zoe Moreau', [false, false, false])}
      <div style="height:var(--s-2)"></div></td></tr>`;
  return settings('Spaces', `
    <div class="page-head"><h1>Spaces</h1><button class="btn btn--primary">${i('plus')}New space</button></div>
    <div class="section" style="padding:0;overflow:hidden">
      <table class="table table--roomy"><thead><tr><th>Space</th><th class="num" style="width:80px">Items</th><th>Members</th><th style="width:140px">Encryption</th><th style="width:90px"></th></tr></thead><tbody>
        ${r('General', 41, ['LM', 'HB', 'IP', 'SR'], 's')}${r('Design', 18, ['LM', 'IP', 'HB', 'SR', 'TD'], 's', true)}${open}${r('Accounting', 12, ['LM', 'IP'], 'e')}${r('Dev', 9, ['LM', 'HB'], 's')}${r('Interns', 6, ['LM', 'TD'], 's')}
      </tbody></table>
    </div>`);
}

// ---------- 47 (27/11/2026): organization place in two choices, trash menu, recovery keys ----------
// 47a–47c. Create an organization: two choices, none chosen at first (Server, Shared folder); the
// server's place is chosen inside it (NAS, Linux server over SSH, this computer). No sentence under
// a choice: what it means is in its tooltip.
const navOrg = settingsNav('Organization', false).replace('</nav>', `<div class="side-item is-active">${i('server')}<span>Organization</span></div></nav>`);
const settingsOrg = (body) => shell('Settings', `<div class="settings">${navOrg}<div class="settings-body"><div class="page">${body}</div></div></div>`, { primary: null });
function orgCreer(which = '', where = 'NAS') {
  const ch = (ico, name, tip, on, extra = '') => `<div class="choice ${on ? 'is-active' : ''}" title="${tip}"><span class="radio ${on ? 'on' : ''}"></span><div class="txt"><b style="font-weight:var(--weight-medium)">${name}</b>${extra}</div><span class="ico">${i(ico)}</span></div>`;
  const seg = (opts) => `<div class="segmented" role="radiogroup">${opts.map(([ico, n, tip]) => `<button role="radio" aria-checked="${n === where}" title="${tip}">${i(ico)}${n}</button>`).join('')}</div>`;
  const ssh = (host, user) => `<div style="display:flex;gap:12px"><div style="flex:1">${fld(where === 'NAS' ? 'NAS address' : 'Server address', host, true)}</div><div style="width:90px">${fld('SSH port', '22')}</div></div>
    <div style="display:flex;gap:12px"><div style="flex:1">${fld('Administrator', user)}</div><div style="flex:1"><div class="field"><span class="label">Password</span><div class="input"><span class="grow dots">••••••••</span></div></div></div></div>
    <div class="row" style="padding:0" title="Docker (Synology Container Manager, QNAP Container Station) or a Linux service"><span class="row-label" style="color:var(--text-muted)">Found</span><span class="chip chip--ok"><span class="dot"></span>${where === 'NAS' ? 'QNAP · QTS 5.2' : 'Debian 13 · systemd'}</span></div>`;
  const here = `<div class="row" style="padding:0"><span class="row-label" title="Reachable by the other members only while this computer is on">This computer</span><span class="chip">PC-STUDIO</span></div>`;
  const server = `<div style="display:flex;flex-direction:column;gap:var(--s-3);margin-top:var(--s-3)">
      <div class="row" style="padding:0"><span class="row-label">Where</span>${seg([['server', 'NAS', 'Cleis installs the server on your NAS over SSH (QNAP, Synology, other Linux NAS)'], ['window', 'Linux server', 'Any Linux machine you reach over SSH: a VPS, a mini PC…'], ['laptop', 'This computer', 'The server runs here; members reach it only while this computer is on']])}</div>
      ${where === 'This computer' ? here : ssh(where === 'NAS' ? 'nas.example-studio.lan' : 'vault.example-studio.com', where === 'NAS' ? 'admin' : 'root')}
    </div>`;
  const place = (ico, name, tip, on, value, action) => `<div class="row" style="padding:var(--s-2) 0" title="${tip}"><span class="radio ${on ? 'on' : ''}"></span><span class="site-ico">${i(ico)}</span><span class="row-label">${name}${value ? `<span class="code" style="margin-left:var(--s-2);color:var(--text-muted);font-size:var(--text-sm)">${value}</span>` : ''}</span>${action}</div>`;
  const folder = `<div style="margin-top:var(--s-3);border:1px solid var(--line-strong);border-radius:var(--radius-md);padding:0 var(--s-3);background:var(--bg)">
      ${place('cloud', 'SharePoint', 'Folder kept by OneDrive for the SharePoint library', true, 'C:\\Users\\lea\\Example Studio\\Cleis', '<button class="btn btn--sm">Change</button>')}
      ${place('cloud', 'Google shared drive', 'Folder kept by Google Drive for desktop', false, '', '<button class="btn btn--sm">Choose folder</button>')}
      ${place('folder', 'NAS share', 'A network folder (\\\\nas\\cleis)', false, '', '<button class="btn btn--sm">Choose folder</button>')}
      ${place('server', 'SFTP', 'A folder on an SSH server', false, '', '<button class="btn btn--sm">Set up</button>')}
    </div>`;
  const go = which === 'server' ? (where === 'This computer' ? 'Create' : 'Install and create') : 'Create';
  return settingsOrg(`
    <div class="page-head"><h1>Organization</h1></div>
    <div class="section">${sectionTitle('Create an organization')}
      <div style="display:flex;gap:12px"><div style="flex:1">${fld('Name', 'Example Studio')}</div><div style="flex:1">${fld('Your name', 'Lea Martin')}</div></div>
      ${ch('server', 'Server', 'Access changes at once; members reach it over the network or your VPN', which === 'server', which === 'server' ? server : '')}
      ${ch('folder', 'Shared folder', 'No server: SharePoint, Google shared drive, NAS share or SFTP; access changes at the next sync', which === 'folder', which === 'folder' ? folder : '')}
      <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary" ${which ? '' : 'disabled style="opacity:.45" title="Choose Server or Shared folder"'}>${go}</button></div>
    </div>
    <div class="section">${sectionTitle('Join an organization')}
      <div style="display:flex;gap:12px"><div style="flex:1">${fld('Server address', '<span class="ph">vault.example-studio.lan</span>')}</div></div>
    </div>`);
}
function orgServeur() { return orgCreer('server', 'NAS'); }
function orgServeurIci() { return orgCreer('server', 'This computer'); }
function orgDossier() { return orgCreer('folder'); }

// 47e. Right-click on Trash: Empty trash… (asks first, as the button on the Trash page).
function corbeilleMenu() {
  const side = personalSidebar().replace(`<div class="side-item ">${i('trash')}<span>Trash</span>`, `<div class="side-item" style="box-shadow:inset 0 0 0 1px var(--focus);border-radius:var(--radius-sm)">${i('trash')}<span>Trash</span><span class="count">5</span>`);
  const m = ctx(24, 700, `${mi('trash', 'Open')}${sep}${mi('trash', 'Empty trash…', '', { danger: true, hover: true })}`, 200);
  return shell('Vault', `<div class="work">${side}<section class="center">${vaultToolbar()}<div class="table-wrap">${vaultTable({ selected: null })}</div></section></div>${statusbar()}`, { overlay: m, doc: 'My vault' });
}

// 47f. Recovery keys: a column (left / total, orange at 2 or fewer) and a section of the panel
// under the 2FA key: one line per code, masked until shown, copy on hover, a box to mark it used
// (struck through). "+" pastes the codes a site gives, one per line or separated by spaces.
function cleRecuperation(adding = false) {
  const codes = [['4f9k-2xq8', false], ['a8d2-k1m3', true], ['p7wc-9ze4', false], ['m2rt-6hv1', false], ['x5ne-0bq7', true], ['c3jy-8lk2', false]];
  const shown = adding;
  const line = ([c, used], k) => `<div style="display:flex;align-items:center;gap:var(--s-2);height:26px;padding:0 4px;border-radius:var(--radius-sm);${k === 2 ? 'background:var(--surface-hover)' : ''}">
      <span class="check ${used ? 'on' : ''}" title="${used ? 'Used' : 'Mark as used'}">${used ? i('check') : ''}</span>
      <span class="code" style="flex:1;font-size:var(--text-sm);${used ? 'text-decoration:line-through;color:var(--text-faint)' : ''}">${shown || k === 2 ? c : '••••-••••'}</span>
      ${k === 2 ? `<button class="icon-btn icon-btn--sm" style="background:var(--surface-active);color:var(--text)" title="Copy">${i('copy')}</button>` : '<span style="width:24px"></span>'}</div>`;
  const add = adding ? `<div class="field"><div class="input is-focus" style="height:auto;min-height:64px;align-items:flex-start;padding:6px 8px;font-family:var(--font-mono);font-size:var(--text-sm)">r4tq-1kd9 v8bn-3sz0<br>h6wl-5ce2 j1ux-7pa4</div></div>
      <div style="display:flex;justify-content:flex-end;gap:var(--s-2)"><button class="btn btn--sm">Cancel</button><button class="btn btn--primary btn--sm" title="4 codes found">Add 4</button></div>` : '';
  const section = `<div class="section" style="padding:var(--s-3);gap:var(--s-2);background:var(--surface-raised)">
      <div style="display:flex;align-items:center;gap:var(--s-2)"><h3 style="flex:1">Recovery keys</h3><span class="chip" title="Unused codes left">4 / 6</span>
        <button class="icon-btn icon-btn--sm" title="Show the codes">${i('eye')}</button><button class="icon-btn icon-btn--sm" title="Add codes">${i('plus')}</button></div>
      ${codes.map(line).join('')}${add}</div>`;
  const insp = inspector().replace(/(<div class="field"><span class="label">Notes<\/span>)/, `${section}$1`);
  const rows = vaultTable({ selected: 'Figma' })
    .replace('<th class="num" style="width:100px" title="Price × licenses, per year">Yearly cost</th>', '<th style="width:120px" title="Unused codes left / codes kept">Recovery keys</th>');
  // The Yearly cost column becomes Recovery keys in this mockup (same width, fewer columns).
  const rk = { OVHcloud: '8 / 10', Cloudflare: '—', Figma: '4 / 6', 'Adobe Creative Cloud': '—', Notion: '—', 'Google Workspace': '2 / 10', 'Microsoft 365': '—', 'Bank — business portal': '—', GitHub: '16 / 16', Webflow: '—', Slack: '—', 'Office router': '—', Canva: '—', Dropbox: '—' };
  let html = rows;
  for (const r of ROWS) {
    const v = rk[r.n];
    const cell = v === '—' ? '<span style="color:var(--text-faint)">—</span>' : (parseInt(v) <= 2 ? `<span class="chip chip--warn" title="2 unused codes left">${v}</span>` : `<span class="num" style="color:var(--text-muted)">${v}</span>`);
    html = html.replace(`<td class="num">${r.cost}</td>\n    </tr>`, `<td>${cell}</td>\n    </tr>`);
  }
  html = html.replace(/<tr><td colspan="6"[\s\S]*?<\/tr><\/tbody>/, '</tbody>');
  return shell('Vault', `<div class="work">${sidebar()}<section class="center">${vaultToolbar()}<div class="table-wrap">${html}</div></section>${insp}</div>${statusbar()}`);
}
function cleRecuperationAjout() { return cleRecuperation(true); }

// ---------- 48 (28/11/2026, 0.2.4): lists by folder, star before the name, scroll bar, search
// clear, type menu, birthday calendar, folder icons, billing period, middle-click scroll, Fill
// best match by the site ----------
// The vault's items with their folder (fictitious).
const FOLDER48 = {
  Dropbox: ['My vault', ''], 'Bank — business portal': ['My vault', 'Banking'], Canva: ['My vault', 'Shopping'],
  OVHcloud: ['Example Studio', 'General'], Cloudflare: ['Example Studio', 'General'], 'Google Workspace': ['Example Studio', 'General'], Slack: ['Example Studio', 'General'],
  Figma: ['Example Studio', 'Design'], 'Adobe Creative Cloud': ['Example Studio', 'Design'], Webflow: ['Example Studio', 'Design'],
  'Microsoft 365': ['Example Studio', 'Accounting'], Notion: ['Example Studio', 'Accounting'],
  GitHub: ['Example Studio', 'Dev'], 'Office router': ['Example Studio', 'Dev'],
};
const FAV48 = ['Figma', 'Notion', 'Google Workspace', 'Canva'];
const ICON48 = { Banking: 'bank', Shopping: 'cart', General: 'folder', Design: 'palette', Accounting: 'receipt', Dev: 'code' };
// A folder's line in the list: its icon and name, the vault before it when the list holds several
// vaults (All items), its count on the right. Not clickable (the sidebar opens the folder).
const sepRow48 = (cols, vault, folder, n, { path = true } = {}) => {
  const name = folder ? (path ? `<span style="color:var(--text-muted)">${vault}</span><span style="color:var(--text-faint);display:inline-grid;transform:rotate(-90deg)">${i('chevron')}</span>${folder}` : folder) : vault;
  return `<tr class="grp"><td colspan="${cols}"><span class="grp-in"><span class="grp-name">${name}</span><span class="grp-n">${n}</span></span></td></tr>`;
};
// The name cell: the star sits in a slot before the name, kept empty for the others (names line up).
const name48 = (r) => `<span class="site"><span class="site-ico">${r.c}</span><span class="star-slot">${FAV48.includes(r.n) ? `<span class="fav" title="Favorite">${i('star')}</span>` : ''}</span><span class="site-name">${r.n}</span></span>`;
function vaultTable48({ only = null, filter = null, selected = null, path = true } = {}) {
  const head = `<tr><th style="width:214px" class="sorted">Name <span class="sort">↑</span></th><th>Username</th><th style="width:116px">Password</th><th style="width:100px">2FA code</th><th style="width:120px">Subscription</th><th class="num" style="width:120px">Price</th><th class="num" style="width:100px">Yearly cost</th></tr>`;
  const order = [['My vault', ''], ['My vault', 'Banking'], ['My vault', 'Shopping'], ['Example Studio', 'General'], ['Example Studio', 'Design'], ['Example Studio', 'Accounting'], ['Example Studio', 'Dev']];
  let body = '';
  let count = 0;
  for (const [v, f] of order) {
    if (only && v !== only) continue;
    const rows = ROWS.filter((r) => FOLDER48[r.n][0] === v && FOLDER48[r.n][1] === f && (!filter || r.n.toLowerCase().includes(filter)));
    if (!rows.length) continue;
    count += rows.length;
    body += sepRow48(7, v, f, rows.length, { path: path && !only });
    body += rows.map((r) => {
      const totp = r.totp ? `<span class="totp"><span class="ring ${r.p < 25 ? 'low' : ''}" style="--p:${r.p}%"></span>${r.totp}</span>` : '<span style="color:var(--text-faint)">—</span>';
      return `<tr class="${r.n === selected ? 'is-selected' : ''}"><td>${name48(r)}</td><td class="muted">${r.u}</td><td><span class="dots">••••••</span></td><td>${totp}</td><td>${statusChip(r.st)}</td><td class="num">${r.price}${r.per ? `<span style="color:var(--text-faint)"> /${r.per}</span>` : ''}</td><td class="num">${r.cost}</td></tr>`;
    }).join('');
  }
  body += `<tr><td colspan="7" style="color:var(--text-muted);font-size:var(--text-sm);border-bottom:0">${count} items</td></tr>`;
  return `<table class="table table--fixed">${'<thead>' + head + '</thead>'}<tbody>${body}</tbody></table>`;
}
// The scroll bar (charter §4: a 2 px line, the visible part as a pill), always there when the list
// is longer than its place; `hover`: under the pointer.
const scrollbar48 = (top, len, hover = false) => `<div class="vbar" title="Drag, or click above or below"><div class="vbar-thumb ${hover ? 'is-hover' : ''}" style="top:${top}%;height:${len}%"></div></div>`;
const search48 = (text = '', hoverX = false) => text
  ? `<div class="input search is-focus">${i('search')}<span class="grow">${text}</span><button class="icon-btn icon-btn--sm ${hoverX ? 'is-hover' : ''}" aria-label="Clear" title="Clear (Esc)" style="margin-right:-6px;${hoverX ? 'background:var(--surface-hover);color:var(--text)' : ''}">${i('x')}</button></div>`
  : `<div class="input search">${i('search')}<span class="grow ph">Search</span><span style="color:var(--text-faint);font-size:var(--text-sm)">Ctrl+K</span></div>`;
const toolbar48 = (search) => vaultToolbar().replace(/<div class="input search">[\s\S]*?Ctrl\+K<\/span><\/div>/, search);
const side48 = (active) => sidebarB({ active })
  .replace(`${i('folder')}<span>Banking</span>`, `${i('bank')}<span>Banking</span>`)
  .replace(`${i('folder')}<span>Shopping</span>`, `${i('cart')}<span>Shopping</span>`)
  .replace(`${i('folder')}<span>Design</span>`, `${i('palette')}<span>Design</span>`)
  .replace(`${i('lock')}<span>Accounting</span>`, `${i('receipt')}<span>Accounting</span>`)
  .replace(`${i('folder')}<span>Dev</span>`, `${i('code')}<span>Dev</span>`)
  .replace('<span>Dev</span><span class="count">3</span>', '<span>Dev</span><span class="count">2</span>')
  .replace('<span>My vault</span><span class="count">3</span>', '<span>My vault</span><span class="count">3</span>')
  .replace('<span>Example Studio</span><span class="count">12</span>', '<span>Example Studio</span><span class="count">11</span>')
  .replace('<span>All items</span><span class="count">15</span>', '<span>All items</span><span class="count">14</span>');

// 48a. All items: each folder starts with its line (vault › folder); the star of a favorite sits
// before its name; the scroll bar on the right.
function listeDossiers() {
  const center = `${toolbar48(search48())}<div class="table-wrap" style="position:relative">${vaultTable48()}${scrollbar48(0, 46)}</div>`;
  return frameB(center, { side: side48('All items') });
}
// 48b. My vault: the folders of this vault only, without the vault's name; the bar under the pointer.
function listeMonCoffre() {
  const center = `${toolbar48(search48())}<div class="table-wrap" style="position:relative">${vaultTable48({ only: 'My vault', selected: 'Canva' })}</div>`;
  return frameB(center, { side: side48('My vault'), inspector: true })
    .replace('<span class="site-ico site-ico--lg">F</span>', '<span class="site-ico site-ico--lg">C</span>')
    .replace('<span style="font-weight:var(--weight-medium);font-size:var(--text-lg)">Figma</span>', '<span style="font-weight:var(--weight-medium);font-size:var(--text-lg)">Canva</span>')
    .replace('<span style="color:var(--text-muted);font-size:var(--text-sm)">Design</span>', '<span style="color:var(--text-muted);font-size:var(--text-sm)">My vault › Shopping</span>')
    .replace('<button class="icon-btn" aria-label="Favorite" title="Favorite">', '<button class="icon-btn" aria-label="Favorite" title="Remove from favorites" style="color:var(--warning-text)">');
}
// 48c. A search typed: × at the right of the box empties it (Esc too); the results keep their folders.
function rechercheEffacer() {
  const center = `${toolbar48(search48('fl', true))}<div class="table-wrap" style="position:relative">${vaultTable48({ filter: 'fl' })}</div>`;
  return frameB(center, { side: side48('All items') });
}

// 48d. All contacts: the same folder lines; star before the name; the Website column shown (its
// switch in Columns now acts on the contacts, not on the vault's Website column).
const CFOLD48 = {
  'Léa Fontaine': ['My contacts', ''], 'Paul Lefèvre': ['My contacts', 'Family'],
  'Sofia Rossi': ['Example Studio', 'Clients'], 'Emma Laurent': ['Example Studio', 'Clients'],
  'Camille Dubois': ['Example Studio', 'Suppliers'], 'Nadia Benali': ['Example Studio', 'Suppliers'], 'Thomas Weber': ['Example Studio', 'Suppliers'],
  'Julien Moreau': ['Example Studio', 'Advisors'], 'Omar Haddad': ['Example Studio', 'Advisors'], 'Marc Girard': ['Example Studio', 'Advisors'],
  'Hugo Bernard': ['Example Studio', 'Team'], 'Ines Petit': ['Example Studio', 'Team'],
};
const SITE48 = { 'Camille Dubois': 'example-print.com', 'Julien Moreau': 'example-legal.com', 'Nadia Benali': 'example-hosting.com', 'Paul Lefèvre': 'example-bakery.com', 'Sofia Rossi': 'example-films.com', 'Marc Girard': 'example-bank.com', 'Emma Laurent': 'example-retail.com', 'Thomas Weber': 'example-print.com' };
function contactsDossiers() {
  const order = [['My contacts', ''], ['My contacts', 'Family'], ['Example Studio', 'Clients'], ['Example Studio', 'Suppliers'], ['Example Studio', 'Advisors'], ['Example Studio', 'Team']];
  let body = '';
  for (const [v, f] of order) {
    const rows = PEOPLE.filter((c) => CFOLD48[c.n][0] === v && CFOLD48[c.n][1] === f);
    body += sepRow48(6, v, f, rows.length).replace(i('user'), i('user'));
    body += rows.map((c) => `<tr class="${c.n === 'Camille Dubois' ? 'is-selected' : ''}">
      <td><span class="site"><span class="avatar avatar--sm">${initials(c.n)}</span><span class="star-slot">${c.fav ? `<span class="fav">${i('star')}</span>` : ''}</span><span class="site-name">${c.n}</span></span></td>
      <td>${c.co}</td><td class="muted">${c.e}</td><td class="muted">${c.ph}</td>
      <td class="muted">${SITE48[c.n] ? `<span class="cellflex"><span class="ell">${SITE48[c.n]}</span>${c.n === 'Camille Dubois' ? `<span class="cell-acts"><button class="icon-btn icon-btn--sm" title="Open">${i('external')}</button></span>` : ''}</span>` : '<span style="color:var(--text-faint)">—</span>'}</td>
      <td>${c.lb.map((l) => `<span class="chip ${LB[l]}" style="margin-right:4px">${l}</span>`).join('')}</td></tr>`).join('');
  }
  body += '<tr><td colspan="6" style="color:var(--text-muted);font-size:var(--text-sm);border-bottom:0">12 contacts</td></tr>';
  const table = `<table class="table table--fixed"><thead><tr><th style="width:214px" class="sorted">Name <span class="sort">↑</span></th><th style="width:140px">Company</th><th>Email</th><th style="width:150px">Phone</th><th style="width:160px">Website</th><th style="width:150px">Labels</th></tr></thead><tbody>${body}</tbody></table>`;
  return shell('Contacts', `
    <div class="work work--no-inspector">${contactsSidebar44('All')}
      <section class="center">
        <div class="toolbar">${search48().replace('Search', 'Search name, company, phone')}
          <button class="btn btn--ghost">${i('filter')}Filter</button><button class="btn btn--ghost">${i('columns')}Columns</button><div style="flex:1"></div>
          <div class="segmented" role="radiogroup" aria-label="View"><button role="radio" aria-checked="true" title="Table">${i('list')}</button><button role="radio" aria-checked="false" title="Cards">${i('columns')}</button></div></div>
        <div class="table-wrap" style="position:relative">${table}${scrollbar48(0, 38)}</div>
      </section>
    </div>
    <footer class="statusbar"><span>All contacts</span><span class="sp"></span><span class="sb-item">${i('shield')}Standard encryption</span></footer>`, { primary: `${i('plus')}New contact` });
}

// 48e. The type of an email or a phone: a click opens the list of all the types (the current one
// checked) instead of moving to the next one. Phone numbers in pairs of digits.
function inspecteurContact48(extra = '') {
  const cp = `<button class="icon-btn icon-btn--sm" aria-label="Copy" title="Copy">${i('copy')}</button>`;
  const row = (kind, val, { open = false, acts = cp } = {}) => `<div style="display:flex;gap:4px;align-items:center"><span class="kind48 ${open ? 'is-open' : ''}" title="Change type">${kind}${i('chevron')}</span><div class="input" style="flex:1"><span class="grow">${val}</span><span class="act">${acts}</span></div></div>`;
  return `<aside class="inspector pane">
    <div class="inspector-head"><span class="avatar avatar--lg">CD</span>
      <div style="flex:1;min-width:0;display:flex;flex-direction:column"><span style="font-weight:var(--weight-medium);font-size:var(--text-lg)">Camille Dubois</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Account manager · Example Print</span></div>
      <button class="icon-btn" title="Remove from favorites" style="color:var(--warning-text)">${i('star')}</button><button class="icon-btn" title="More">${i('more')}</button></div>
    <div class="quick-acts"><button class="btn btn--sm">${i('mail')}Email</button><button class="btn btn--sm">${i('phone')}Call</button><button class="btn btn--sm">${i('copy')}Copy</button></div>
    <div class="inspector-tabs"><span class="is-active">Details</span><span>Linked</span><span>History</span></div>
    <div class="inspector-body" style="padding-top:var(--s-3);gap:var(--s-3)">
      <div class="field"><span class="label" style="display:flex">Email<span style="flex:1"></span>${i('plus')}</span>${row('Work', 'camille@example-print.com')}${row('Personal', 'camille.d@example.com')}</div>
      <div class="field"><span class="label" style="display:flex">Phone<span style="flex:1"></span>${i('plus')}</span>${row('Work', '+33 4 99 00 12 34')}${row('Mobile', '+33 6 99 00 12 35', { open: extra === 'menu' })}</div>
      <div class="field"><span class="label">Address</span>${row('Work', '12 rue de l’Exemple, 69002 Lyon', { acts: `<button class="icon-btn icon-btn--sm" title="Open in maps">${i('external')}</button>` })}</div>
      <div class="field"><span class="label">Birthday</span><div class="input ${extra === 'cal' ? 'is-focus' : ''}"><span class="grow">Mar 12, 1988</span><span class="act"><button class="icon-btn icon-btn--sm" title="Pick a date" ${extra === 'cal' ? 'style="background:var(--surface-active);color:var(--text)"' : ''}>${i('calendar')}</button></span></div></div>
      <div class="field"><span class="label">Notes</span><div class="input" style="height:auto;min-height:44px;align-items:flex-start;padding:6px 8px;color:var(--text-muted)">Quotes within 48 h · prefers email</div></div>
    </div></aside>`;
}
function contactsFrame48(extra, overlay) {
  return shell('Contacts', `
    <div class="work">${contactsSidebar44('All')}
      <section class="center"><div class="toolbar">${search48().replace('Search', 'Search name, company, phone')}<button class="btn btn--ghost">${i('filter')}Filter</button><button class="btn btn--ghost">${i('columns')}Columns</button></div>
        <div class="table-wrap">${contactsTable('Camille Dubois')}</div></section>
      ${inspecteurContact48(extra)}
    </div>
    <footer class="statusbar"><span>Suppliers › Camille Dubois</span><span class="sp"></span><span class="sb-item">${i('shield')}Standard encryption</span><span>Saved</span></footer>`, { primary: `${i('plus')}New contact`, overlay });
}
function typeMenu() {
  const ck = (label, on, hover = false) => `<div class="menu-item ${hover ? 'is-hover' : ''}">${on ? i('check') : '<span style="width:16px"></span>'}<span>${label}</span></div>`;
  const m = `<div class="menu" style="left:1166px;top:376px;width:112px;min-width:0">${ck('Work', false)}${ck('Mobile', true)}${ck('Home', false, true)}${ck('Other', false)}</div>`;
  return contactsFrame48('menu', m);
}
// 48f. Birthday: the calendar of the vault's dates, opened from the box; a birthday may be kept
// without its year.
function anniversaireCalendrier() {
  const days = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'].map((d) => `<span class="cal-dow">${d}</span>`).join('');
  let cells = '';
  for (let k = 0; k < 1; k++) cells += '<span class="cal-day is-out"></span>';
  for (let d = 1; d <= 31; d++) cells += `<span class="cal-day ${d === 12 ? 'is-on' : ''}">${d}</span>`;
  const pop = `<div class="cal" style="left:1170px;top:574px">
    <div class="cal-head"><button class="icon-btn icon-btn--sm" title="Previous month" style="transform:rotate(90deg)">${i('chevron')}</button><span class="sp" style="text-align:center;font-weight:var(--weight-medium)">March <span class="cal-year" title="Click to type the year">1988</span></span><button class="icon-btn icon-btn--sm" title="Next month" style="transform:rotate(-90deg)">${i('chevron')}</button></div>
    <div class="cal-grid">${days}${cells}</div>
    <div class="cal-foot"><button class="btn btn--ghost btn--sm" title="Keep the day and the month only">No year</button><span class="sp"></span><button class="btn btn--ghost btn--sm">Clear</button></div>
  </div>`;
  return contactsFrame48('cal', pop);
}

// 48g. A folder's icon: right-click › Icon…, a grid of icons (the current one outlined), Default
// puts the folder icon back. Personal folders: yours; an organization's spaces: its admins, seen by
// every member.
const FICONS48 = ['folder', 'star', 'heart', 'user', 'users', 'briefcase', 'building', 'bank', 'wallet', 'receipt', 'cart', 'gift', 'home', 'car', 'luggage', 'cap', 'book', 'medical', 'code', 'palette', 'camera', 'music', 'gamepad', 'wrench', 'mail', 'globe', 'cloud', 'key', 'shield', 'leaf'];
function iconeDossier() {
  const menu = ctx(150, 214, `${mi('plus', 'New item here')}${mi('text', 'Rename')}${mi('palette', 'Icon', '', { hover: true, sub: true })}${sep}${mi('trash', 'Delete', '', { danger: true })}`, 200);
  const grid = FICONS48.map((n) => `<button class="ico-pick ${n === 'cart' ? 'is-on' : ''} ${n === 'gift' ? 'is-hover' : ''}" title="${n[0].toUpperCase() + n.slice(1)}">${i(n)}</button>`).join('');
  const pick = `<div class="menu" style="left:374px;top:270px;width:236px;padding:8px"><div class="ico-grid">${grid}</div><div class="menu-sep"></div><div style="display:flex"><button class="btn btn--ghost btn--sm">${i('folder')}Default</button></div></div>`;
  const side = side48('Shopping').replace('<div class="side-item  ', '<div class="side-item ').replace(`${i('cart')}<span>Shopping</span>`, `${i('cart')}<span>Shopping</span>`);
  const center = `${toolbar48(search48())}<div class="table-wrap" style="position:relative">${vaultTable48({ only: 'My vault' })}</div>`;
  return frameB(center, { side, overlay: menu + pick });
}

// 48h. The price's billing period: a button at the right of the box, as the type of a contact's
// email: a click goes from month to year and back. The yearly cost follows.
function periodePrix(year = false) {
  const per = year ? 'year' : 'month';
  const insp = inspector().replace(/<div class="section" style="padding:var\(--s-3\);gap:var\(--s-2\);background:var\(--surface-raised\)">[\s\S]*?<\/dl>\s*<\/div>/, `<div class="section" style="padding:var(--s-3);gap:var(--s-2);background:var(--surface-raised)">
        <div style="display:flex;align-items:center;justify-content:space-between"><h3>Subscription</h3>${statusChip('Active')}</div>
        <div class="kvi"><span>Price</span><div class="input input--sm"><span class="grow num" style="text-align:left">€15.00</span><span class="per48" title="Billed per ${per} · click for ${year ? 'month' : 'year'}">/ ${per}</span></div></div>
        <div class="kvi"><span>Licenses</span><div class="input input--sm"><span class="grow">4</span></div></div>
        <div class="kvi"><span>Yearly cost</span><b style="margin-left:auto;font-weight:var(--weight-medium)">${year ? '€60.00' : '€720.00'}</b></div>
        <div class="kvi"><span>Renewal</span><div class="input input--sm"><span class="grow">Dec 3, 2026</span><span class="act" title="Pick a date">${i('calendar')}</span></div></div>
      </div>`);
  return shell('Vault', `<div class="work">${side48('Design')}<section class="center">${toolbar48(search48())}<div class="table-wrap">${vaultTable48({ only: 'Example Studio', selected: 'Figma' }).replace(/<tr class="grp">[\s\S]*?<\/tr>/g, (m) => (m.includes('Design') ? m : m))}</div></section>${insp}</div>${statusbar()}`);
}
function periodePrixAn() { return periodePrix(true); }

// 48i. Middle click in a list: the anchor appears where the wheel was pressed; moving the pointer
// away scrolls, faster the farther it goes (above: up, below: down); any click or Esc stops it.
// Held and dragged, it scrolls while held.
function defilementMolette() {
  const anchor = `<div class="anchor48" style="left:640px;top:420px" title="Move the pointer to scroll">${i('autoscroll')}</div>
    <svg class="cursor48" style="left:652px;top:560px" viewBox="0 0 24 24"><path d="M12 22 6 15h4V2h4v13h4Z"/></svg>`;
  const center = `${toolbar48(search48())}<div class="table-wrap" style="position:relative">${vaultTable48()}${scrollbar48(22, 46, true)}</div>`;
  return frameB(center, { side: side48('All items'), overlay: anchor });
}

// 48k. Fill best match on a page no item is saved for exactly: Quick Access lists the most likely
// logins first. Order: the item linked to the app; its Website on this address; its Website on
// the same site (another sub-domain); the site's name in the item's name; the window's title.
// Fill best match still fills at once when a single item is saved for this address.
function remplissageSite() {
  const it = (c, n, sub, why, sel = false) => `<div class="qa-item ${sel ? 'is-sel' : ''}"><span class="site-ico">${c}</span><div class="meta"><span class="site-name">${n}</span><span class="sub">${sub}</span></div>${why ? `<span class="chip" title="${why[1]}">${why[0]}</span>` : ''}${sel ? `<span class="qa-hint">${keys('↵')}Fill</span>` : ''}</div>`;
  return `<div class="desk">
    <div class="win" style="left:380px;top:420px;width:680px">
      <div class="win-bar"><span class="win-dots"><i></i><i></i><i></i></span><span>app.example-books.com/login</span></div>
      <div class="win-body">
        <div style="font-size:var(--text-lg);font-weight:var(--weight-medium)">Sign in to Example Books</div>
        <div class="field"><span class="label">Email</span><div class="input is-focus"><span class="grow"></span></div></div>
        <div style="display:flex;justify-content:flex-end"><button class="btn btn--primary">Sign in</button></div>
      </div>
    </div>
    <div class="qa" role="dialog" aria-label="Quick Access">
      <div class="qa-search">${i('search')}<span class="grow ph">Search</span><span class="chip chip--auto" title="Address of the tab in front">${i('globe')}app.example-books.com</span></div>
      <div class="qa-list">
        <div class="menu-head">Most likely</div>
        ${it('E', 'Example Books — billing', 'accounting@example-studio.com · Accounting', ['Same site', 'Saved for billing.example-books.com'], true)}
        ${it('E', 'Example Books', 'ines@example-studio.com · General', ['Name', '“Example Books” in its name, no website saved'])}
        ${it('E', 'Example Books — test', 'lea@example-studio.com · My vault', ['Name', '“Example Books” in its name, no website saved'])}
        <div class="menu-head">Other</div>
        ${it('M', 'Microsoft 365', 'accounting@example-studio.com · Accounting', null)}
      </div>
      <div class="qa-foot"><span class="qa-hint">${keys('Shift', '↵')}Type</span><span class="qa-hint">${keys('Ctrl', 'C')}Password</span><span style="flex:1"></span></div>
    </div>
  </div>`;
}

// 48l. Preferences › Lists: the folder lines can be turned off (on by default).
function preferencesLignesDossiers() {
  return settings('Preferences', `
    <div class="page-head"><h1>Preferences</h1></div>
    <div class="section">${sectionTitle('Lists')}
      <div class="row"><span class="row-label" title="In All items, a vault, All contacts: a line with each folder's name before its items">Folder lines</span><span class="switch on"></span></div>
    </div>`);
}

// ---------- 49. Lists, preferences, sync places (28/11/2026) ----------

// 49a. Filter: a "Show" group first, Favorites and items with recovery keys, with their counts;
// the button counts the filters on.
function filtreAfficher() {
  const opt = (label, on, count, tip = '') => `<div class="menu-item" ${tip ? `title="${tip}"` : ''}><span class="check ${on ? 'on' : ''}">${on ? i('check') : ''}</span><span>${label}</span><span class="kbd" style="margin-left:auto">${count}</span></div>`;
  const menu = `<div class="menu" style="top:84px;left:527px;width:240px">
    <div class="menu-head">Show</div>
    ${opt('Favorites', true, 4)}${opt('With recovery keys', false, 3, 'Items holding recovery codes')}
    <div class="menu-sep"></div>
    <div class="menu-head">Subscription</div>
    ${opt('Active', false, 7)}${opt('Renew soon', false, 1)}${opt('Pending', false, 1)}${opt('Free', false, 4)}${opt('Off', false, 2)}
    <div class="menu-sep"></div>
    <div class="menu-head">2FA</div>
    ${opt('With a 2FA code', false, 7)}${opt('Without', false, 8)}
    <div class="menu-sep"></div>
    <div class="menu-item">${i('x')}<span>Clear filters</span></div>
  </div>`;
  const tb = toolbar48(search48()).replace('btn btn--ghost">' + i('filter') + 'Filter', 'btn" style="background:var(--surface-active)">' + i('filter') + 'Filter · 1');
  const favOnly = vaultTable49({ keep: (r) => FAV48.includes(r.n) });
  return frameB(`${tb}<div class="table-wrap" style="position:relative">${favOnly}</div>`, { side: side48('All items'), overlay: menu });
}

// 49b. A folder's line (no icon, 0.2.4): its name on the left; on the right the vault's name (only when
// the list holds more than one vault) next to the count. No "›".
const sepRow49 = (cols, vault, folder, n, { showVault = true } = {}) => {
  const name = folder || vault;
  const right = showVault && folder ? `<span class="grp-vault">${vault}</span>` : '';
  return `<tr class="grp"><td colspan="${cols}"><span class="grp-in"><span class="grp-name">${name}</span><span class="grp-n">${right}${n}</span></span></td></tr>`;
};
// The star of a favorite after its name, full (0.2.4, as in Contacts).
const name49 = (r) => `<span class="site"><span class="site-ico">${r.c}</span><span class="site-name">${r.n}</span>${FAV48.includes(r.n) ? `<span class="fav" title="Favorite" style="margin-left:6px">${i('starFill')}</span>` : ''}</span>`;
function vaultTable49({ only = null, keep = null, showVault = true, selected = null } = {}) {
  const head = `<tr><th style="width:214px" class="sorted">Name <span class="sort">↑</span></th><th>Username</th><th style="width:116px">Password</th><th style="width:100px">2FA code</th><th style="width:120px">Subscription</th><th class="num" style="width:120px">Price</th><th class="num" style="width:100px">Yearly cost</th></tr>`;
  const order = [['My vault', ''], ['My vault', 'Banking'], ['My vault', 'Shopping'], ['Example Studio', 'General'], ['Example Studio', 'Design'], ['Example Studio', 'Accounting'], ['Example Studio', 'Dev']];
  let body = '';
  let count = 0;
  for (const [v, f] of order) {
    if (only && v !== only) continue;
    const rows = ROWS.filter((r) => FOLDER48[r.n][0] === v && FOLDER48[r.n][1] === f && (!keep || keep(r)));
    if (!rows.length) continue;
    count += rows.length;
    body += sepRow49(7, v, f, rows.length, { showVault: showVault && !only });
    body += rows.map((r) => {
      const totp = r.totp ? `<span class="totp"><span class="ring ${r.p < 25 ? 'low' : ''}" style="--p:${r.p}%"></span>${r.totp}</span>` : '<span style="color:var(--text-faint)">—</span>';
      return `<tr class="${r.n === selected ? 'is-selected' : ''}"><td>${name49(r)}</td><td class="muted">${r.u}</td><td><span class="dots">••••••</span></td><td>${totp}</td><td>${statusChip(r.st)}</td><td class="num">${r.price}${r.per ? `<span style="color:var(--text-faint)"> /${r.per}</span>` : ''}</td><td class="num">${r.cost}</td></tr>`;
    }).join('');
  }
  body += `<tr><td colspan="7" style="color:var(--text-muted);font-size:var(--text-sm);border-bottom:0">${count} items</td></tr>`;
  return `<table class="table table--fixed"><thead>${head}</thead><tbody>${body}</tbody></table>`;
}
function lignesDossiers49() {
  return frameB(`${toolbar48(search48())}<div class="table-wrap" style="position:relative">${vaultTable49()}${scrollbar48(0, 46)}</div>`, { side: side48('All items') });
}
// 49c. Without an organization (or a vault's own All items): one vault, so no vault name on the lines.
function lignesUnCoffre49() {
  const side = side48('All items');
  return frameB(`${toolbar48(search48())}<div class="table-wrap" style="position:relative">${vaultTable49({ only: 'My vault' })}</div>`, { side });
}

// 49d. Preferences, reordered: Appearance first (with the folder lines, the language and the
// currency), Startup second, then one "Navigation" card for the vaults, the tabs and the sidebar's
// shortcuts.
function pref49Body(start = 'All items') {
  const sw = (c, on = false, label) => `<button class="swatch ${on ? 'on' : ''}" style="${c ? `background:${c}` : ''}" aria-label="${label}" title="${label}">${c ? '' : i('x')}</button>`;
  const ck = (on) => `<span class="check ${on ? 'on' : ''}" style="margin:auto">${on ? i('check') : ''}</span>`;
  const cols = [['user', 'My vault'], ['users', 'Example Studio']];
  const head = (first) => `<div style="display:grid;grid-template-columns:1fr repeat(${cols.length},128px);align-items:center;height:32px;color:var(--text-muted);font-size:var(--text-xs);font-weight:var(--weight-medium);border-bottom:1px solid var(--line)"><span>${first}</span>${cols.map(([ic, n]) => `<span style="display:flex;align-items:center;justify-content:center;gap:6px">${i(ic)}${n}</span>`).join('')}</div>`;
  const line = (ic, label, vals, tip = '') => `<div style="display:grid;grid-template-columns:1fr repeat(${vals.length},128px);align-items:center;height:36px;border-bottom:1px solid var(--line)" ${tip ? `title="${tip}"` : ''}><span style="display:flex;align-items:center;gap:8px">${i(ic)}${label}</span>${vals.map(ck).join('')}</div>`;
  const headC = `<div style="display:grid;grid-template-columns:1fr 128px;align-items:center;height:32px;color:var(--text-muted);font-size:var(--text-xs);font-weight:var(--weight-medium);border-bottom:1px solid var(--line)"><span>Contacts</span><span style="display:flex;justify-content:center">Shown</span></div>`;
  return `
    <div class="page-head"><h1>Preferences</h1></div>
    <div class="section">${sectionTitle('Appearance')}
      <div class="row"><span class="row-label">Theme</span><div class="segmented" role="radiogroup" aria-label="Theme"><button role="radio" aria-checked="true">Dark</button><button role="radio" aria-checked="false">Light</button><button role="radio" aria-checked="false" title="Follow Windows or macOS">System</button></div></div>
      <div class="row"><span class="row-label">Logo</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="true">Light</button><button role="radio" aria-checked="false">Dark</button></div></div>
      <div class="row"><span class="row-label">Accent color</span><div class="swatches">${sw('', false, 'None')}${sw('var(--accent)', true, 'Ochre (default)')}${sw('#3f5a75', false, 'Blue')}${sw('#494a6a', false, 'Violet')}${sw('#48664f', false, 'Green')}${sw('#3f6766', false, 'Teal')}${sw('#7a4646', false, 'Red')}</div></div>
      <div class="row"><span class="row-label" title="Where the accent color is used">Accent scope</span><div class="segmented" role="radiogroup"><button role="radio" aria-checked="false">Minimal</button><button role="radio" aria-checked="false">Standard</button><button role="radio" aria-checked="true">Extended</button></div></div>
      <div class="row"><span class="row-label" title="In All items, a vault, All contacts: a line with each folder's name before its items">Folder lines</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Language</span><div class="input select" style="width:200px"><span class="grow">English</span></div></div>
      <div class="row"><span class="row-label">Currency</span><div class="input select" style="width:200px"><span class="grow">Euro (€)</span></div></div>
    </div>
    <div class="section">${sectionTitle('Startup')}
      <div class="row"><span class="row-label">Open at login</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label">Start in</span><div class="input select ${start === 'menu' ? 'is-focus' : ''}" style="width:200px"><span class="grow">${start === 'menu' ? 'My vault' : start}</span></div></div>
    </div>
    <div class="section">${sectionTitle('Navigation', 'Tabs at the top, shortcuts in the sidebar')}
      <div class="row"><span class="row-label" title="Off: one Vault tab, every vault in its sidebar">Personal vault in its own tab</span><span class="switch"></span></div>
      <div class="row"><span class="row-label">Personal vault name</span><div class="input" style="width:200px"><span class="grow">My vault</span></div></div>
      <div class="row"><span class="row-label">${i('users')}Contacts tab</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label" title="Shown to administrators only">${i('shield')}Health tab</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label" title="Shown to administrators only · hiding the tab does not stop recording">${i('clock')}Activity tab</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label" title="At the top of the sidebar">Shortcuts</span><span style="color:var(--text-muted);font-size:var(--text-sm)">Same for every vault</span><span class="switch"></span></div>
      <div style="padding-left:var(--s-4)">
        ${head('Vault')}
        ${line('list', 'All items', [true, true])}
        ${line('star', 'Favorites', [true, false])}
        ${line('clock', 'Renew soon', [false, true], 'Subscriptions renewing in the next 30 days')}
        <div style="height:var(--s-3)"></div>
        ${headC}
        ${line('users', 'All contacts', [true])}
        ${line('star', 'Favorites', [true])}
        ${line('calendar', 'Birthdays soon', [true], 'Next 30 days')}
        ${line('list', 'Uncategorized', [true], 'Contacts outside any folder')}
      </div>
    </div>`;
}
function preferences49() { return settings('Preferences', pref49Body()); }
function preferences49bas() { return settings('Preferences', pref49Body(), '', 560); }
// 49e. Start in, one Vault tab: My vault is offered (the personal vault's items) with the views.
function demarrerDans49() {
  return settings('Preferences', pref49Body('menu'), `<div class="menu" style="left:1089px;top:666px;width:200px">${mi('check', 'My vault')}${mi('', 'Example Studio')}${sep}${mi('', 'All items')}${mi('', 'Favorites')}${mi('', 'Contacts')}</div>`);
}

// 49f. Sync: the places in use, the first one leading (the others hold a copy of it), then one
// line to add a place. Nothing listed that is not in use.
const place49 = (ico, name, tip, right) => `<div class="row"><span class="site-ico" style="background:var(--surface-active)">${i(ico)}</span><span class="row-label" title="${tip}">${name}</span>${right}</div>`;
function synchro49({ menu = false, empty = false } = {}) {
  const places = empty
    ? `<div class="row"><span class="site-ico" style="background:var(--surface-active)">${i('laptop')}</span><span class="row-label" title="This vault stays on this computer">Local</span><button class="btn btn--sm">${i('plus')}Add a place</button></div>`
    : `${place49('cloud', 'Google Drive', 'Cleis / vault.cleis in your Google Drive · ines@example-studio.com', `<span class="chip chip--set" title="Merged with this computer at each sync; the other places hold a copy of it">Main</span><span class="chip chip--ok"><span class="dot"></span>Synced just now</span><button class="icon-btn icon-btn--sm" aria-label="More" title="Make main, disconnect">${i('more')}</button>`)}
      ${place49('cloud', 'OneDrive', 'Cleis / vault.cleis in your OneDrive · ines@example-studio.com', `<span class="chip" title="Gets a copy of Google Drive at each sync">Copy</span><span class="chip chip--ok"><span class="dot"></span>Synced just now</span><button class="icon-btn icon-btn--sm" aria-label="More" title="Make main, disconnect">${i('more')}</button>`)}
      ${place49('folder', 'Folder', 'D:\\Backups\\Cleis', `<span class="chip" title="Gets a copy of Google Drive at each sync">Copy</span><span class="chip chip--ok"><span class="dot"></span>Synced just now</span><button class="icon-btn icon-btn--sm" aria-label="More" title="Make main, disconnect">${i('more')}</button>`)}
      <div class="row"><span class="row-label">Sync automatically</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label"></span><button class="btn btn--sm ${menu ? 'is-active' : ''}">${i('plus')}Add a place</button></div>`;
  const addMenu = `<div class="menu" style="left:1069px;top:${empty ? 196 : 390}px;width:220px">${mi('cloud', 'Google Drive')}${mi('cloud', 'OneDrive')}${mi('folder', 'Folder')}${mi('repo', 'GitHub')}${mi('server', 'SFTP')}</div>`;
  const backups = backupsSection49();
  return settings('Sync & backups', `
    <div class="page-head"><h1 style="flex:none">Sync & backups</h1><span style="flex:1"></span>${empty ? '' : `<button class="btn btn--primary">${i('sync')}Sync now</button>`}</div>
    <div class="section">${sectionTitle('Sync', 'Your encrypted vault, kept the same on every device using these places')}${places}</div>
    ${backups}`, menu ? addMenu : '');
}
function synchroPlaces49() { return synchro49(); }
function synchroAjouter49() { return synchro49({ menu: true }); }
function synchroVide49() { return synchro49({ empty: true, menu: true }); }

// 49g. Backups: the 5 latest, each one restorable or deleted (bin at the end of the line); the
// older ones under "Show 7 more".
function backupsSection49(open = false, hover = 1) {
  const dates = ['2026-11-28 14-02', '2026-11-28 09-15', '2026-11-27 18-40', '2026-11-27 11-03', '2026-11-26 16-22', '2026-11-26 08-57', '2026-11-25 19-31', '2026-11-25 10-12', '2026-11-24 17-45', '2026-11-24 09-20', '2026-11-23 15-08', '2026-11-22 12-34'];
  const shown = open ? dates : dates.slice(0, 5);
  const rows = shown.map((d, n) => `<div class="row ${n === hover ? 'is-hover' : ''}"><span class="row-label">${d}</span><span style="color:var(--text-muted);font-size:var(--text-sm)">${148 - n} KB</span><button class="btn btn--sm" title="Brings back what this backup holds; nothing newer is lost">Restore</button><button class="icon-btn icon-btn--sm" aria-label="Delete" title="Delete this backup" style="${n === hover ? 'color:var(--danger-text)' : ''}">${i('trash')}</button></div>`).join('');
  return `<div class="section">${sectionTitle('Backups', 'Dated copies on this computer you can go back to')}
      <div class="row"><span class="row-label">Keep</span><div class="input select" style="width:160px"><span class="grow">30 versions</span></div><button class="btn btn--sm">${i('upload')}Back up now</button></div>
      ${rows}
      <div class="row"><span class="row-label"></span><button class="btn btn--ghost btn--sm"><span style="display:inline-grid;${open ? 'transform:rotate(180deg)' : ''}">${i('chevron')}</span>${open ? 'Show fewer' : 'Show 7 more'}</button></div>
      <div class="row"><span class="row-label" title="Copy of your vault, opened with your password">Encrypted export</span><button class="btn btn--sm">${i('download')}Export</button></div>
    </div>`;
}
function sauvegardes49() {
  return settings('Sync & backups', `
    <div class="page-head"><h1 style="flex:none">Sync & backups</h1><span style="flex:1"></span><button class="btn btn--primary">${i('sync')}Sync now</button></div>
    ${backupsSection49()}`);
}

// 49h. Google Drive or OneDrive added: where the vault goes. A "Cleis" folder at the top of the
// drive by default; "Choose a folder" opens the drive's folders (Google: its own picker in the
// browser). The vault file can also be moved later in the drive: Cleis finds it again.
function dossierDrive49(google = true) {
  const name = google ? 'Google Drive' : 'OneDrive';
  const folders = ['Cleis', 'Documents', 'Documents / Admin', 'Personal', 'Pictures'];
  const list = folders.map((f, n) => `<div class="menu-item ${n === 2 ? 'is-hover' : ''}" style="height:32px">${i('folder')}<span>${f}</span>${n === 0 ? '<span class="chip" style="margin-left:auto" title="Made by Cleis">New</span>' : ''}</div>`).join('');
  // Google lets Cleis see only the files it made or that you pick in Google's own window: the
  // folder is picked there (in the browser), Cleis never lists your Drive.
  const where = google
    ? `<div class="field"><span class="label">Folder</span>
        <div class="row" style="padding:0"><div class="input" style="flex:1"><span class="grow">${i('folder')} Documents / Admin</span></div><button class="btn" title="Opens Google Drive's folder picker in your browser">${i('external')}Choose in Google Drive</button></div>
      </div>`
    : `<div class="field"><span class="label">Folder</span>
        <div style="border:var(--border) solid var(--line-strong);border-radius:var(--r-md);padding:4px;max-height:200px;overflow:hidden;background:var(--bg)">${list}</div>
      </div>`;
  const body = `
    <div class="field"><span class="label">Account</span><div class="input"><span class="grow">ines@example-studio.com</span></div></div>
    ${where}
    <div class="field"><span class="label">File</span><div class="input"><span class="grow" style="color:var(--text-muted)">Documents / Admin /</span><span>vault.cleis</span></div></div>
    <div class="dialog-actions"><button class="btn btn--ghost">Cancel</button><button class="btn btn--primary">${i('check')}Use this folder</button></div>`;
  return settings('Sync & backups', `
    <div class="page-head"><h1 style="flex:none">Sync & backups</h1></div>`, dlg(`${name} — where to keep the vault`, body));
}
function dossierDrive49g() { return dossierDrive49(true); }
function dossierDrive49o() { return dossierDrive49(false); }

// 49i. Leaving an organization: refused while you are the only one who can invite. Its spaces stay;
// the dialog says what to do instead.
function quitterOrg49() {
  const body = `<p style="margin:0;color:var(--text-muted)">You are the only member who can invite people. After leaving, no one could let you back in.</p>
    <div class="dialog-actions"><button class="btn btn--ghost">Cancel</button><button class="btn">${i('users')}Members</button><button class="btn btn--danger">${i('trash')}Delete the organization</button></div>`;
  return settings('Server', serveurBody49(), dlg('Leave Example Studio?', body));
}
const serveurBody49 = () => `<div class="page-head"><h1>Organization</h1></div>
  <div class="section">${sectionTitle('Example Studio')}
    <div class="row"><span class="row-label">Where</span><span style="color:var(--text-muted)">Shared folder · D:\\Shared\\Example Studio</span></div>
    <div class="row"><span class="row-label">Signed in as</span><span>Inès Martin · Super Admin</span></div>
    <div class="row"><span class="row-label"></span><button class="btn btn--sm">${i('sync')}Sync now</button><button class="btn btn--danger-ghost btn--sm">Leave</button></div>
  </div>`;

export const SCREENS = {
  '01-coffre': ['Vault', coffre],
  '01b-filtre': ['Vault filter', filtre],
  '01c-nouvel-espace': ['New space', nouvelEspace],
  '01d-nouveau-dossier': ['New personal folder', nouveauDossier],
  '01e-supprimer-dossier': ['Delete a personal folder', supprimerDossier],
  '02-colonnes': ['Custom columns', colonnes],
  '03-premier-lancement': ['First launch', premierLancement],
  '04-deverrouiller': ['Unlock', deverrouiller],
  '05-code-2fa': ['Verification code', codeVerif],
  '05b-code-serveur': ['Verification code asked by the server', codeServeur],
  '06-securite-compte': ['Account security', securite],
  '07-membres': ['Members and roles', membres],
  '07b-inviter': ['Invite', inviter],
  '07c-code-invitation': ['Invitation code', codeInvitation],
  '07d-nouveau-role': ['New role', nouveauRole],
  '07e-super-admin': ['Members — Super Admin menu', superAdmin],
  '08-politique': ['Security policy', politique],
  '09-chiffrement': ['Encryption', chiffrement],
  '10-rechiffrer': ['Re-encrypt a space', rechiffrer],
  '11-journal': ['Activity log', journal],
  '12-sante': ['Health report', sante],
  '13-sauvegardes': ['Sync & backups — organization', sauvegardes],
  '14a-import-fichier': ['Import: file', importFichier],
  '14-import': ['CSV import', importCsv],
  '14c-import-revue': ['Import: review', importRevue],
  '15-preferences': ['Preferences', preferences],
  '16-remplissage': ['Autofill', remplissage],
  '17-acces-rapide': ['Quick Access', accesRapide],
  '18-politique-suite': ['Security policy — log, autofill', politiqueSuite],
  '19-remplissage-suite': ['Autofill — browsers, permissions', remplissageSuite],
  '20-contacts': ['Contacts', contacts],
  '21-fusion': ['Contacts — merge & fix', fusion],
  '22-restaurer': ['Restore my vault', restaurer],
  '23-synchro': ['Sync & backups', synchro],
  '32-onglets-organisation': ['Vault tabs — organization', ongletsOrg],
  '32b-onglets-personnel': ['Vault tabs — My vault', ongletsPerso],
  '32c-reglages-coffres': ['Preferences — vaults, two organizations', reglagesDeuxOrgs],
  '32d-journal-deux-orgs': ['Activity — two organizations', journalDeuxOrgs],
  '32e-sante-deux-orgs': ['Health — two organizations', santeDeuxOrgs],
  '33-menu-element': ['Right-click — item', menuElement],
  '33b-menu-espace': ['Right-click — space', menuEspace],
  '33c-menu-contact': ['Right-click — contact', menuContact],
  '33d-menu-membre': ['Right-click — member', menuMembre],
  '07f-roles-delegation': ['Roles — Admin delegations', rolesDelegation],
  '08b-politique-admin': ['Security policy seen by an Admin', politiqueAdmin],
  '34-confirmer-transfert': ['Confirm — transfer Super Admin', confirmTransfert],
  '34b-confirmer-retrait': ['Confirm — remove a member', confirmRetrait],
  '23b-place-github': ['Place: GitHub', placeGithub],
  '23c-place-sftp': ['Place: SFTP', placeSftp],
  '24-serveur': ['Server', serveur],
  '25-rejoindre': ['Join an organization', rejoindre],
  '25b-creer-organisation': ['Create an organization', creerOrganisation],
  '24b-serveur-heberge': ['Server hosted on this computer', serveurHeberge],
  '26-profil': ['Profile', profil],
  '27-appareils': ['Devices', appareils],
  '28-espaces': ['Spaces', espaces],
  '28b-renommer-espace': ['Rename a space', renommerEspace],
  '28c-supprimer-espace': ['Delete a space', supprimerEspace],
  '35-menu-vide-liste': ['Right-click — list void', menuVideListe],
  '35b-menu-vide-laterale': ['Right-click — sidebar void', menuVideLaterale],
  '35c-glisser-dossier': ['Drag an item onto a folder', glisserVersDossier],
  '35d-reorganiser': ['Reorder folders and items', reorganiser],
  '35e-largeur-colonnes': ['Column width', largeurColonnes],
  '35f-personnaliser': ['Customize', personnaliser],
  '35g-calendrier': ['Date — calendar', calendrier],
  '35h-ouverture': ['Security — opening Cleis', ouverture],
  '35i-premier-lancement-protection': ['First launch — unlock with', premierLancementProtection],
  '35j-raccourcis-coffres': ['Preferences — shortcuts per vault', raccourcisCoffres],
  '35k-politique-ouverture': ['Security policy — opening Cleis', politiqueOuverture],
  '36-roue-couleur': ['Preferences — your own accent color', roueCouleur],
  '36b-compte': ['Account — sign in', compte],
  '36c-compte-connecte': ['Account — signed in', compteConnecte],
  '36d-nouvel-ordinateur': ['First launch — sign in on a new computer', nouvelOrdinateur],
  '36e-photo-contact': ['Contact — photo', photoContact],
  '36f-stockage-organisation': ['Create an organization — where it lives', stockageOrganisation],
  '36i-dossier-organisation': ['Create an organization — shared folder', dossierOrganisation],
  '36g-politique-pin': ['Security policy — organization PIN', politiquePin],
  '36h-saisie-pin': ['Organization PIN asked', saisiePin],
  '37a-dossier-en-ligne': ['New folder — named in place', dossierEnLigne],
  '37b-dossier-renomme': ['Folder — renamed by double-click', dossierRenomme],
  '37c-separateurs-colonnes': ['Columns — edge bars', separateursColonnes],
  '37d-pastille-connexion': ['Top bar — connection dot', pastilleConnexion],
  '37e-ouverture-pin': ['Security — PIN for my vault', ouverturePin],
  '37f-choisir-pin': ['Security — new PIN', choisirPin],
  '37g-deverrouiller-pin': ['Unlock — PIN', deverrouillerPin],
  '37h-premier-lancement-pin': ['First launch — PIN', premierLancementPin],
  '37i-raccourcis-un-onglet': ['Preferences — shortcuts per vault, one Vault tab', raccourcisUnOnglet],
  '37j-laterale-raccourcis': ['Sidebar — shortcuts under each vault', laterale37],
  '37k-demarrer-dans': ['Preferences — start in a vault tab', demarrerDans],
  '37l-synchro-suppressions': ['Sync & backups — deletions', synchro37],
  '37m-supprimer-compte': ['Delete the account', supprimerCompte],
  '37n-connexion-github': ['Account — sign in with GitHub', connexionGithub],
  '37o-menu-mot-de-passe': ['Right-click — password field', menuChampMotDePasse],
  '37p-clic-droit-ligne': ['Right-click — row outlined, not opened', clicDroitLigne],
  '37q-pin-organisation': ['Security policy — organization PIN typed twice', choisirPinOrganisation],
  '37r-github-sans-coffre': ['Sign in with GitHub — no vault on the account', githubSansCoffre],
  '39a-compte-fusion': ['Account — with security, top', compteFusionHaut],
  '39b-compte-fusion-bas': ['Account — bottom, delete account', compteFusionBas],
  '39c-revoquer-github': ['Account — revoke GitHub access', revoquerGithub],
  '39d-compte-local': ['Account — local, GitHub access revocable', compteLocalRevoquer],
  '40f-parcours-debut': ['First launch — the three options alone', parcoursDebut],
  '41a-github-app-depot': ['GitHub App — create the repository', ghAppCreer],
  '41b-github-app-installer': ['GitHub App — let Cleis use only it', ghAppInstaller],
  '42a-acces-org-rien': ['Security policy — accessing the organization, nothing asked', accesOrgOff],
  '42b-acces-org-mot-de-passe': ['Security policy — accessing the organization, a password', accesOrgMotDePasse],
  '42c-acces-org-menu': ['Security policy — accessing the organization, with what', accesOrgMenu],
  '42d-acces-org-pin': ['Security policy — accessing the organization, a PIN', accesOrgPin],
  '42e-acces-org-hello': ['Security policy — accessing the organization, Windows Hello', accesOrgHello],
  '42f-acces-org-definir': ['Security policy — the organization password typed twice', accesOrgDefinir],
  '42g-acces-org-membre': ['A member — the organization asks its password', accesOrgMembre],
  '42h-acces-org-membre-hello': ['A member — the organization asks Windows Hello', accesOrgMembreHello],
  '42i-serveur-supprimer': ['Server — delete (Super Admin)', serveurSupprimer],
  '42j-serveur-supprimer-confirmer': ['Server — delete, confirmed', serveurSupprimerConfirmer],
  '42k-logo-organisation': ['Server — organization logo, none yet', serveurLogo],
  '42l-logo-survol': ['Server — organization logo under the pointer', serveurLogoSurvol],
  '43a-sante-changer': ['Health — Change asks first', santeChanger],
  '44a-contacts-organisation': ['Contacts — the organization as a section', contactsOrg],
  '44b-contacts-sans-dossier': ['Contacts — organization, uncategorized', contactsUncategorized],
  '45a-raccourcis-grille': ['Preferences — shortcuts as a grid, per vault', raccourcisGrille],
  '45b-raccourcis-grille-meme': ['Preferences — shortcuts as a grid, same for every vault', raccourcisGrilleMeme],
  '46a-profil-menu': ['Profile menu — not signed in', profilMenu],
  '46b-profil-menu-github': ['Profile menu — signed in to GitHub', profilMenuGithub],
  '46c-profil-editer': ['Profile — edit from the avatar', profilEditer],
  '46d-colonne-site': ['Website column — open link, stronger cell buttons', siteColonne],
  '46e-raccourcis-cases': ['Shortcuts — keys in an editable box', raccourcisCases],
  '46f-espaces-membres': ['Spaces — members of a space', espacesMembres],
  '47a-organisation-choix': ['Create an organization — nothing chosen', orgCreer],
  '47b-organisation-serveur': ['Create an organization — Server, on the NAS', orgServeur],
  '47c-organisation-serveur-ici': ['Create an organization — Server, on this computer', orgServeurIci],
  '47d-organisation-dossier': ['Create an organization — Shared folder', orgDossier],
  '47e-corbeille-menu': ['Trash — right-click', corbeilleMenu],
  '47f-cles-recuperation': ['Recovery keys — column and section', cleRecuperation],
  '47g-cles-recuperation-ajout': ['Recovery keys — paste codes', cleRecuperationAjout],
  '40a-parcours-code': ['First launch — GitHub chosen, code to type', parcoursCode],
  '40b-parcours-creer': ['First launch — signed in, vault made here', parcoursCreer],
  '40c-parcours-ouvrir': ['First launch — signed in, vault opened here', parcoursOuvrir],
  '40d-profil-compte': ['Top bar — the account and its menu', profilCompte],
  '40e-compte-local': ['Account — signed out, no Revoke', compteLocal40],
  '38-raccourcis-clavier': ['Settings — keyboard shortcuts', raccourcisClavier],
  '38b-raccourci-conflit': ['Settings — shortcut already in use', raccourciConflit],
  '48a-liste-dossiers': ['All items — folder lines, star before the name, scroll bar', listeDossiers],
  '48b-liste-mon-coffre': ['My vault — its folders', listeMonCoffre],
  '48c-recherche-effacer': ['Search — clear button', rechercheEffacer],
  '48d-contacts-dossiers': ['All contacts — folder lines, Website column', contactsDossiers],
  '48e-type-menu': ['Contact — type menu, numbers in pairs', typeMenu],
  '48f-anniversaire-calendrier': ['Contact — birthday calendar', anniversaireCalendrier],
  '48g-icone-dossier': ['Folder — icon', iconeDossier],
  '48h-periode-prix': ['Subscription — billing period, per month', periodePrix],
  '48i-periode-prix-an': ['Subscription — billing period, per year', periodePrixAn],
  '48j-defilement-molette': ['Middle click — scroll', defilementMolette],
  '48k-remplissage-site': ['Fill best match — most likely logins by the site', remplissageSite],
  '48l-preferences-lignes-dossiers': ['Preferences — folder lines in lists', preferencesLignesDossiers],
  '49a-filtre-afficher': ['Filter — Show: favorites, recovery keys', filtreAfficher],
  '49b-lignes-dossiers': ['All items — folder lines, vault at the right', lignesDossiers49],
  '49c-lignes-un-coffre': ['All items — one vault, no vault name', lignesUnCoffre49],
  '49d-preferences': ['Preferences — Appearance, Startup, Navigation', preferences49],
  '49m-preferences-navigation': ['Preferences — Navigation card', preferences49bas],
  '49e-demarrer-mon-coffre': ['Preferences — Start in My vault', demarrerDans49],
  '49f-synchro-places': ['Sync — main place and copies', synchroPlaces49],
  '49g-synchro-ajouter': ['Sync — add a place', synchroAjouter49],
  '49h-synchro-vide': ['Sync — no place yet', synchroVide49],
  '49i-sauvegardes': ['Backups — 5 latest, delete, show more', sauvegardes49],
  '49j-drive-dossier': ['Google Drive — where to keep the vault', dossierDrive49g],
  '49k-onedrive-dossier': ['OneDrive — where to keep the vault', dossierDrive49o],
  '49l-quitter-org': ['Leave — refused for the only one who can invite', quitterOrg49],
};
