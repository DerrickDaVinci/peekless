// The action ring: opens under the cursor while you drag files with the
// trigger key held. Drop onto a segment to run it; drop in the middle to cancel.
(() => {
  const T = window.__TAURI__;
  const invoke = (c, a) => T.core.invoke(c, a);
  const svg = document.getElementById('ring');
  const { C, r } = SX;
  const IMG = /\.(jpe?g|png|heic|heif|webp|tiff?|gif|bmp)$/i;
  const ZH = { drop: '拖到一项上', cancel: '松手取消', done: (n) => `完成 ${n} 个`, hidden: (n) => `遮住 ${n} 处`, copied: '已复制，去粘贴', none: '没有可处理的图片', failed: '处理失败' };
  const EN = { drop: 'Drop on one', cancel: 'Release to cancel', done: (n) => `${n} done`, hidden: (n) => `${n} covered`, copied: 'Copied. Paste away.', none: 'No images to process', failed: 'Something failed' };

  let zh = SX.sysZh;
  let L = zh ? ZH : EN;
  let ids = [...SX.ORDER];
  let paths = [];
  let hot = -2; // -2 none, -1 hub, 0..n-1 segment
  let busy = false;
  let dropped = false;
  let result = null;
  // Near a screen edge the ring is shifted, so the cursor may start on top of
  // a segment. Segments only arm once the cursor has clearly moved.
  let start = null;
  let armed = false;
  const ARM_PX = 12;

  function applySettings(s) {
    if (!s) return;
    zh = SX.zhFor(s.lang);
    L = zh ? ZH : EN;
    SX.applyTheme(s.theme);
    if (Array.isArray(s.ringActions) && s.ringActions.length >= 2) ids = s.ringActions.filter((a) => SX.ACTIONS[a]);
    render();
  }

  const disabled = (i) => SX.ACTIONS[ids[i]].needsImage && !paths.some((p) => IMG.test(p));

  function hub() {
    if (busy) return `<circle cx="${C}" cy="${C}" r="${r - 6}"/><circle class="spin" cx="${C}" cy="${C}" r="18" stroke-dasharray="70 200"/>`;
    if (result) {
      const cls = result.ok ? 'ok' : 'err';
      return `<circle cx="${C}" cy="${C}" r="${r - 6}"/>
        <text class="big ${cls}" x="${C}" y="${C - 2}">${result.ok ? '✓' : '!'}</text>
        <text x="${C}" y="${C + 15}">${result.line}</text>
        ${result.sub ? `<text x="${C}" y="${C + 29}">${result.sub}</text>` : ''}`;
    }
    return `<circle cx="${C}" cy="${C}" r="${r - 6}"/><text x="${C}" y="${C + 4}">${hot === -1 ? L.cancel : L.drop}</text>`;
  }

  function render() {
    let s = SX.ringSVG({ ids, zh, hot, off: disabled, hub: hub() });
    if (paths.length > 1 && !busy && !result) {
      const bx = C + 98, by = C - 98;
      s += `<circle class="count" cx="${bx}" cy="${by}" r="13"/><text class="count-t" x="${bx}" y="${by + 4}">${paths.length}</text>`;
    }
    svg.innerHTML = s;
  }

  function which(x, y) {
    if (!start) start = [x, y];
    if (!armed && Math.hypot(x - start[0], y - start[1]) > ARM_PX) armed = true;
    return armed ? SX.which(x, y, ids.length) : -1;
  }

  function reset() {
    paths = [];
    hot = -2;
    busy = false;
    dropped = false;
    result = null;
    start = null;
    armed = false;
    render();
    // Restart the pop-in without relying on animation frames (paused while hidden).
    svg.classList.remove('in');
    void svg.getBoundingClientRect();
    svg.classList.add('in');
  }

  const hide = (ms = 0) => setTimeout(() => invoke('ring_hide'), ms);

  async function run(i) {
    const id = ids[i], a = SX.ACTIONS[id];
    if (disabled(i)) { result = { ok: false, line: L.none }; render(); return hide(1200); }
    busy = true;
    render();
    const targets = a.needsImage ? paths.filter((p) => IMG.test(p)) : paths;
    const res = await invoke('ring_action', { action: id, paths: targets }).catch((e) => ({ done: 0, failed: 1, error: String(e) }));
    busy = false;
    if (id === 'inspect') return hide(0);
    const ok = res.done > 0 && !res.failed;
    result = {
      ok,
      line: !res.done ? L.failed : id === 'copy' ? L.copied : L.done(res.done),
      sub: res.hidden ? L.hidden(res.hidden) : res.failed ? (res.error || '').slice(0, 22) : '',
    };
    render();
    hide(ok ? 1500 : 2600);
  }

  const toLocal = (p) => [p.x / devicePixelRatio, p.y / devicePixelRatio];

  T.webview.getCurrentWebview().onDragDropEvent((ev) => {
    const p = ev.payload;
    if (busy || result) return;
    if (p.type === 'enter') paths = p.paths || [];
    if (p.type === 'enter' || p.type === 'over') {
      const [x, y] = toLocal(p.position);
      const h = which(x, y);
      if (h !== hot) { hot = h; render(); }
    } else if (p.type === 'leave') {
      hot = -2;
      render();
    } else if (p.type === 'drop') {
      dropped = true;
      paths = p.paths || paths;
      const [x, y] = toLocal(p.position);
      const h = which(x, y);
      if (h >= 0) run(h); else hide(0);
    }
  });

  // The mouse went up: if it was not over the ring, close.
  T.event.listen('ring-release', () => setTimeout(() => { if (!dropped && !busy) invoke('ring_hide'); }, 300));
  T.event.listen('ring-show', reset);
  T.event.listen('settings-changed', (e) => applySettings(e.payload));
  reset();
  invoke('info').then((i) => applySettings(i.settings)).catch(() => {});
})();
