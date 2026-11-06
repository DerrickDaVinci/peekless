// Peekless main window. Plain JS, no build step: everything heavy happens in Rust.
(() => {
  const T = window.__TAURI__;
  const invoke = (cmd, args) => T.core.invoke(cmd, args);
  const $ = (s) => document.querySelector(s);
  const $$ = (s) => Array.from(document.querySelectorAll(s));
  const esc = (s) => String(s ?? '').replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
  let zh = SX.sysZh;

  // ------------------------------------------------------------ copy
  const STR = {
    tagline: ['遮遮 · 发送之前，先清理一下', 'Clean it before you send it'],
    navHome: ['首页', 'Home'], navGuard: ['剪贴板守护', 'Clipboard guard'], navRing: ['操作环', 'Action ring'], navKeys: ['快捷键', 'Shortcuts'],
    navRules: ['识别规则', 'Detection'], navOutput: ['输出', 'Output'], navGeneral: ['通用', 'General'], navAbout: ['关于', 'About'],
    guardOnPill: ['守护中', 'Guarding'], guardOffPill: ['守护已关闭', 'Guard off'],
    homeTitle: ['首页', 'Home'],
    homeSub: ['截图或复制文字后什么都不用做，有敏感信息时右下角会提醒你。照片可以直接拖进来检查。', 'Just screenshot or copy as usual: a card appears when something sensitive is found. Drop photos here to check them.'],
    dropTitle: ['把照片或截图拖到这里', 'Drop photos or screenshots here'],
    dropHintRing: (k) => (zh ? `或在访达里按住 ${k} 拖动文件，光标旁会弹出操作环。` : `Or hold ${k} while dragging files in Finder: the action ring opens at your cursor.`),
    dropHintPlain: ['看看它们暴露了什么，再另存一份干净的。', 'See what they reveal, then save clean copies.'],
    pick: ['选择文件…', 'Choose files…'], cleanClip: ['清理剪贴板', 'Clean clipboard'],
    statGuard: ['剪贴板守护', 'Clipboard guard'], statRing: ['操作环', 'Action ring'], statKey: ['快捷键', 'Shortcut'],
    on: ['已开启', 'On'], off: ['已关闭', 'Off'], none: ['未设置', 'Not set'],
    recent: ['最近处理', 'Recent'], clear: ['清空', 'Clear'],
    historyEmpty: ['处理过的内容会显示在这里。只记录文件名和数量，不保存任何内容。', 'Cleaned items show up here. Only names and counts are kept, never the content.'],
    guardSub: ['截图或复制的文字里有敏感信息时，屏幕右下角会提醒你。没有敏感信息时完全安静。', 'When a screenshot or copied text contains something sensitive, a card appears in the corner. Otherwise it stays silent.'],
    guardOn: ['自动守护剪贴板', 'Guard the clipboard'], guardOnS: ['只读取剪贴板的变化计数，内容变了才检查，几乎不耗电。', 'Only a change counter is polled; content is read only when it changes.'],
    whenFound: ['发现敏感信息后', 'When something is found'], whenFoundS: ['"直接遮住"会立刻打码，卡片上保留撤销。', '"Cover right away" covers it immediately and keeps an Undo.'],
    modeAsk: ['先问我', 'Ask me'], modeAuto: ['直接遮住', 'Cover right away'],
    cardTime: ['卡片停留时间', 'Card stays for'], cardTimeS: ['鼠标移到卡片上时会暂停计时。', 'The timer pauses while the pointer is over the card.'],
    watchImages: ['检查截图和图片', 'Check screenshots and images'], watchImagesS: ['用系统自带的文字识别读取图片里的文字。', 'Reads text in images with the OS text recognizer.'],
    watchText: ['检查复制的文字', 'Check copied text'], watchTextS: ['日志、.env、报错信息里的密钥会被发现。', 'Finds secrets in logs, .env files and error messages.'],
    tryIt: ['试一试', 'Try it'], tryItS: ['放一张示例截图到剪贴板，里面的信息全部是虚构的。几秒后右下角会弹出提醒。', 'Puts a sample screenshot with fictional data on the clipboard. A card appears a few seconds later.'],
    tryBtn: ['放入示例截图', 'Copy a sample'], tried: ['示例截图已放入剪贴板，留意屏幕右下角。', 'Sample copied. Watch the bottom-right corner.'],
    guardOffTry: ['先打开"自动守护剪贴板"。', 'Turn on the clipboard guard first.'],
    guardNote: ['1Password、Bitwarden、钥匙串等密码管理器复制的内容会被标记为"敏感"，Peekless 不会读取。', 'Entries that password managers (1Password, Bitwarden, Keychain…) mark as concealed are never read.'],
    ringSub: ['拖动文件时按住触发键，光标旁弹出操作环；拖到某一格松手即执行，拖到中间松手取消。', 'Hold the trigger key while dragging files: the ring opens at your cursor. Release on an action to run it, in the middle to cancel.'],
    ringOn: ['启用操作环', 'Enable the action ring'],
    ringOnS: (k) => (zh ? `在访达、桌面或任何地方拖动文件时按住 ${k}。不需要辅助功能权限。` : `Hold ${k} while dragging files anywhere. No Accessibility permission needed.`),
    ringKey: ['触发键', 'Trigger key'], ringKeyS: ['按住哪个键拖动文件时弹出操作环。', 'Which key opens the ring while dragging.'],
    ringActions: ['环上的操作', 'Actions on the ring'], ringActionsS: ['从正上方开始顺时针排列，至少保留两个。', 'Clockwise from the top. Keep at least two.'],
    resetDefault: ['恢复默认', 'Reset to default'], preview: ['预览', 'Preview'], keepTwo: ['至少要保留两个操作。', 'Keep at least two actions.'],
    keysSub: ['随时清理剪贴板，打开完整的确认窗口。', 'Review the clipboard in full, any time.'],
    scClean: ['清理剪贴板', 'Clean clipboard'], scCleanS: ['打开确认窗口，可以逐项取消，也能手动加框。', 'Opens the review window: toggle each box or draw your own.'],
    scHow: ['点击快捷键按钮，然后按下新的组合键。按 Esc 取消。', 'Click the shortcut, then press a new key combination. Esc cancels.'],
    recording: ['按下新的快捷键…', 'Press a new shortcut…'], needMod: ['请包含至少一个修饰键（⌘ ⌥ ⌃ ⇧）。', 'Include at least one modifier key.'],
    scTaken: ['这个组合被系统或其他应用占用了，换一个试试。', 'That combination is taken by the system or another app. Try another.'],
    ringGesture: ['操作环手势', 'Action ring gesture'], ringGestureS: (k) => (zh ? `按住 ${k} 拖动文件` : `Hold ${k} while dragging files`), configure: ['设置…', 'Configure…'],
    rulesSub: ['选择要自动识别和遮挡的内容。关掉的类型在守护、确认窗口和操作环里都不会再被标出。', 'Choose what gets detected. Disabled types are ignored by the guard, the review window and the ring.'],
    styleTitle: ['遮挡方式', 'Redaction style'], styleS: ['马赛克有时能被还原出文字，敏感信息建议用实心。', 'Mosaic can sometimes be reversed. Use solid for anything sensitive.'],
    solid: ['实心', 'Solid'], pixel: ['马赛克', 'Mosaic'],
    gCred: ['凭据', 'Credentials'], gId: ['证件与金融', 'IDs and money'], gContact: ['联系方式与网络', 'Contact and network'],
    outputSub: ['清理后的文件保存在哪里、叫什么。原文件永远不会被修改。', 'Where cleaned files go and what they are called. Originals are never modified.'],
    saveWhere: ['保存位置', 'Save to'], beside: ['原文件旁边', 'Next to the original'], folder: ['指定文件夹…', 'A folder…'],
    cleanName: ['清除隐藏信息后的文件名', 'Name after stripping info'], redactName: ['打码后的文件名', 'Name after covering text'],
    revealAfter: ['完成后在访达中显示', 'Show in Finder when done'], revealAfterS: ['用操作环或"清除隐藏信息"处理完后自动打开文件夹。', 'Reveal the new files after the ring or "Remove hidden info" runs.'],
    outputNote: ['同名文件已存在时会自动加上 -2、-3，不会覆盖。', 'If a file with that name exists, -2, -3… is added. Nothing is overwritten.'],
    lang: ['界面语言', 'Language'], theme: ['外观', 'Appearance'], followSystem: ['跟随系统', 'Follow system'], light: ['浅色', 'Light'], dark: ['深色', 'Dark'],
    login: ['开机自动启动', 'Open at login'], loginS: ['登录后在菜单栏待命，守护和操作环随时可用。', 'Waits in the menu bar after you log in.'],
    hidden: ['启动时不显示窗口', 'Start without a window'], hiddenS: ['只在菜单栏待命，需要时再点图标打开。', 'Stay in the menu bar until you open it.'],
    keepHistory: ['记录最近处理', 'Keep recent activity'], keepHistoryS: ['在首页列出最近处理过的内容。只保存文件名和数量，不保存内容。关掉后不再记录新的条目。', 'List recent work on Home. Only names and counts are kept. Turning it off stops new entries.'],
    promise: ['只读承诺', 'Privacy promise'], promiseS: ['不联网，不上传，没有统计。处理记录只保存文件名和数量。', 'No network, no uploads, no analytics. History keeps only names and counts.'],
    engine: ['文字识别', 'Text recognition'], engineS: ['使用系统自带的引擎：macOS Vision / Windows OCR。不下载任何模型。', 'Uses the engine built into your OS: macOS Vision / Windows OCR. No model downloads.'],
    cli: ['命令行', 'Command line'], cliS: ['同一个引擎，可以在终端和脚本里调用。', 'The same engine, for terminals and scripts.'],
    license: ['开源许可', 'License'], licenseS: ['MIT。自动识别不可能百分之百准确，每次复制前都会让你确认。', 'MIT. Automatic detection is never perfect, so you always confirm before anything is copied.'],
    found: (n) => (zh ? `找到 ${n} 处` : `${n} item${n === 1 ? '' : 's'} found`), nothing: ['没有发现敏感信息', 'Nothing sensitive found'],
    imgSub: (l) => (zh ? (l ? `识别了 ${l} 行文字。勾选的内容会被遮住。` : '勾选的内容会被遮住。') : l ? `Read ${l} line${l === 1 ? '' : 's'} of text. Checked items will be covered.` : 'Checked items will be covered.'),
    imgEmpty: ['没找到需要遮挡的内容。仍然可以在图上拖动，手动遮住任何区域。', 'Nothing to cover was found. Drag on the image to cover any area by hand.'],
    drag: ['在图上拖动可以手动添加遮挡，点击遮挡块可以取消。', 'Drag on the image to add a box. Click a box to toggle it.'],
    pixelWarn: ['马赛克有时能被还原出文字，敏感信息建议用实心遮挡。', 'Mosaic can sometimes be reversed. Use solid bars for anything sensitive.'],
    copyImg: ['复制干净的图片', 'Copy clean image'], copyImgNone: ['复制图片', 'Copy image'], save: ['另存为…', 'Save as…'], cancel: ['取消', 'Cancel'],
    txtTitle: (n) => (zh ? `文字里有 ${n} 处敏感信息` : `${n} sensitive item${n === 1 ? '' : 's'} in this text`), txtNone: ['这段文字看起来是干净的', 'This text looks clean'],
    txtSub: ['勾选的内容会替换成 [REDACTED:类型]。', 'Checked items become [REDACTED:type].'], copyTxt: ['复制干净的文字', 'Copy clean text'],
    copied: ['已复制，可以直接粘贴了。', 'Copied. Paste it wherever you like.'], saved: (p) => (zh ? `已保存：${p}` : `Saved: ${p}`),
    reading: ['正在读取剪贴板…', 'Reading clipboard…'], empty: ['剪贴板是空的。先截图或复制一段文字。', 'Your clipboard is empty. Take a screenshot or copy some text first.'],
    filesTitle: (n) => (zh ? `${n} 个文件` : `${n} file${n === 1 ? '' : 's'}`),
    filesSub: (n) => (zh ? (n ? `${n} 个文件带有隐藏信息。清理后会另存一份，原文件不变。` : '没有发现隐藏信息。') : n ? `${n} contain hidden metadata. Cleaning saves a copy; originals stay untouched.` : 'No hidden metadata found.'),
    cleanAll: ['清除隐藏信息', 'Remove hidden info'], redactThis: ['遮挡图中文字', 'Cover text in image'], noMeta: ['没有隐藏信息', 'No hidden metadata'],
    unsupported: ['暂不支持这种格式', 'This format is not supported yet'],
    gps: (v) => (zh ? `这张照片会暴露拍摄地点（${v}）。` : `This photo reveals where it was taken (${v}).`),
    cleanedTo: (n, b) => (zh ? `已另存为 ${n}，去掉了 ${b}` : `Saved as ${n}, ${b} removed`), reveal: ['显示', 'Show'],
    dropNow: ['松手开始检查', 'Drop to check'], dropNowS: ['照片、截图都可以', 'Photos and screenshots'],
    error: (m) => (zh ? `出错了：${m}` : `Something went wrong: ${m}`),
    justNow: ['刚刚', 'just now'], minAgo: (n) => (zh ? `${n} 分钟前` : `${n} min ago`), hrAgo: (n) => (zh ? `${n} 小时前` : `${n} h ago`),
  };
  const KIND = {
    email: ['邮箱', 'Email'], phone: ['电话', 'Phone'], card: ['银行卡号', 'Card number'], cn_id: ['中国身份证', 'China ID'],
    my_ic: ['马来西亚 MyKad', 'Malaysia MyKad'], sg_nric: ['新加坡 NRIC', 'Singapore NRIC'], hk_id: ['香港身份证', 'Hong Kong ID'],
    iban: ['IBAN 账号', 'IBAN'], api_key: ['API 密钥', 'API key'], token: ['登录令牌', 'Access token'], secret: ['密码 / 密钥', 'Password / secret'],
    connection: ['数据库密码', 'Connection password'], private_key: ['私钥', 'Private key'], crypto: ['钱包地址', 'Crypto wallet'], ip: ['IP 地址', 'IP address'],
    manual: ['手动遮挡', 'Manual box'],
  };
  const RULES = [
    ['gCred', [['api_key', 'sk-ant-api03-•••• · ghp_•••• · AKIA••••'], ['token', 'eyJhbGciOi••••.eyJzdWIi••••'], ['secret', 'password: •••• · 密码：••••'], ['connection', 'postgres://admin:••••@db'], ['private_key', '-----BEGIN PRIVATE KEY-----']]],
    ['gId', [['cn_id', '110105 1949 1231 002X'], ['my_ic', '900101-14-5678'], ['sg_nric', 'S1234567D'], ['hk_id', 'A123456(3)'], ['card', '4111 1111 1111 1111'], ['iban', 'GB82 WEST 1234 5698 …'], ['crypto', '0x5290…b4f0 · bc1q…']]],
    ['gContact', [['email', 'name@example.com'], ['phone', '138 1234 5678 · +60 12-345 6789'], ['ip', '203.0.113.7']]],
  ];
  const META = {
    gps: ['拍摄位置', 'Location'], camera: ['设备型号', 'Device'], lens: ['镜头', 'Lens'], taken: ['拍摄时间', 'Date taken'], software: ['软件版本', 'Software'],
    author: ['作者', 'Author'], copyright: ['版权', 'Copyright'], serial: ['序列号', 'Serial number'], comment: ['备注', 'Comment'],
    xmp: ['XMP 编辑记录', 'XMP data'], iptc: ['IPTC 信息', 'IPTC data'], png_text: ['嵌入文字', 'Embedded text'], c2pa: ['内容凭证', 'Content credentials'],
  };
  const MODKEY = { option: '⌥', command: '⌘', control: '⌃', shift: '⇧' };
  const t = (k) => { const v = STR[k]; return Array.isArray(v) ? v[zh ? 0 : 1] : v; };
  const tk = (pair) => (pair ? pair[zh ? 0 : 1] : '');

  // ------------------------------------------------------------ state
  const S = { page: 'home', info: { platform: 'macos', shortcutOk: true }, set: {}, defaults: {}, img: null, items: [], style: 'solid', text: null, files: [], results: {}, hot: -1, history: [], recording: false };

  function go(page) {
    if (S.page === 'image' && page !== 'image' && S.img) { invoke('discard', { id: S.img.id }).catch(() => {}); S.img = null; }
    if (page !== 'text') S.text = null;
    S.page = page;
    $('#app').dataset.page = page;
    const navPage = ['image', 'text', 'files'].includes(page) ? 'home' : page;
    $$('#nav button').forEach((b) => { if (b.dataset.page === navPage) b.setAttribute('aria-current', 'page'); else b.removeAttribute('aria-current'); });
    $('#main').scrollTop = 0;
    if (page === 'home') loadHistory();
  }

  let toastTimer = 0;
  function toast(msg) {
    const el = $('#toast');
    el.textContent = msg;
    el.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => el.classList.remove('show'), 3200);
  }
  const fail = (e) => toast(STR.error(String(e)));

  // ------------------------------------------------------------ paint
  function paint() {
    document.documentElement.lang = zh ? 'zh-CN' : 'en';
    $$('[data-i18n]').forEach((el) => { const v = t(el.dataset.i18n); if (typeof v === 'string') el.textContent = v; });
    const st = S.set;
    const k = MODKEY[st.ringModifier] || '⌥';
    const sc = SX.shortcutLabel(st.shortcut);
    // derived copy
    $('#dropHint').textContent = st.ring && S.info.platform === 'macos' ? STR.dropHintRing(k) : t('dropHintPlain');
    $('#ringOnS').textContent = STR.ringOnS(k);
    $('#ringGestureS').textContent = STR.ringGestureS(k);
    $('#cleanKbd').textContent = st.shortcutEnabled ? sc : '';
    $('#cleanKbd').hidden = !st.shortcutEnabled;
    $('#statGuard').textContent = st.guard ? t('on') : t('off');
    $('#statGuard').classList.toggle('on', !!st.guard);
    $('#statRing').textContent = st.ring ? `${t('on')} · ${k}` : t('off');
    $('#statRing').classList.toggle('on', !!st.ring);
    $('#statKey').textContent = st.shortcutEnabled ? sc : t('off');
    $('#guardPill').classList.toggle('on', !!st.guard);
    $('#guardPillText').textContent = st.guard ? t('guardOnPill') : t('guardOffPill');
    $('#ver').textContent = `v${S.info.version || ''}`;
    $('#aboutVer').textContent = `v${S.info.version || ''}`;
    $('#cliCode').textContent = 'peekless text notes.txt\npbpaste | peekless text | pbcopy\npeekless redact screenshot.png\npeekless clean ~/Pictures/*.jpg\npeekless clipboard';
    // controls
    $$('input[type=checkbox][data-set]').forEach((el) => { el.checked = !!st[el.dataset.set]; });
    $$('.seg[data-set]').forEach((seg) => {
      const v = String(st[seg.dataset.set]);
      seg.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.v === v)));
    });
    $$('input[type=range][data-set]').forEach((el) => { el.value = st[el.dataset.set]; });
    $('#cardSecondsOut').textContent = zh ? `${st.cardSeconds} 秒` : `${st.cardSeconds} s`;
    $$('input.text[data-set]').forEach((el) => { if (document.activeElement !== el) el.value = st[el.dataset.set] || ''; });
    $$('[data-needs]').forEach((el) => el.classList.toggle('off', !st[el.dataset.needs]));
    // shortcut
    if (!S.recording) {
      $('#recorder').textContent = sc || t('none');
      $('#recorder').classList.remove('rec');
    }
    $('#scWarnRow').hidden = S.info.shortcutOk !== false || !st.shortcutEnabled;
    $('#scWarn').textContent = t('scTaken');
    // output
    $('#outSeg').querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', String((b.dataset.v === 'folder') === !!st.outputDir)));
    $('#outDirText').textContent = st.outputDir || t('beside');
    previewNames();
    renderRules();
    renderRingEditor();
    renderHistory();
    if (S.page === 'image' && S.img) renderImageSide();
    if (S.page === 'text' && S.text) renderText();
    if (S.page === 'files') renderFiles();
  }

  function previewNames() {
    const c = $('input[data-set=cleanTag]').value || S.set.cleanTag || 'clean';
    const r = $('input[data-set=redactTag]').value || S.set.redactTag || 'redacted';
    $('#cleanPreview').textContent = `IMG_0001.${c}.jpg`;
    $('#redactPreview').textContent = `IMG_0001.${r}.png`;
  }

  function applyLang() {
    zh = SX.zhFor(S.set.lang);
    SX.applyTheme(S.set.theme);
    paint();
    invoke('set_lang', { zh, shortcut: S.set.shortcutEnabled ? SX.shortcutLabel(S.set.shortcut) : '' }).catch(() => {});
  }

  // ------------------------------------------------------------ settings binding
  async function save(patch) {
    S.set = { ...S.set, ...patch };
    applyLang();
    try {
      const r = await invoke('set_settings', { settings: S.set });
      if (r) { S.set = r.settings; S.info.shortcutOk = r.shortcutOk; applyLang(); }
    } catch (e) { fail(e); }
  }
  document.addEventListener('change', (e) => {
    const el = e.target;
    if (!el.dataset || !el.dataset.set) return;
    if (el.type === 'checkbox') save({ [el.dataset.set]: el.checked });
    else if (el.type === 'range') save({ [el.dataset.set]: Number(el.value) });
    else if (el.classList.contains('text')) save({ [el.dataset.set]: el.value.trim() });
  });
  document.addEventListener('input', (e) => {
    if (e.target.matches('input[type=range][data-set=cardSeconds]')) $('#cardSecondsOut').textContent = zh ? `${e.target.value} 秒` : `${e.target.value} s`;
    if (e.target.matches('input.text[data-set]')) previewNames();
  });
  document.addEventListener('click', (e) => {
    const b = e.target.closest('.seg[data-set] button');
    if (b) {
      const v = b.dataset.v;
      save({ [b.parentElement.dataset.set]: v === 'true' ? true : v === 'false' ? false : v });
      return;
    }
    const g = e.target.closest('[data-goto]');
    if (g) go(g.dataset.goto);
  });
  $('#nav').addEventListener('click', (e) => { const b = e.target.closest('button[data-page]'); if (b) go(b.dataset.page); });
  $('#guardPill').addEventListener('click', () => save({ guard: !S.set.guard }));
  $('#trySample').addEventListener('click', async () => {
    if (!S.set.guard) return toast(t('guardOffTry'));
    try { await invoke('try_sample'); toast(t('tried')); } catch (e) { fail(e); }
  });

  // output folder
  $('#outSeg').addEventListener('click', async (e) => {
    const b = e.target.closest('button[data-v]');
    if (!b) return;
    if (b.dataset.v === 'beside') return save({ outputDir: null });
    const dir = await invoke('pick_folder').catch(() => null);
    if (dir) save({ outputDir: dir });
  });

  // rules
  function renderRules() {
    const off = new Set(S.set.disabledKinds || []);
    $('#ruleGroups').innerHTML = RULES.map(([g, kinds]) => `<h3 class="group-title">${esc(t(g))}</h3><div class="group">${kinds.map(([k, ex]) => `
      <div class="row"><div><b>${esc(tk(KIND[k]))}</b><p class="mono">${esc(ex)}</p></div>
      <label class="switch"><input type="checkbox" data-kind="${k}" ${off.has(k) ? '' : 'checked'}><i></i></label></div>`).join('')}</div>`).join('');
  }
  $('#ruleGroups').addEventListener('change', (e) => {
    const k = e.target.dataset.kind;
    if (!k) return;
    const off = new Set(S.set.disabledKinds || []);
    if (e.target.checked) off.delete(k); else off.add(k);
    save({ disabledKinds: [...off] });
  });

  // ring editor: every action is listed; enabled ones keep their order first.
  function ringOrder() {
    const on = (S.set.ringActions || SX.ORDER).filter((a) => SX.ACTIONS[a]);
    return [...on, ...SX.ORDER.filter((a) => !on.includes(a))];
  }
  function renderRingEditor() {
    const on = S.set.ringActions || SX.ORDER;
    const all = ringOrder();
    $('#ringList').innerHTML = all.map((id, i) => {
      const a = SX.ACTIONS[id], enabled = on.includes(id), pos = enabled ? on.indexOf(id) + 1 : '–';
      return `<li class="${enabled ? '' : 'off'}" data-id="${id}">
        <span class="pos">${pos}</span>
        <span class="ai"><svg viewBox="-12 -12 24 24">${a.icon}</svg></span>
        <span class="nm"><b>${esc(tk(a.label))}</b><span>${esc(tk(a.desc))}</span></span>
        <span class="ctl"><button class="mv" type="button" data-mv="-1" ${!enabled || pos === 1 ? 'disabled' : ''} aria-label="up">↑</button><button class="mv" type="button" data-mv="1" ${!enabled || pos === on.length ? 'disabled' : ''} aria-label="down">↓</button>
        <label class="switch"><input type="checkbox" data-ring="${id}" ${enabled ? 'checked' : ''}><i></i></label></span></li>`;
    }).join('');
    const svg = $('#ringPreview');
    svg.classList.add('ring-svg');
    svg.innerHTML = SX.ringSVG({ ids: on, zh });
  }
  $('#ringList').addEventListener('click', (e) => {
    const mv = e.target.closest('[data-mv]');
    if (!mv) return;
    const id = mv.closest('li').dataset.id;
    const on = [...(S.set.ringActions || SX.ORDER)];
    const i = on.indexOf(id), j = i + Number(mv.dataset.mv);
    if (i < 0 || j < 0 || j >= on.length) return;
    [on[i], on[j]] = [on[j], on[i]];
    save({ ringActions: on });
  });
  $('#ringList').addEventListener('change', (e) => {
    const id = e.target.dataset.ring;
    if (!id) return;
    let on = [...(S.set.ringActions || SX.ORDER)];
    if (e.target.checked) on.push(id);
    else if (on.length <= 2) { e.target.checked = true; return toast(t('keepTwo')); }
    else on = on.filter((a) => a !== id);
    save({ ringActions: on });
  });
  $('#ringReset').addEventListener('click', () => save({ ringActions: [...S.defaults.ringActions], ringModifier: S.defaults.ringModifier }));

  // shortcut recorder
  $('#recorder').addEventListener('click', () => {
    S.recording = true;
    $('#recorder').textContent = t('recording');
    $('#recorder').classList.add('rec');
  });
  $('#scReset').addEventListener('click', () => save({ shortcut: S.defaults.shortcut, shortcutEnabled: true }));
  window.addEventListener('keydown', async (e) => {
    if (!S.recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') { S.recording = false; paint(); return; }
    if (/^(Meta|Control|Alt|Shift|OS)/.test(e.code) || ['Meta', 'Control', 'Alt', 'Shift'].includes(e.key)) return;
    const mods = [];
    if (e.metaKey) mods.push(SX.isMac ? 'Cmd' : 'Super');
    if (e.ctrlKey) mods.push('Ctrl');
    if (e.altKey) mods.push('Alt');
    if (e.shiftKey) mods.push('Shift');
    if (!mods.length && !/^F\d+$/.test(e.code)) { $('#recorder').textContent = t('needMod'); return; }
    const accel = [...mods, e.code].join('+');
    S.recording = false;
    if (await invoke('check_shortcut', { shortcut: accel }).catch(() => false)) save({ shortcut: accel, shortcutEnabled: true });
    else paint();
  }, true);

  // ------------------------------------------------------------ history
  async function loadHistory() {
    S.history = await invoke('history').catch(() => []);
    renderHistory();
  }
  const ICON = {
    cover: '<path d="M4 7h16M4 17h10"/><rect x="4" y="10" width="12" height="4" rx="1.5" fill="currentColor" stroke="none"/>',
    text: '<path d="M5 6h14M5 10h14M5 14h9M5 18h6"/>',
    strip: '<path d="M12 21c-4-4.5-7-8-7-11.5a7 7 0 0 1 14 0C19 13 16 16.5 12 21z"/><path d="m4 4 16 16"/>',
    copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
  };
  function when(sec) {
    const d = Date.now() / 1000 - sec;
    if (d < 60) return t('justNow');
    if (d < 3600) return STR.minAgo(Math.floor(d / 60));
    if (d < 86400) return STR.hrAgo(Math.floor(d / 3600));
    return new Date(sec * 1000).toLocaleDateString(zh ? 'zh-CN' : undefined, { month: 'short', day: 'numeric' });
  }
  function describe(h) {
    const label = h.label && /^\d+$/.test(h.label) ? (zh ? `${h.label} 个文件` : `${h.label} files`) : h.label;
    const n = h.hidden;
    const Z = {
      'clipboard:cover': `剪贴板截图 · 遮住 ${n} 处`, 'clipboard:text': `剪贴板文字 · 遮住 ${n} 处`,
      'review:copy': `复制了打码图片 · 遮住 ${n} 处`, 'review:cover': `另存了打码图片 · ${label}`, 'review:text': `复制了干净文字 · 遮住 ${n} 处`,
      'ring:strip': `去掉隐藏信息 · ${label}`, 'files:strip': `去掉隐藏信息 · ${label}`, 'ring:cover': `遮住文字 · ${label} · ${n} 处`, 'ring:copy': `复制了干净图片 · ${label}`,
    };
    const E = {
      'clipboard:cover': `Clipboard screenshot · ${n} covered`, 'clipboard:text': `Clipboard text · ${n} covered`,
      'review:copy': `Copied a covered image · ${n} covered`, 'review:cover': `Saved a covered image · ${label}`, 'review:text': `Copied clean text · ${n} covered`,
      'ring:strip': `Stripped info · ${label}`, 'files:strip': `Stripped info · ${label}`, 'ring:cover': `Covered text · ${label} · ${n}`, 'ring:copy': `Copied clean image · ${label}`,
    };
    return (zh ? Z : E)[`${h.source}:${h.action}`] || `${h.action} · ${label}`;
  }
  function renderHistory() {
    const H = S.history || [];
    $('#clearHistory').hidden = !H.length;
    $('#history').innerHTML = H.length ? H.slice(0, 12).map((h, i) => `<li>
      <span class="ic ${h.action === 'text' ? 'cover' : h.action}"><svg viewBox="0 0 24 24">${ICON[h.action] || ICON.cover}</svg></span>
      <div><div class="t">${esc(describe(h))}</div></div>
      <span class="when">${esc(when(h.time))}${h.outputs && h.outputs.length ? `<button class="link" type="button" data-reveal-h="${i}">${esc(t('reveal'))}</button>` : ''}</span></li>`).join('')
      : `<li class="empty">${esc(t('historyEmpty'))}</li>`;
  }
  $('#history').addEventListener('click', (e) => {
    const b = e.target.closest('[data-reveal-h]');
    if (b) invoke('reveal_many', { paths: S.history[+b.dataset.revealH].outputs });
  });
  $('#clearHistory').addEventListener('click', async () => { await invoke('clear_history').catch(() => {}); S.history = []; renderHistory(); });

  // ------------------------------------------------------------ clipboard review
  async function cleanClipboard() {
    go('image');
    $('#busy').hidden = false;
    $('#imgTitle').textContent = t('reading');
    $('#imgSub').textContent = '';
    $('#imgList').innerHTML = '';
    try { route(await invoke('clean_clipboard')); } catch (e) { go('home'); fail(e); } finally { $('#busy').hidden = true; }
  }
  function route(r) {
    if (r.type === 'image') return showImage(r);
    if (r.type === 'text') return showText(r);
    go('home');
    toast(t('empty'));
  }

  // ------------------------------------------------------------ image review
  const cv = $('#cv');
  const ctx = cv.getContext('2d');
  let scale = 1;
  let drag = null;

  function showImage(r) {
    S.img = { ...r, el: new Image() };
    S.items = r.findings.map((f) => ({ ...f, on: true }));
    S.style = S.set.style || 'solid';
    S.img.el.onload = () => layout();
    S.img.el.src = r.src;
    go('image');
    renderImageSide();
  }
  function renderImageSide() {
    const n = S.items.filter((i) => i.kind !== 'manual').length;
    $('#imgTitle').textContent = n ? STR.found(n) : t('nothing');
    $('#imgSub').textContent = STR.imgSub(S.img.lines);
    const on = S.items.some((i) => i.on);
    $('#imgCopy').innerHTML = `${esc(t(on ? 'copyImg' : 'copyImgNone'))} <kbd>⏎</kbd>`;
    $('#imgList').innerHTML = S.items.length
      ? S.items.map((it, i) => `<li data-i="${i}" class="${it.on ? '' : 'off'} ${S.hot === i ? 'hot' : ''}">
          <input type="checkbox" ${it.on ? 'checked' : ''} tabindex="-1" aria-label="${esc(tk(KIND[it.kind] || KIND.manual))}">
          <span class="k">${esc(tk(KIND[it.kind] || KIND.manual))}</span>
          <span class="sev s${it.kind === 'manual' ? 0 : it.severity}"></span>
          <span class="p">${esc(it.preview)}</span></li>`).join('')
      : `<li class="empty">${esc(t('imgEmpty'))}</li>`;
    $$('#styleSeg button').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.s === S.style)));
    $('#pixelWarn').hidden = S.style !== 'pixelate';
  }
  function layout() {
    if (!S.img || !S.img.el.naturalWidth) return;
    const stage = $('#stage');
    const pad = 44, w = S.img.width, h = S.img.height;
    scale = Math.min((stage.clientWidth - pad) / w, (stage.clientHeight - pad) / h, 1);
    const dpr = window.devicePixelRatio || 1;
    cv.style.width = `${Math.round(w * scale)}px`;
    cv.style.height = `${Math.round(h * scale)}px`;
    cv.width = Math.round(w * scale * dpr);
    cv.height = Math.round(h * scale * dpr);
    ctx.setTransform(scale * dpr, 0, 0, scale * dpr, 0, 0);
    draw();
  }
  function draw() {
    if (!S.img) return;
    const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim();
    ctx.clearRect(0, 0, S.img.width, S.img.height);
    ctx.drawImage(S.img.el, 0, 0);
    const px = 1 / scale;
    S.items.forEach((it, i) => {
      for (const [x, y, w, h] of it.boxes) {
        if (it.on) {
          // Opaque, like the result; translucent only while hovered so you can check what is underneath.
          ctx.fillStyle = S.hot === i ? 'rgba(17,18,20,0.55)' : 'rgb(17,18,20)';
          ctx.fillRect(x, y, w, h);
        } else {
          ctx.setLineDash([5 * px, 4 * px]);
          ctx.strokeStyle = accent;
          ctx.lineWidth = 1.5 * px;
          ctx.strokeRect(x, y, w, h);
          ctx.setLineDash([]);
        }
        if (S.hot === i) {
          ctx.strokeStyle = accent;
          ctx.lineWidth = 2.5 * px;
          ctx.strokeRect(x - 2 * px, y - 2 * px, w + 4 * px, h + 4 * px);
        }
      }
    });
    if (drag && drag.moved) {
      const [x, y, w, h] = rect(drag);
      ctx.fillStyle = 'rgba(31,85,230,0.18)';
      ctx.fillRect(x, y, w, h);
      ctx.strokeStyle = accent;
      ctx.lineWidth = 1.5 * px;
      ctx.strokeRect(x, y, w, h);
    }
  }
  const rect = (d) => [Math.min(d.x0, d.x1), Math.min(d.y0, d.y1), Math.abs(d.x1 - d.x0), Math.abs(d.y1 - d.y0)];
  const toImg = (e) => { const r = cv.getBoundingClientRect(); return [(e.clientX - r.left) / scale, (e.clientY - r.top) / scale]; };
  const hit = (x, y) => {
    for (let i = S.items.length - 1; i >= 0; i--) {
      if (S.items[i].boxes.some(([bx, by, bw, bh]) => x >= bx && x <= bx + bw && y >= by && y <= by + bh)) return i;
    }
    return -1;
  };
  cv.addEventListener('pointerdown', (e) => { const [x, y] = toImg(e); drag = { x0: x, y0: y, x1: x, y1: y, moved: false }; cv.setPointerCapture(e.pointerId); });
  cv.addEventListener('pointermove', (e) => {
    const [x, y] = toImg(e);
    if (drag) {
      drag.x1 = x; drag.y1 = y;
      if (Math.hypot(drag.x1 - drag.x0, drag.y1 - drag.y0) * scale > 4) drag.moved = true;
      draw();
      return;
    }
    const i = hit(x, y);
    cv.style.cursor = i >= 0 ? 'pointer' : 'crosshair';
    if (i !== S.hot) { S.hot = i; draw(); renderImageSide(); }
  });
  cv.addEventListener('pointerup', () => {
    if (!drag) return;
    const d = drag;
    drag = null;
    if (d.moved) {
      const [x, y, w, h] = rect(d);
      if (w * scale > 6 && h * scale > 6) S.items.push({ kind: 'manual', preview: `${Math.round(w)} × ${Math.round(h)} px`, severity: 0, boxes: [[x, y, w, h]], on: true });
    } else {
      const i = hit(d.x0, d.y0);
      if (i >= 0) S.items[i].on = !S.items[i].on;
    }
    renderImageSide();
    draw();
  });
  cv.addEventListener('pointerleave', () => { if (!drag && S.hot !== -1) { S.hot = -1; draw(); renderImageSide(); } });
  $('#imgList').addEventListener('click', (e) => {
    const li = e.target.closest('li[data-i]');
    if (!li) return;
    const it = S.items[+li.dataset.i];
    it.on = !it.on;
    renderImageSide();
    draw();
  });
  $('#imgList').addEventListener('mouseover', (e) => {
    const li = e.target.closest('li[data-i]');
    const i = li ? +li.dataset.i : -1;
    if (i !== S.hot) { S.hot = i; draw(); }
  });
  $('#styleSeg').addEventListener('click', (e) => { const b = e.target.closest('button[data-s]'); if (b) { S.style = b.dataset.s; renderImageSide(); } });

  async function applyImage(action) {
    const boxes = S.items.filter((i) => i.on).flatMap((i) => i.boxes);
    try {
      const res = await invoke('apply_image', { id: S.img.id, boxes, style: S.style, action });
      if (res === 'cancelled') return;
      finished(action === 'copy' ? t('copied') : STR.saved(res), action === 'copy');
    } catch (e) { fail(e); }
  }
  $('#imgCopy').addEventListener('click', () => applyImage('copy'));
  $('#imgSave').addEventListener('click', () => applyImage('save'));
  $('#imgCancel').addEventListener('click', () => back());

  function finished(msg, hide) {
    toast(msg);
    const toFiles = S.img && S.img.origin === 'file';
    go(toFiles ? 'files' : 'home');
    if (toFiles) renderFiles();
    if (hide) setTimeout(() => invoke('hide_window').catch(() => {}), 900);
  }
  function back() {
    const toFiles = S.img && S.img.origin === 'file' && S.files.length;
    go(toFiles ? 'files' : 'home');
  }

  // ------------------------------------------------------------ text review
  function showText(r) {
    S.text = { text: r.text, chars: Array.from(r.text) };
    S.items = r.findings.map((f) => ({ ...f, on: true }));
    go('text');
    S.text = { text: r.text, chars: Array.from(r.text) };
    renderText();
  }
  function renderText() {
    const { chars } = S.text;
    let html = '', at = 0;
    S.items.forEach((it, i) => {
      html += esc(chars.slice(at, it.start).join(''));
      html += `<mark data-i="${i}" class="${it.on ? '' : 'off'} ${S.hot === i ? 'hot' : ''}">${esc(chars.slice(it.start, it.end).join(''))}</mark>`;
      at = it.end;
    });
    $('#txt').innerHTML = html + esc(chars.slice(at).join(''));
    const n = S.items.length;
    $('#txtTitle').textContent = n ? STR.txtTitle(n) : t('txtNone');
    $('#txtCopy').innerHTML = `${esc(t('copyTxt'))} <kbd>⏎</kbd>`;
    $('#txtCopy').disabled = !S.items.some((i) => i.on);
    $('#txtList').innerHTML = S.items.map((it, i) => `<li data-i="${i}" class="${it.on ? '' : 'off'} ${S.hot === i ? 'hot' : ''}">
        <input type="checkbox" ${it.on ? 'checked' : ''} tabindex="-1" aria-label="${esc(tk(KIND[it.kind]))}">
        <span class="k">${esc(tk(KIND[it.kind] || KIND.manual))}</span><span class="sev s${it.severity}"></span>
        <span class="p">${esc(it.preview)}</span></li>`).join('');
  }
  const toggleText = (e) => { const el = e.target.closest('[data-i]'); if (!el) return; const it = S.items[+el.dataset.i]; it.on = !it.on; renderText(); };
  $('#txt').addEventListener('click', toggleText);
  $('#txtList').addEventListener('click', toggleText);
  $('#txtCopy').addEventListener('click', async () => {
    const ranges = S.items.filter((i) => i.on).map((i) => [i.start, i.end, i.kind]);
    try { await invoke('apply_text', { text: S.text.text, ranges }); finished(t('copied'), true); } catch (e) { fail(e); }
  });
  $('#txtCancel').addEventListener('click', () => go('home'));

  // ------------------------------------------------------------ files
  async function openFiles(paths) {
    if (!paths || !paths.length) return;
    try {
      S.files = await invoke('inspect_files', { paths });
      S.results = {};
      go('files');
      renderFiles();
    } catch (e) { fail(e); }
  }
  const human = (b) => (b >= 1048576 ? `${(b / 1048576).toFixed(1)} MB` : b >= 1024 ? `${Math.round(b / 1024)} KB` : `${b} B`);
  function renderFiles() {
    const withMeta = S.files.filter((f) => f.items.length).length;
    $('#filesTitle').textContent = STR.filesTitle(S.files.length);
    $('#filesSub').textContent = STR.filesSub(withMeta);
    $('#cleanAll').hidden = !S.files.some((f) => f.can_clean && f.items.length && !S.results[f.path]);
    $('#fileList').innerHTML = S.files.map((f, i) => {
      const res = S.results[f.path];
      const items = f.items.map((it) => `<span class="item r${it.risk}"><b>${esc(tk(META[it.key] || [it.key, it.key]))}</b><span title="${esc(it.value)}">${esc(it.value)}</span></span>`).join('');
      const status = !f.items.length ? `<span class="clean-ok">${esc(t('noMeta'))}</span>` : !f.can_clean ? `<span class="muted small">${esc(t('unsupported'))}</span>` : '';
      const result = res ? (res.error
        ? `<div class="result err">${esc(res.error)}</div>`
        : `<div class="result">${esc(STR.cleanedTo(res.out.split(/[\\/]/).pop(), human(Math.max(0, res.removed_bytes))))}<button class="btn ghost" type="button" data-reveal="${esc(res.out)}">${esc(t('reveal'))}</button></div>`) : '';
      return `<div class="file">
        <div><div class="nm" title="${esc(f.path)}">${esc(f.name)}</div><div class="meta">${esc(f.format.toUpperCase())} · ${human(f.size)}</div></div>
        <div class="acts">${status}${f.can_redact ? `<button class="btn" type="button" data-redact="${i}">${esc(t('redactThis'))}</button>` : ''}</div>
        ${f.gps ? `<p class="gps">${esc(STR.gps(f.gps.map((v) => v.toFixed(4)).join(', ')))}</p>` : ''}
        ${items ? `<div class="items">${items}</div>` : ''}${result}</div>`;
    }).join('');
  }
  $('#fileList').addEventListener('click', async (e) => {
    const r = e.target.closest('[data-redact]');
    if (r) {
      const f = S.files[+r.dataset.redact];
      go('image');
      $('#busy').hidden = false;
      $('#imgTitle').textContent = f.name;
      $('#imgList').innerHTML = '';
      try { showImage(await invoke('open_image', { path: f.path })); } catch (err) { go('files'); fail(err); } finally { $('#busy').hidden = true; }
      return;
    }
    const v = e.target.closest('[data-reveal]');
    if (v) invoke('reveal_many', { paths: [v.dataset.reveal] });
  });
  $('#cleanAll').addEventListener('click', async () => {
    const paths = S.files.filter((f) => f.can_clean && f.items.length && !S.results[f.path]).map((f) => f.path);
    try { (await invoke('clean_files', { paths })).forEach((r) => { S.results[r.path] = r; }); renderFiles(); } catch (e) { fail(e); }
  });
  $('#pickBtn').addEventListener('click', async () => { try { openFiles(await invoke('pick_files')); } catch (e) { fail(e); } });
  $('#cleanBtn').addEventListener('click', cleanClipboard);

  // ------------------------------------------------------------ keyboard, drop, events
  document.addEventListener('keydown', (e) => {
    if (S.recording || e.target.matches('input.text')) return;
    if (e.key === 'Escape' && ['image', 'text', 'files'].includes(S.page)) { e.preventDefault(); S.page === 'image' ? back() : go('home'); }
    if (e.key === 'Enter' && !e.isComposing) {
      if (S.page === 'image' && S.img) { e.preventDefault(); applyImage('copy'); }
      if (S.page === 'text' && S.text && !$('#txtCopy').disabled) { e.preventDefault(); $('#txtCopy').click(); }
    }
  });
  window.addEventListener('resize', () => { if (S.page === 'image') layout(); });

  T.event.listen('trigger-clean', () => cleanClipboard());
  T.event.listen('open-review', (e) => route(e.payload));
  T.event.listen('open-files', (e) => openFiles(e.payload));
  T.event.listen('history-changed', () => loadHistory());
  T.event.listen('settings-changed', (e) => { S.set = e.payload; applyLang(); });
  try {
    T.webview.getCurrentWebview().onDragDropEvent((ev) => {
      const p = ev.payload;
      if (p.type === 'enter' || p.type === 'over') { $('#drop').hidden = false; $('#dropzone').classList.add('hot'); }
      else if (p.type === 'leave') { $('#drop').hidden = true; $('#dropzone').classList.remove('hot'); }
      else if (p.type === 'drop') { $('#drop').hidden = true; $('#dropzone').classList.remove('hot'); openFiles(p.paths); }
    });
  } catch (_) { /* drag and drop unavailable */ }

  invoke('info').then((i) => {
    S.info = i;
    S.set = i.settings;
    S.defaults = i.defaults;
    if (i.platform !== 'macos') $('#app').classList.add('os-win');
    applyLang();
    loadHistory();
  }).catch(fail);
  go('home');
})();
