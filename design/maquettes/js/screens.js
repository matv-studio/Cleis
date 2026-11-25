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
function shell(active, inner, { primary = `${i('plus')}New item`, overlay = '' } = {}) {
  return `<div class="app">
    <header class="topbar">
      <div class="topbar-start">
        <span class="logo" aria-hidden="true">${i('key')}</span>
        <span class="doc-switch" title="Switch vault">Example Studio ${i('chevron', 'chev')}</span>
      </div>
      <nav class="tabs" role="tablist">${TABS.map((t) => `<span class="tab ${t === active ? 'is-active' : ''}" role="tab">${t}</span>`).join('')}</nav>
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

function vaultTable({ selected = 'Figma', hover = null, lic = false } = {}) {
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
    return `<tr class="${isSel ? 'is-selected' : ''} ${isHov ? 'is-hover' : ''}">
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
      <div class="choice is-active"><span class="radio on"></span><div class="txt"><b style="font-weight:var(--weight-medium)">Add an organization server</b><span>Shared spaces, on top of your personal vault</span>
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
        <div style="display:flex;gap:var(--s-2);justify-content:center">
          <button class="btn btn--ghost btn--sm" title="Windows Hello / Touch ID — turned off by your organization" disabled>${i('fingerprint')}Biometrics</button>
          <button class="btn btn--ghost btn--sm">${i('usb')}Security key</button>
        </div>
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
    ${it('user', 'Profile')}${it('lock', 'Security')}${it('laptop', 'Devices')}${it('gear', 'Preferences')}${it('fill', 'Autofill')}${it('sync', 'Sync & backups')}
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
        <div class="segmented" role="radiogroup"><button role="radio" aria-checked="true">${i('phone')}App</button><button role="radio" aria-checked="false">${i('mail')}Email</button><button role="radio" aria-checked="false">SMS</button><button role="radio" aria-checked="false">${i('usb')}Key</button></div></div>
      <div class="row"><span class="row-label">Recovery codes</span><span style="color:var(--text-muted);font-size:var(--text-sm)">8 left</span><button class="btn">Show</button></div>
    </div>
    <div class="section">${sectionTitle('Signing in to the server', 'Rules set by the Example Studio administrator.')}
      <div class="row"><span class="row-label" style="color:var(--inherit-text)">Verification code</span>${inherited('Required')}</div>
      <div class="row"><span class="row-label">Method</span><div class="input select" style="width:220px"><span class="grow">Authenticator app</span></div></div>
      <div class="row"><span class="row-label" style="color:var(--inherit-text)">Remember device</span>${inherited('30 days')}</div>
    </div>
    <div class="section">${sectionTitle('Locking')}
      <div class="row"><span class="row-label" style="color:var(--inherit-text)" title="Set by your organization">After inactivity</span>${inherited('Never')}</div>
      <div class="row planned" title="Turned off by your organization"><span class="row-label">Biometrics (Windows Hello, Touch ID)</span><span class="switch"></span></div>
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
      <div class="row"><span class="row-label">Accent color</span><div class="swatches">${sw('', false, 'None')}${sw('#a57118', true, 'Bronze (default)')}${sw('#3b82f6', false, 'Blue')}${sw('#8b5cf6', false, 'Violet')}${sw('#10b981', false, 'Green')}${sw('#f59e0b', false, 'Amber')}${sw('#ef4444', false, 'Red')}</div></div>
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
function membres() {
  const m = (ini, n, e, role, groups, tfa, exp, last, lockExp = false) => `<tr>
    <td><div class="member"><span class="avatar">${ini}</span><div class="meta"><span>${n}</span><span class="sub">${e}</span></div></div></td>
    <td><div class="input select" style="width:120px;height:24px"><span class="grow">${role}</span></div></td>
    <td>${groups.map((g) => `<span class="chip" style="margin-right:4px">${g}</span>`).join('')}</td>
    <td>${tfa ? '<span class="chip chip--ok"><span class="dot"></span>On</span>' : '<span class="chip chip--warn" title="Asked at next sign-in"><span class="dot"></span>To set up</span>'}</td>
    <td title="${lockExp ? 'Administrators can always export' : 'Can export the organization’s passwords'}"><span class="switch ${exp ? 'on' : ''} ${lockExp ? 'planned' : ''}"></span></td>
    <td class="muted">${last}</td>
    <td><button class="icon-btn icon-btn--sm" aria-label="More" title="More">${i('more')}</button></td></tr>`;
  const perm = (n, cells) => `<tr><td>${n}</td>${cells.map((c) => `<td style="text-align:center">${c ? `<span class="check on" style="display:inline-grid">${i('check')}</span>` : '<span class="check" style="display:inline-grid"></span>'}</td>`).join('')}</tr>`;
  return settings('Members', `
    <div class="page-head"><h1>Members</h1><div class="input search" style="width:220px">${i('search')}<span class="grow ph">Search</span></div><button class="btn btn--primary">${i('plus')}Invite</button></div>
    <div class="section" style="padding:0;overflow:hidden">
      <table class="table table--roomy"><thead><tr><th>Member</th><th>Role</th><th>Spaces</th><th>2FA</th><th style="width:70px" title="Allowed to export the organization’s passwords">Export</th><th>Last sign-in</th><th style="width:40px"></th></tr></thead><tbody>
      ${m('LM', 'Lea Martin', 'lea@example-studio.com', 'Admin', ['All'], true, true, 'Today, 9:12 AM', true)}
      ${m('HB', 'Hugo Bernard', 'hugo@example-studio.com', 'Member', ['General', 'Design', 'Dev'], true, false, 'Today, 10:47 AM')}
      ${m('IP', 'Ines Petit', 'ines@example-studio.com', 'Member', ['General', 'Accounting'], true, true, 'Yesterday, 5:30 PM')}
      ${m('SR', 'Sarah Roux', 'sarah@example-studio.com', 'Read only', ['General'], false, false, 'Nov 20, 2026')}
      ${m('TD', 'Tom Durand', 'tom@example-studio.com', 'Intern', ['Interns'], true, false, 'Nov 21, 2026')}
      <tr><td colspan="7" style="color:var(--text-muted);border-bottom:0"><div class="member"><span class="avatar" style="background:transparent;border:1px dashed var(--line-strong)">${i('mail')}</span><div class="meta"><span>noa@example-studio.com</span><span class="sub">Invited Nov 22 · expires in 5 days</span></div><button class="btn btn--ghost btn--sm" style="margin-left:auto">Resend</button><button class="btn btn--danger-ghost btn--sm">Revoke</button></div></td></tr>
      </tbody></table>
    </div>
    <div class="section">${sectionTitle('Roles', 'What each role can do in the spaces it can access.')}
      <table class="table" style="margin-top:-4px"><thead><tr><th></th><th style="text-align:center">Admin</th><th style="text-align:center">Member</th><th style="text-align:center">Read only</th><th style="text-align:center" title="Fill through the browser extension only">Intern</th></tr></thead><tbody>
        ${perm('See passwords', [1, 1, 1, 0])}
        ${perm('Copy, fill', [1, 1, 1, 1])}
        ${perm('Add, edit', [1, 1, 0, 0])}
        ${perm('Delete', [1, 1, 0, 0])}
        ${perm('Manage members and spaces', [1, 0, 0, 0])}
        ${perm('Activity log, health report', [1, 0, 0, 0])}
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
        <div>${meth('usb', 'Security key, passkey', 1)}${meth('globe', 'Google account', 0)}${meth('globe', 'Microsoft account', 1)}</div>
      </div>
    </div>
    <div class="section">${sectionTitle('Locking')}
      <div class="row"><span class="row-label">After inactivity</span><div class="input select" style="width:160px"><span class="grow">Never</span></div></div>
      <div class="row"><span class="row-label">Biometrics</span><span class="switch"></span></div>
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
        <div><div class="planned" title="Paused for now">${meth('globe', 'Browser extension', 0)}</div>${meth('window', 'Read browser addresses', 1)}</div>
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
      <div class="section planned" title="Planned: check against known breaches (external service, off by default)"><div class="row" style="padding:0"><span class="row-label">${i('globe')}Passwords found in breaches</span><span class="switch"></span></div></div>
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
    <div class="section planned" title="Paused for now">${sectionTitle('Browser extension')}
      ${br('Chrome', '<span class="chip chip--ok"><span class="dot"></span>Active</span>')}
      ${br('Edge', '<span class="chip">Not installed</span>', '<button class="btn btn--sm">Install</button>')}
      ${br('Firefox', '<span class="chip chip--ok"><span class="dot"></span>Active</span>')}
      ${br('Safari', '<span class="chip chip--warn" title="Unsigned extension: Safari turns “Allow unsigned extensions” off when it quits">Turn on after each launch</span>', '<button class="btn btn--sm">Turn on</button>')}
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
  const place = (ico, name, sub, action) => `<div class="row"><span class="site-ico">${i(ico)}</span><div class="member row-label"><div class="meta"><span>${name}</span><span class="sub">${sub}</span></div></div>${action}</div>`;
  return settings('Sync & backups', `
    <div class="page-head"><h1 style="flex:none">Sync & backups</h1><div class="segmented" role="radiogroup" aria-label="Whose vault" title="Your vault, or the organization's (administrators)"><button role="radio" aria-checked="true">Mine</button><button role="radio" aria-checked="false">Example Studio</button></div><span style="flex:1"></span><button class="btn btn--primary">${i('sync')}Sync now</button></div>
    <div class="section">${sectionTitle('Sync', 'Your encrypted vault, kept the same on every device signed in to this place. Your master password stays on your devices.')}
      <div class="row"><span class="site-ico">${i('repo')}</span><div class="member row-label"><div class="meta"><span class="code">lea-example/cleis-vault</span><span class="sub">GitHub · lea-example</span></div></div><span class="chip chip--ok" title="Last sync today at 10:42"><span class="dot"></span>Up to date</span><button class="btn btn--ghost btn--sm">Disconnect</button></div>
      <div class="row"><span class="row-label">Sync automatically</span><span class="switch on"></span></div>
      <div class="row"><span class="row-label" title="Devices that opened this vault">Devices</span><span class="muted" style="font-size:var(--text-sm)">This PC · MacBook · Office PC</span><button class="btn btn--ghost btn--sm">Manage</button></div>
    </div>
    <div class="section">${sectionTitle('Other places', 'A second copy, kept in sync too.')}
      ${place('cloud', 'OneDrive', 'Microsoft account', '<button class="btn btn--sm">Sign in</button>')}
      ${place('cloud', 'Google Drive', 'Google account', '<button class="btn btn--sm">Sign in</button>')}
      ${place('server', 'SFTP', 'Your own server', '<button class="btn btn--sm">Set up</button>')}
    </div>
    <div class="section">${sectionTitle('New device', 'What a new computer needs besides your master password.')}
      <div class="row"><span class="row-label" title="QR code and text holding the place of your vault and its access. Useless without the master password.">${i('qr')}Setup code</span><button class="btn btn--sm">Show</button></div>
      <div class="row"><span class="row-label" title="PDF with the setup code and a space to write your master password. Print it and keep it safe.">${i('file')}Emergency kit</span><button class="btn btn--sm">${i('download')}Download</button></div>
    </div>
    <div class="section">${sectionTitle('Backups', 'Dated copies you can go back to.')}
      <div class="row"><span class="row-label">Keep</span><div class="input select" style="width:160px"><span class="grow">30 versions</span></div></div>
      <div class="row"><span class="row-label">Encrypted export</span><button class="btn btn--sm">${i('download')}Export</button></div>
    </div>`);
}

export const SCREENS = {
  '01-coffre': ['Vault', coffre],
  '02-colonnes': ['Custom columns', colonnes],
  '03-premier-lancement': ['First launch', premierLancement],
  '04-deverrouiller': ['Unlock', deverrouiller],
  '05-code-2fa': ['Verification code', codeVerif],
  '06-securite-compte': ['Account security', securite],
  '07-membres': ['Members and roles', membres],
  '08-politique': ['Security policy', politique],
  '09-chiffrement': ['Encryption', chiffrement],
  '10-rechiffrer': ['Re-encrypt a space', rechiffrer],
  '11-journal': ['Activity log', journal],
  '12-sante': ['Health report', sante],
  '13-sauvegardes': ['Sync & backups — organization', sauvegardes],
  '14-import': ['CSV import', importCsv],
  '15-preferences': ['Preferences', preferences],
  '16-remplissage': ['Autofill', remplissage],
  '17-acces-rapide': ['Quick Access', accesRapide],
  '18-politique-suite': ['Security policy — log, autofill', politiqueSuite],
  '19-remplissage-suite': ['Autofill — browsers, permissions', remplissageSuite],
  '20-contacts': ['Contacts', contacts],
  '21-fusion': ['Contacts — merge & fix', fusion],
  '22-restaurer': ['Restore my vault', restaurer],
  '23-synchro': ['Sync & backups', synchro],
};
