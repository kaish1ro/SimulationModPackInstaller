/* global React */
const { useState, useEffect, useRef } = React;

/* ============================================================
   ICONS (tiny inline)
   ============================================================ */
function Icon({ d, size = 14, stroke = 1.6 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none"
         stroke="currentColor" strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round">
      {d}
    </svg>
  );
}
const Icons = {
  min: <line x1="5" y1="12" x2="19" y2="12" />,
  max: <rect x="5" y="5" width="14" height="14" rx="1" />,
  close: <g><line x1="6" y1="6" x2="18" y2="18" /><line x1="18" y1="6" x2="6" y2="18" /></g>,
  folder: <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />,
  download: <g><path d="M12 3v12" /><path d="M7 11l5 5 5-5" /><path d="M5 21h14" /></g>,
  play: <path d="M7 4l13 8-13 8z" />,
  refresh: <g><path d="M21 12a9 9 0 1 1-3-6.7" /><path d="M21 4v5h-5" /></g>,
  check: <path d="M5 12l4 4 10-10" />,
  install: <path d="M3 7v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V7" />,
  list: <g><line x1="8" y1="7" x2="20" y2="7" /><line x1="8" y1="12" x2="20" y2="12" /><line x1="8" y1="17" x2="20" y2="17" /><circle cx="4" cy="7" r="1" /><circle cx="4" cy="12" r="1" /><circle cx="4" cy="17" r="1" /></g>,
  sliders: <g><line x1="4" y1="8" x2="20" y2="8" /><line x1="4" y1="16" x2="20" y2="16" /><circle cx="9" cy="8" r="2.4" /><circle cx="15" cy="16" r="2.4" /></g>,
  info: <g><circle cx="12" cy="12" r="9" /><line x1="12" y1="11" x2="12" y2="16" /><circle cx="12" cy="8" r="0.6" /></g>,
};

/* ============================================================
   TITLE BAR
   ============================================================ */
function TitleBar({ onMinimize, onClose }) {
  return (
    <div className="titlebar" data-tauri-drag-region>
      <span className="tb-dot" />
      <span className="tb-name"><b>SIMULATION</b>_installer.exe</span>
      <span className="tb-spacer" />
      <div className="tb-watch">
        <span className="tb-rec" />SIMULATION v0.1.0
      </div>
      <span className="tb-spacer" />
      <div className="tb-ctrls">
        <button className="tb-btn" title="Свернуть" onClick={onMinimize}><Icon d={Icons.min} size={11} /></button>
        <button className="tb-btn close" title="Закрыть" onClick={onClose}><Icon d={Icons.close} size={11} /></button>
      </div>
    </div>
  );
}

/* ============================================================
   HERO
   ============================================================ */
function BuildHero({ build }) {
  return (
    <div className="hero">
      <div className="hero-art" style={{ perspective: 300 }}>
        <div className="ha-grid" />
        <div className="ha-core" style={{ filter: 'drop-shadow(0 0 14px rgba(109,91,246,0.9))' }}>
          <CommandBlock size={56} />
        </div>
        <div className="ha-label">repeating // command_block</div>
      </div>

      <div className="hero-main">
        <span className="eyebrow">Modpack · {build.mc} · {build.loader}</span>
        <img className="sim-logo" src="logo.png" alt={build.title} />
        <div className="hero-meta">
          <span className="chip"><span className="k">моды</span> {build.mods}</span>
          <span className="chip"><span className="k">размер</span> {build.size}</span>
          <span className="chip accent">{build.channel}</span>
        </div>
      </div>

      <div className="hero-ver">
        <div className="vlabel">версия сборки</div>
        <div className="vnum">v{build.version}</div>
      </div>
    </div>
  );
}

/* ============================================================
   TABS
   ============================================================ */
function Tabs({ active, onChange, updateCount }) {
  const tabs = [
    { id: "install", label: "Установка", icon: Icons.install },
    { id: "changelog", label: "Что нового", icon: Icons.list, badge: updateCount },
    { id: "settings", label: "Настройки", icon: Icons.sliders },
    { id: "about", label: "О сборке", icon: Icons.info },
  ];
  return (
    <div className="tabstrip">
      {tabs.map((t) => (
        <button key={t.id} className="tab" data-active={active === t.id}
                onClick={() => onChange(t.id)}>
          <Icon d={t.icon} size={13} />
          {t.label}
          {t.badge ? <span className="badge">{t.badge}</span> : null}
        </button>
      ))}
    </div>
  );
}

/* ============================================================
   JAVA STATUS LINE
   ============================================================ */
function JavaStatus({ state }) {
  // state: { phase: 'pending'|'ok'|'fail', text, sub }
  return (
    <div className={"statusline " + state.phase}>
      <span className="dot" />
      <span>{state.text}</span>
      {state.sub ? <span className="sub">{state.sub}</span> : null}
    </div>
  );
}

/* ============================================================
   LOG
   ============================================================ */
function LogView({ lines, busy }) {
  const ref = useRef(null);
  useEffect(() => {
    if (ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [lines.length]);
  return (
    <div className="log" ref={ref}>
      {lines.map((l, i) => (
        <div className={"ln " + (l.type || "")} key={i}>
          <span className="ts">{l.ts}</span>
          <span className={l.type}>{l.text}</span>
        </div>
      ))}
      {busy ? <div className="ln"><span className="ts">{nowTs()}</span><span className="sys cursor" /></div> : null}
    </div>
  );
}

function nowTs() {
  const d = new Date();
  const p = (n) => String(n).padStart(2, "0");
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

/* ============================================================
   STATUS BAR
   ============================================================ */
function StatusBar({ build, phase }) {
  const msg = {
    idle: "ожидание команды",
    checking: "проверка обновлений…",
    installing: "запись данных симуляции…",
    done: "установка завершена",
  }[phase] || "ожидание";
  return (
    <div className="statusbar">
      <span className="sb-watch"><span className="eye" />наблюдение продолжается</span>
      <span className="sb-spacer" />
      <span>{msg}</span>
      <span>·</span>
      <span>build {build.version}</span>
      <span>·</span>
      <span>{build.loader} {build.mc}</span>
    </div>
  );
}

Object.assign(window, {
  Icon, Icons, TitleBar, BuildHero, Tabs, JavaStatus, LogView, StatusBar, nowTs,
});
