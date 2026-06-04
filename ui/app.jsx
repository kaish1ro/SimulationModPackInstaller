/* global React, TitleBar, BuildHero, Tabs, StatusBar, InstallPanel, ChangelogPanel,
   SettingsPanel, AboutPanel, TOGGLE_DEFS, nowTs,
   useTweaks, TweaksPanel, TweakSection, TweakRadio, TweakToggle */
const { useState, useEffect, useRef, useCallback } = React;

// ── Tauri API ──────────────────────────────────────────────────────────────
async function invoke(cmd, args) {
  const T = window.__TAURI__;
  if (!T) throw new Error('Tauri not available');
  const fn = T.tauri?.invoke ?? T.invoke;
  return fn(cmd, args ?? {});
}
function listenEvent(ev, cb) {
  const T = window.__TAURI__;
  const fn = T?.event?.listen ?? T?.listen;
  return fn ? fn(ev, cb) : Promise.resolve(() => {});
}
async function tauriWindow() {
  const T = window.__TAURI__;
  return T?.window?.appWindow ?? T?.window?.getCurrent?.();
}

// ── BUILD constants ────────────────────────────────────────────────────────
const BUILD = {
  title: "СИМУЛЯЦИЯ",
  version: "0.1.0",
  mc: "1.20.1",
  loader: "Forge",
  mods: "131",
  size: "~4 ГБ",
  dims: "9",
  bosses: "20+",
  channel: "pre-alpha",
};

const LS_KEY = "sim_installer_v1";

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "glitch": "normal",
  "accent": "indigo",
  "commandGrid": true,
  "scanlines": true
}/*EDITMODE-END*/;

const ACCENTS = {
  indigo: { "--accent": "#6d5bf6", "--accent-2": "#3fe0d8", "--accent-warn": "#ff4d6d", "--accent-soft": "rgba(109,91,246,0.14)" },
  cyan:   { "--accent": "#25e6c8", "--accent-2": "#6d5bf6", "--accent-warn": "#ff4d6d", "--accent-soft": "rgba(37,230,200,0.14)" },
  red:    { "--accent": "#ff4d6d", "--accent-2": "#ff9d3f", "--accent-warn": "#ff4d6d", "--accent-soft": "rgba(255,77,109,0.14)" },
};
const GLITCH_AMT = { calm: 0.35, normal: 1, chaos: 1.8 };

function loadState() {
  try { return JSON.parse(localStorage.getItem(LS_KEY)) || {}; }
  catch { return {}; }
}

function App() {
  const saved = useRef(loadState()).current;
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);

  const [tab, setTab]           = useState(saved.tab || "install");
  const [dir, setDir]           = useState(saved.dir || "");
  const [phase, setPhase]       = useState("idle");
  const [installExists, setInstallExists] = useState(null); // null|'ok'|'missing'|'empty'
  const [progress, setProgress] = useState(saved.installed ? 1 : 0);
  const [stage, setStage]       = useState(saved.installed ? "Установлено" : "");
  const [log, setLog]           = useState([]);
  const [java, setJava]         = useState({ phase: "pending", text: "Проверка Java…", sub: "" });
  const [updateCount, setUpdateCount] = useState(0);

  const [ram, setRam]           = useState(saved.ram || 6144);
  const [jvmArgs, setJvmArgs]   = useState(saved.jvmArgs ||
    "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200");
  const [toggles, setToggles]   = useState(saved.toggles ||
    { closeOnPlay: true, autoUpdate: true, preRelease: false });
  const [saveStatus, setSaveStatus] = useState(null);
  const [profilesPath, setProfilesPath] = useState('');

  // persist
  useEffect(() => {
    localStorage.setItem(LS_KEY, JSON.stringify({
      tab, dir, installed: phase === "done", ram, jvmArgs, toggles,
    }));
  }, [tab, dir, phase, ram, jvmArgs, toggles]);

  // java check
  useEffect(() => {
    (async () => {
      try {
        const ok = await invoke('check_java');
        setJava(ok
          ? { phase: "ok",   text: "Java 17+ найдена", sub: "" }
          : { phase: "fail", text: "Java 17+ не найдена", sub: "будет использована встроенная в Minecraft" }
        );
      } catch (e) {
        setJava({ phase: "fail", text: "Ошибка проверки Java", sub: String(e).slice(0, 60) });
      }
    })();
  }, []);

  // default dir + profiles path
  useEffect(() => {
    if (dir) return;
    invoke('get_default_install_dir').then(setDir).catch(() => {});
  }, []);
  useEffect(() => {
    invoke('get_profiles_path').then(setProfilesPath).catch(() => {});
  }, []);

  // проверяем наличие файлов при смене папки (с дебаунсом 600мс)
  useEffect(() => {
    if (!dir) return;
    const t = setTimeout(async () => {
      try {
        const status = await invoke('check_install_exists', { installDir: dir });
        setInstallExists(status);
        if (status === 'ok') {
          setPhase('done');  // сборка найдена — сразу показываем Play/Update
        } else {
          setPhase('idle');  // файлов нет — предлагаем установить
        }
      } catch { setInstallExists(null); }
    }, 600);
    return () => clearTimeout(t);
  }, [dir]);

  // progress events
  useEffect(() => {
    let unsub;
    const STAGE_LABELS = {
      manifest: "Загрузка манифеста",
      java:     "Проверка Java",
      forge:    "Установка Forge",
      mods:     "Загрузка модов",
      configs:  "Загрузка конфигов",
      profile:  "Создание профиля",
      done:     "Готово",
    };
    listenEvent('install_progress', ({ payload }) => {
      if (!payload) return;
      const { stage: s, progress: p, message } = payload;
      // stage — короткий ключ (mods/forge/…), message — детальный текст с именем файла
      if (s && STAGE_LABELS[s]) setStage(STAGE_LABELS[s]);
      else if (s) setStage(s);
      if (p != null) setProgress(p);
      if (message) setLog(prev => [...prev, { ts: nowTs(), type: 'info', text: message }]);
    }).then(fn => { unsub = fn; });
    return () => unsub?.();
  }, []);

  const pushLog = useCallback((type, text) => {
    setLog(prev => [...prev, { ts: nowTs(), type, text }]);
  }, []);

  // browse
  const browse = useCallback(async () => {
    try {
      const picked = await invoke('pick_directory');
      if (picked) setDir(picked);
    } catch (e) {
      pushLog('err', 'Диалог недоступен: ' + e);
    }
  }, [pushLog]);

  // install
  const install = useCallback(async () => {
    if (phase === "installing" || !dir) return;
    setPhase("installing");
    setProgress(0);
    setLog([]);
    setStage("Подготовка…");
    pushLog("sys", "🚀 запуск установки сборки «Симуляция»");
    try {
      await invoke('install', { installDir: dir, ramMb: ram });
      pushLog("ok", "✓ установка завершена · барьер снят");
      setStage("Установлено");
      setPhase("done");
      setProgress(1);
    } catch (e) {
      pushLog("err", "✗ " + e);
      setPhase("idle");
    }
  }, [phase, dir, pushLog]);

  // check updates
  const checkUpdates = useCallback(async () => {
    if (phase === "checking") return;
    setPhase("checking");
    setLog([]);
    pushLog("info", "🔍 проверка обновлений…");
    try {
      const changed = await invoke('check_updates', { installDir: dir });
      if (changed.length === 0) {
        pushLog("ok", "✓ сборка актуальна");
        setUpdateCount(0);
      } else {
        pushLog("info", `найдено изменений: ${changed.length}. Загружаю…`);
        await invoke('apply_updates', { installDir: dir });
        pushLog("ok", "✓ обновление завершено");
        setUpdateCount(0);
      }
    } catch (e) {
      pushLog("err", "✗ " + e);
    }
    setPhase("done");
  }, [phase, dir, pushLog]);

  // play
  const play = useCallback(async () => {
    pushLog("sys", `▶ запуск Minecraft ${BUILD.mc} · ${(ram/1024).toFixed(0)} ГБ RAM`);
    try {
      const msg = await invoke('launch_game', { installDir: dir });
      pushLog("ok", "✓ " + msg);
      pushLog("info", "выбери профиль SimulationModPack в лаунчере");
    } catch (e) {
      pushLog("err", String(e));
    }
  }, [ram, dir, pushLog]);

  // window controls
  const winMinimize = useCallback(async () => {
    try { const w = await tauriWindow(); await w?.minimize(); } catch {}
  }, []);
  const winClose = useCallback(async () => {
    try { const w = await tauriWindow(); await w?.close(); } catch {}
  }, []);

  const toggle = useCallback((id) => {
    setToggles(p => ({ ...p, [id]: !p[id] }));
  }, []);

  // save settings
  const saveSettings = useCallback(async () => {
    setSaveStatus(null);
    try {
      const msg = await invoke('save_settings', { ramMb: ram, extraJvmArgs: jvmArgs });
      setSaveStatus({ ok: true, msg });
    } catch (e) {
      setSaveStatus({ ok: false, msg: String(e) });
    }
    setTimeout(() => setSaveStatus(null), 5000);
  }, [ram, jvmArgs]);

  const rootStyle = { ...ACCENTS[t.accent], "--glitch-amt": GLITCH_AMT[t.glitch] };

  return (
    <div className={"stage" + (t.commandGrid ? "" : " no-grid")}>
      <div
        className="sim-root"
        style={rootStyle}
        data-glitch={t.glitch}
        data-scanlines={t.scanlines ? "on" : "off"}
      >
        <div className="crt" />
        <TitleBar onMinimize={winMinimize} onClose={winClose} />
        <BuildHero build={BUILD} />
        <Tabs active={tab} onChange={setTab} updateCount={updateCount} />

        <div className="content">
          {tab === "install"    && (
            <InstallPanel
              dir={dir} onBrowse={browse} onDirChange={setDir}
              java={java} phase={phase} progress={progress} stage={stage} log={log}
              installExists={installExists}
              onInstall={install} onPlay={play} onCheckUpdates={checkUpdates}
            />
          )}
          {tab === "changelog"  && <ChangelogPanel />}
          {tab === "settings"   && (
            <SettingsPanel
              ram={ram} onRam={setRam}
              jvmArgs={jvmArgs} onJvmArgs={setJvmArgs}
              toggles={toggles} onToggle={toggle}
              onSave={saveSettings} saveStatus={saveStatus}
              profilesPath={profilesPath}
            />
          )}
          {tab === "about"      && <AboutPanel build={BUILD} />}
        </div>

        <StatusBar build={BUILD} phase={phase} />
      </div>

      <TweaksPanel>
        <TweakSection label="Глитч" />
        <TweakRadio label="Интенсивность" value={t.glitch}
          options={["calm", "normal", "chaos"]}
          onChange={(v) => setTweak("glitch", v)} />
        <TweakToggle label="Scanlines (CRT)" value={t.scanlines}
          onChange={(v) => setTweak("scanlines", v)} />
        <TweakToggle label="Сетка командных блоков" value={t.commandGrid}
          onChange={(v) => setTweak("commandGrid", v)} />
        <TweakSection label="Палитра акцента" />
        <TweakRadio label="Цвет" value={t.accent}
          options={["indigo", "cyan", "red"]}
          onChange={(v) => setTweak("accent", v)} />
      </TweaksPanel>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);
