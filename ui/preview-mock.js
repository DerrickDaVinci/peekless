// Browser preview only: when the page is opened outside the app (no
// window.__TAURI__), fake the backend with sample data so the UI can be
// designed in any browser. Inside the app this file does nothing.
(() => {
  if (window.__TAURI__) return;
  const listeners = {};
  const DEFAULTS = { lang: 'auto', theme: 'auto', launchAtLogin: false, startHidden: false, guard: true, autoCover: false, guardImages: true, guardText: true, cardSeconds: 15,
    ring: true, ringModifier: 'option', ringActions: ['inspect', 'strip', 'copy', 'cover'], shortcut: 'CmdOrCtrl+Alt+KeyR', shortcutEnabled: true,
    keepHistory: true, disabledKinds: [], style: 'solid', outputDir: null, cleanTag: 'clean', redactTag: 'redacted', revealAfter: false };
  const sample = {
    type: 'image', id: 1, width: 1100, height: 1100, lines: 7, origin: 'clipboard', file: null,
    src: '../docs/before.png',
    findings: [{"kind": "email", "preview": "derr••••com", "severity": 2, "boxes": [[259, 98, 357, 46]]}, {"kind": "phone", "preview": "13••••8", "severity": 2, "boxes": [[95, 158, 191, 46]]}, {"kind": "cn_id", "preview": "1101••••02X", "severity": 3, "boxes": [[416, 158, 296, 46]]}, {"kind": "card", "preview": "4111••••111", "severity": 3, "boxes": [[93, 218, 244, 46]]}, {"kind": "my_ic", "preview": "9001••••678", "severity": 3, "boxes": [[185, 283, 233, 38]]}, {"kind": "phone", "preview": "+60 ••••789", "severity": 2, "boxes": [[553, 283, 234, 38]]}, {"kind": "api_key", "preview": "sk-a••••HUJ", "severity": 3, "boxes": [[410, 340, 588, 44]]}],
  };
  const text = 'DATABASE_URL=postgres://admin:s3cretPw@db.internal:5432/app\nOPENAI_API_KEY=sk-proj-8fJ2kLmN0pQrStUvWxYz12345678\n联系人：derrick.demo@example.com，手机 13812345678\nThe meeting is at 10:30.';
  const files = [
    { path: '/Users/me/Downloads/IMG_0042.JPG', name: 'IMG_0042.JPG', format: 'jpg', size: 3432943, gps: [48.8584, 2.2945], can_clean: true, can_redact: true, note: null,
      items: [{ key: 'gps', value: '48.85840, 2.29450', risk: 3 }, { key: 'camera', value: 'Apple iPhone 14 Pro', risk: 2 }, { key: 'taken', value: '2026-07-24 16:57:33', risk: 2 }, { key: 'lens', value: 'iPhone 14 Pro back triple camera 6.86mm f/1.78', risk: 1 }, { key: 'software', value: '26.0.2', risk: 1 }] },
    { path: '/Users/me/Desktop/render.png', name: 'render.png', format: 'png', size: 812340, gps: null, can_clean: true, can_redact: true, note: null,
      items: [{ key: 'png_text', value: 'parameters: a cat astronaut, studio light, by my secret prompt…', risk: 3 }] },
    { path: '/Users/me/Desktop/Screenshot.png', name: 'Screenshot 2026-11-06 at 1.36.51 PM.png', format: 'png', size: 1067037, gps: null, can_clean: true, can_redact: true, note: null, items: [] },
  ];
  const mode = new URLSearchParams(location.search).get('mock') || 'image';
  const handlers = {
    info: () => ({ shortcutOk: true, platform: 'macos', version: '0.1.0', settings: { ...DEFAULTS }, defaults: { ...DEFAULTS } }),
    set_settings: ({ settings }) => ({ settings, shortcutOk: true }),
    check_shortcut: () => true,
    set_lang: () => null,
    try_sample: () => null,
    pick_folder: () => '/Users/me/Desktop/Cleaned',
    clear_history: () => null,
    history: () => [
      { time: Date.now() / 1000 - 40, source: 'clipboard', action: 'cover', label: '', hidden: 7, outputs: [] },
      { time: Date.now() / 1000 - 1500, source: 'ring', action: 'strip', label: 'IMG_0042.JPG', hidden: 0, outputs: ['/Users/me/Downloads/IMG_0042.clean.jpg'] },
      { time: Date.now() / 1000 - 7400, source: 'clipboard', action: 'text', label: '', hidden: 2, outputs: [] },
      { time: Date.now() / 1000 - 90000, source: 'ring', action: 'cover', label: '3', hidden: 11, outputs: ['/a.png', '/b.png', '/c.png'] },
    ],
    clean_clipboard: () => (mode === 'text' ? { type: 'text', text, findings: window.__mockTextFindings(text) } : sample),
    inspect_files: () => files,
    clean_files: ({ paths }) => paths.map((p) => ({ path: p, out: p.replace(/\.(\w+)$/, '.clean.$1'), removed_bytes: 110204, error: null })),
    open_image: () => ({ ...sample, origin: 'file' }),
    apply_image: () => 'copied',
    apply_text: () => 'ok',
    pick_files: () => files.map((f) => f.path),
    hud_payload: () => (mode === 'hudtext'
      ? { mode: 'ask', kind: 'text', items: [{ kind: 'api_key', preview: 'sk-a••••IJ' }, { kind: 'email', preview: 'derr••••om' }], excerpt: 'OPENAI_API_KEY=[REDACTED:api_key]\n联系人：[REDACTED:email]，明天见', src: null, width: 0, height: 0, boxes: [], seq: 1 }
      : { mode: mode === 'huddone' ? 'done' : 'ask', kind: 'image', items: sample.findings.map((f) => ({ kind: f.kind, preview: f.preview })), src: sample.src, width: sample.width, height: sample.height, boxes: sample.findings.flatMap((f) => f.boxes), excerpt: null, seq: 1 }),
    hud_accept: () => null,
    ring_action: ({ action, paths }) => ({ done: action === 'copy' ? 1 : paths.length, failed: 0, hidden: 7, outputs: paths, error: null }),
    ring_hide: () => null,
    hud_hide: () => null,
  };
  window.__mockTextFindings = (t) => {
    const chars = Array.from(t);
    const find = (s, kind, sev) => {
      const i = chars.join('').indexOf(s);
      const start = Array.from(chars.join('').slice(0, i)).length;
      return { kind, start, end: start + Array.from(s).length, preview: s.slice(0, 4) + '••••' + s.slice(-2), severity: sev };
    };
    return [find('s3cretPw', 'connection', 3), find('sk-proj-8fJ2kLmN0pQrStUvWxYz12345678', 'api_key', 3), find('derrick.demo@example.com', 'email', 2), find('13812345678', 'phone', 2)];
  };
  window.__TAURI__ = {
    core: { invoke: async (cmd, args) => { await new Promise((r) => setTimeout(r, 250)); const h = handlers[cmd]; return h ? h(args || {}) : null; } },
    event: { listen: (n, cb) => { listeners[n] = cb; return Promise.resolve(() => {}); } },
    webview: { getCurrentWebview: () => ({ onDragDropEvent: () => Promise.resolve(() => {}) }) },
  };
  // ?page=ring opens a settings page, ?open=review|files opens a view (used for README screenshots).
  const q = new URLSearchParams(location.search);
  addEventListener('load', () => setTimeout(() => {
    if (q.get('page')) document.querySelector(`#nav button[data-page="${q.get('page')}"]`)?.click();
    if (q.get('open') === 'review') document.querySelector('#cleanBtn')?.click();
    if (q.get('open') === 'files') document.querySelector('#pickBtn')?.click();
  }, 50));
  window.__mock = { trigger: () => listeners['trigger-clean'] && listeners['trigger-clean'](), files: () => handlers.inspect_files() };
})();
