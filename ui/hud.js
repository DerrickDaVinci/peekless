// The guard card: appears bottom-right when something sensitive was copied.
(() => {
  const T = window.__TAURI__;
  const invoke = (c, a) => T.core.invoke(c, a);
  const $ = (s) => document.querySelector(s);
  const esc = (s) => String(s ?? '').replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
  const sysZh = (navigator.language || '').toLowerCase().startsWith('zh');
  let L, KIND;
  function setLang(lang) {
    const zh = lang === 'zh' ? true : lang === 'en' ? false : sysZh;
    L = zh
      ? { found: (n) => `发现 ${n} 处敏感信息`, done: (n) => `已遮住 ${n} 处，可以直接粘贴了`, cover: '遮住', skip: '不用', adjust: '调整…', undo: '撤销', and: '、' }
      : { found: (n) => `${n} sensitive item${n === 1 ? '' : 's'} found`, done: (n) => `Covered ${n} item${n === 1 ? '' : 's'}. Ready to paste.`, cover: 'Cover', skip: 'Ignore', adjust: 'Adjust…', undo: 'Undo', and: ', ' };
    KIND = zh
      ? { email: '邮箱', phone: '电话', card: '银行卡号', cn_id: '身份证', my_ic: 'MyKad', sg_nric: 'NRIC', hk_id: '香港身份证', iban: 'IBAN', api_key: 'API 密钥', token: '令牌', secret: '密码', connection: '数据库密码', private_key: '私钥', crypto: '钱包地址', ip: 'IP 地址' }
      : { email: 'email', phone: 'phone', card: 'card number', cn_id: 'China ID', my_ic: 'MyKad', sg_nric: 'NRIC', hk_id: 'HKID', iban: 'IBAN', api_key: 'API key', token: 'token', secret: 'password', connection: 'DB password', private_key: 'private key', crypto: 'wallet', ip: 'IP address' };

  }
  setLang('auto');
  let hud = null;
  let timer = 0;
  let deadline = 0;
  let left = 0;
  let paused = false;

  function kinds(items) {
    const seen = [];
    for (const it of items) {
      const k = KIND[it.kind] || it.kind;
      if (!seen.includes(k)) seen.push(k);
    }
    return seen.join(L.and);
  }

  function preview() {
    const pv = $('#pv');
    if (hud.kind === 'image') {
      const cv = document.createElement('canvas');
      const img = new Image();
      img.onload = () => {
        cv.width = hud.width;
        cv.height = hud.height;
        const ctx = cv.getContext('2d');
        ctx.drawImage(img, 0, 0);
        ctx.fillStyle = 'rgb(17,18,20)';
        for (const [x, y, w, h] of hud.boxes) ctx.fillRect(x, y, w, h);
      };
      img.src = hud.src;
      pv.replaceChildren(cv);
    } else {
      const html = esc(hud.excerpt).replace(/\[REDACTED:([a-z_]+)\]/g, (_, k) => `<b>${esc(KIND[k] || k)}</b>`);
      pv.innerHTML = `<pre>${html}</pre>`;
    }
  }

  function render() {
    const n = hud.items.length;
    const done = hud.mode === 'done';
    $('#card').classList.toggle('done', done);
    $('#ti').innerHTML = `${esc(done ? L.done(n) : L.found(n))}<small>${esc(kinds(hud.items))}</small>`;
    $('#acts').innerHTML = done
      ? `<button class="btn" type="button" data-a="undo">${L.undo}</button><button class="link" type="button" data-a="adjust">${L.adjust}</button>`
      : `<button class="btn primary" type="button" data-a="cover">${L.cover}</button><button class="btn" type="button" data-a="skip">${L.skip}</button><button class="link" type="button" data-a="adjust">${L.adjust}</button>`;
    preview();
    const secs = Math.max(5, hud.seconds || 15);
    countdown(done ? Math.min(7, secs) * 1000 : secs * 1000);
  }

  function countdown(ms) {
    left = ms;
    deadline = performance.now() + ms;
    cancelAnimationFrame(timer);
    const tick = (now) => {
      if (paused) { deadline = now + left; } else { left = Math.max(0, deadline - now); }
      $('#bar').style.transform = `scaleX(${left / ms})`;
      if (left <= 0) return invoke('hud_hide');
      timer = requestAnimationFrame(tick);
    };
    timer = requestAnimationFrame(tick);
  }

  async function load() {
    hud = await invoke('hud_payload');
    if (hud) render();
  }

  $('#acts').addEventListener('click', async (e) => {
    const a = e.target.closest('[data-a]')?.dataset.a;
    if (!a) return;
    try {
      if (a === 'cover') { await invoke('hud_accept'); hud.mode = 'done'; render(); }
      if (a === 'skip') await invoke('hud_hide');
      if (a === 'adjust') await invoke('hud_review');
      if (a === 'undo') await invoke('hud_undo');
    } catch (err) {
      $('#ti').innerHTML = `${esc(String(err))}`;
    }
  });
  $('#x').addEventListener('click', () => invoke('hud_hide'));
  document.addEventListener('mouseenter', () => { paused = true; });
  document.addEventListener('mouseleave', () => { paused = false; });

  T.event.listen('hud-update', load);
  T.event.listen('settings-changed', (e) => { setLang(e.payload.lang); SX.applyTheme(e.payload.theme); if (hud) render(); });
  invoke('info').then((i) => { setLang(i.settings && i.settings.lang); SX.applyTheme(i.settings && i.settings.theme); }).catch(() => {}).finally(load);
})();
