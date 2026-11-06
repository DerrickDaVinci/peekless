// Shared by the main window, the guard card and the action ring.
window.SX = (() => {
  const sysZh = (navigator.language || '').toLowerCase().startsWith('zh');
  const isMac = /Mac/i.test(navigator.platform || navigator.userAgent);

  // Ring actions. Icons are 24×24 strokes drawn around (0,0).
  const ACTIONS = {
    inspect: { label: ['检查', 'Inspect'], desc: ['在主窗口列出文件暴露了什么', 'List what the files reveal'], needsImage: false,
      icon: '<path d="M-10 0c3-5 6.5-7 10-7s7 2 10 7c-3 5-6.5 7-10 7s-7-2-10-7z"/><circle cx="0" cy="0" r="3"/>' },
    strip: { label: ['去掉定位', 'Strip info'], desc: ['无损清除定位、设备等隐藏信息，另存一份', 'Remove location and device data losslessly, save a copy'], needsImage: true,
      icon: '<path d="M0 9c-4-4.5-7-8-7-11.5a7 7 0 0 1 14 0C7 1 4 4.5 0 9z"/><path d="M-9 -9l18 18"/>' },
    copy: { label: ['复制干净图', 'Copy clean'], desc: ['打码并去掉信息，直接放进剪贴板', 'Cover and strip, then put it on the clipboard'], needsImage: true,
      icon: '<rect x="-8" y="-9" width="12" height="14" rx="2"/><path d="M-3 9h8a3 3 0 0 0 3-3V-4"/><rect class="solid" x="-6" y="-3" width="8" height="3" rx="1"/>' },
    cover: { label: ['遮住文字', 'Cover text'], desc: ['打码图中的敏感文字，另存为新图片', 'Cover sensitive text and save a new image'], needsImage: true,
      icon: '<path d="M-9 -6h18M-9 6h12"/><rect class="solid" x="-9" y="-2.5" width="13" height="5" rx="1.5"/>' },
  };
  const ORDER = ['inspect', 'strip', 'copy', 'cover'];

  const C = 150, R = 138, r = 52;
  const pt = (deg, rad) => [C + rad * Math.cos((deg * Math.PI) / 180), C + rad * Math.sin((deg * Math.PI) / 180)];
  const mid = (i, n) => -90 + (i * 360) / n;
  function arc(i, n) {
    const span = 360 / n, gap = 2.2;
    const a0 = mid(i, n) - span / 2 + gap, a1 = mid(i, n) + span / 2 - gap;
    const big = a1 - a0 > 180 ? 1 : 0;
    const [x0, y0] = pt(a0, R), [x1, y1] = pt(a1, R), [x2, y2] = pt(a1, r), [x3, y3] = pt(a0, r);
    return `M${x0} ${y0}A${R} ${R} 0 ${big} 1 ${x1} ${y1}L${x2} ${y2}A${r} ${r} 0 ${big} 0 ${x3} ${y3}Z`;
  }

  /** SVG markup for the ring. opts: ids, zh, hot, off(i)->bool, hub (inner markup). */
  function ringSVG({ ids, zh, hot = -2, off = () => false, hub = '' }) {
    const n = ids.length;
    let s = '';
    ids.forEach((id, i) => {
      const a = ACTIONS[id];
      const [lx, ly] = pt(mid(i, n), (R + r) / 2);
      s += `<g class="rs ${hot === i ? 'hot' : ''} ${off(i) ? 'off' : ''}"><path d="${arc(i, n)}"/><g class="ic" transform="translate(${lx} ${ly - 10})">${a.icon}</g><text x="${lx}" y="${ly + 18}">${a.label[zh ? 0 : 1]}</text></g>`;
    });
    return s + `<g class="rh ${hot === -1 ? 'hot' : ''}">${hub || `<circle cx="${C}" cy="${C}" r="${r - 6}"/>`}</g>`;
  }

  /** Which segment is under (x, y) in ring coordinates: -1 hub, -2 outside. */
  function which(x, y, n) {
    const dx = x - C, dy = y - C, d = Math.hypot(dx, dy);
    if (d < r) return -1;
    if (d > R + 8) return -2;
    const span = 360 / n;
    const deg = ((Math.atan2(dy, dx) * 180) / Math.PI + 90 + span / 2 + 720) % 360;
    return Math.floor(deg / span) % n;
  }

  function applyTheme(theme) {
    const root = document.documentElement;
    if (theme === 'light' || theme === 'dark') root.dataset.theme = theme;
    else delete root.dataset.theme;
  }

  const zhFor = (lang) => (lang === 'zh' ? true : lang === 'en' ? false : sysZh);

  /** "CmdOrCtrl+Alt+KeyR" -> "⌘⌥R" on macOS, "Ctrl+Alt+R" on Windows. */
  function shortcutLabel(accel) {
    if (!accel) return '';
    const MAC = { cmdorctrl: '⌘', commandorcontrol: '⌘', cmd: '⌘', command: '⌘', super: '⌘', meta: '⌘', ctrl: '⌃', control: '⌃', alt: '⌥', option: '⌥', shift: '⇧' };
    const WIN = { cmdorctrl: 'Ctrl', commandorcontrol: 'Ctrl', cmd: 'Win', command: 'Win', super: 'Win', meta: 'Win', ctrl: 'Ctrl', control: 'Ctrl', alt: 'Alt', option: 'Alt', shift: 'Shift' };
    const ARROWS = { arrowup: '↑', arrowdown: '↓', arrowleft: '←', arrowright: '→' };
    const parts = accel.split('+');
    const key = parts.pop();
    const k = key.replace(/^Key/, '').replace(/^Digit/, '').replace(/^Numpad/, 'Num ');
    const keyLabel = ARROWS[key.toLowerCase()] || k;
    const mods = parts.map((p) => (isMac ? MAC : WIN)[p.toLowerCase()] || p);
    return isMac ? mods.join('') + keyLabel : [...mods, keyLabel].join('+');
  }

  return { ACTIONS, ORDER, ringSVG, which, applyTheme, zhFor, shortcutLabel, isMac, sysZh, C, R, r };
})();
