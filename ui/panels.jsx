/* global React, Icon, Icons, JavaStatus, LogView */
const { useState: useStateP } = React;

/* ============================================================
   INSTALL PANEL
   ============================================================ */
function InstallPanel({
  dir, onBrowse, onDirChange,
  java, phase, progress, stage, log,
  installExists,
  onInstall, onPlay, onCheckUpdates,
}) {
  const installing = phase === "installing";
  const checking = phase === "checking";
  const done = phase === "done";
  const busy = installing || checking;
  const needsInstall = installExists === 'missing';

  return (
    <div className="panel" style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>

      {/* ── done: кнопки сверху ── */}
      {done && !installing && (
        <div className="actions" style={{ marginBottom: 16 }}>
          <button className="btn btn-play btn-lg" onClick={onPlay}>
            <Icon d={Icons.play} size={15} stroke={2} /> Играть
          </button>
          <button className="btn btn-ghost" onClick={onCheckUpdates} disabled={checking}>
            <Icon d={Icons.refresh} size={14} />
            {checking ? "Проверка…" : "Обновления"}
          </button>
        </div>
      )}

      {/* ── папка установки ── */}
      <label className="field-label">Папка установки</label>
      <div className="dir-row">
        <input
          className="input"
          value={dir}
          placeholder="C:\\Users\\...\\.simulation"
          onChange={(e) => onDirChange(e.target.value)}
          spellCheck={false}
        />
        <button className="btn btn-ghost" onClick={(e) => { e.stopPropagation(); onBrowse(); }}
                style={{ pointerEvents: 'auto', zIndex: 10 }}>
          <Icon d={Icons.folder} size={14} /> Обзор
        </button>
      </div>

      {/* ── java / статус ── */}
      {!installing && <JavaStatus state={java} />}

      {needsInstall && !installing && (
        <div className="statusline fail" style={{ marginTop: 10 }}>
          <span className="dot" />
          <span>Файлы сборки не найдены — требуется установка</span>
        </div>
      )}

      {/* ── done banner ── */}
      {done && !installing && (
        <div className="done-banner" style={{ marginTop: 14 }}>
          <div className="di"><Icon d={Icons.check} size={22} stroke={2.4} /></div>
          <div>
            <div className="dt">Сборка установлена</div>
            <div className="ds">барьер снят · сборка готова к запуску</div>
          </div>
        </div>
      )}

      {/* ── кнопка установки ── */}
      {!done && !busy && (
        <div className="actions">
          <button className="btn btn-primary btn-lg" onClick={onInstall} disabled={busy || !dir}>
            <Icon d={Icons.download} size={15} />
            {needsInstall ? "Установить (требуется)" : "Установить сборку"}
          </button>
        </div>
      )}

      {/* ── прогресс-бар: только во время установки ── */}
      {(installing || checking) && (
        <div className="progress-wrap">
          <div className="progress-head">
            <span className="stage">{stage || "Подготовка…"}</span>
            <span className="pct">{Math.round(progress * 100)}%</span>
          </div>
          <div className="progress-track" style={{ marginBottom: 12 }}>
            <div className="progress-fill" style={{ width: progress * 100 + "%" }} />
          </div>
        </div>
      )}

      {/* ── лог: показываем пока есть записи (включая после ошибки) ── */}
      {log.length > 0 && (
        <div style={{ marginTop: installing ? 0 : 12 }}>
          <LogView lines={log} busy={busy} />
        </div>
      )}
    </div>
  );
}

/* ============================================================
   CHANGELOG PANEL
   ============================================================ */
const CHANGELOG = [
  {
    ver: "0.1.0", date: "4 июня 2026", isNew: true,
    items: [
      { t: "Pre-Alpha: первый закрытый тест с друзьями", k: "new", meta: "pre-alpha" },
      { t: "Система разломов — красные, синие, фиолетовые, элитные волны", k: "new" },
      { t: "Странник (NPC на Ollama) появляется после первого выхода из Ада", k: "lore" },
      { t: "Питер — житель деревни с ИИ-диалогом, прогрев модели при приближении", k: "lore" },
      { t: "Прогрессия порталов: Энд требует Нижний мир, модовые — убийства дракона", k: "new" },
      { t: "Cataclysm: урон оружия ограничен по стейджам (8→9 после дракона)", k: "new" },
      { t: "Броня Cataclysm/Aether/Blue Skies/Undergarden — кастомные статы через AttributeModifier", k: "new" },
      { t: "Apotheosis: отключены боссы, гемы, спавнеры; ограничены чары", k: "fix" },
      { t: "Убраны структуры Born in Chaos, Threateningly Mobs, BOMD, Apotheosis", k: "fix" },
      { t: "Все таблицы лута деревень / данжей очищены от готового снаряжения", k: "fix" },
      { t: "KubeJS: заблокированы рецепты Cyclic (жезлы, яблоки, чармы, наковальни)", k: "fix" },
      { t: "FTB Квесты: 5 глав прогрессии + доска объявлений + вкладка Cataclysm", k: "new" },
      { t: "Patchouli: Записки Странника, Каталог Артефактов, Атлас Катастроф", k: "lore" },
      { t: "Spice of Life: мягкий дикей до 60%, история 10 блюд", k: "fix" },
    ],
  },
];

function ChangelogPanel() {
  return (
    <div className="panel">
      <h2 className="section-h">Журнал изменений</h2>
      <p className="section-sub">// записи системы · от свежих к старым</p>
      <div className="changelog">
        {CHANGELOG.map((e) => (
          <div className="cl-entry" key={e.ver}>
            <div>
              <div className="cl-ver">v{e.ver}</div>
              <div className="cl-date">{e.date}</div>
              {e.isNew && <span className="cl-tag new">текущая</span>}
            </div>
            <ul className="cl-list">
              {e.items.map((it, i) => (
                <li className={it.k} key={i}>
                  {it.t}
                  {it.meta ? <span className="meta">&nbsp;· {it.meta}</span> : null}
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}

/* ============================================================
   SETTINGS PANEL
   ============================================================ */
function SettingsPanel({ ram, onRam, jvmArgs, onJvmArgs, toggles, onToggle, onSave, saveStatus, profilesPath }) {
  const ramGb = (ram / 1024).toFixed(ram % 1024 === 0 ? 0 : 1);
  return (
    <div className="panel">
      <h2 className="section-h">Параметры запуска</h2>
      <p className="section-sub">// конфигурация JVM · применяется при следующем запуске</p>

      <div className="settings-grid">
        <div className="set-row">
          <label className="field-label">
            Выделенная память (RAM)
            <span className="val">{ramGb} ГБ</span>
          </label>
          <input
            className="slider" type="range"
            min={2048} max={16384} step={512}
            value={ram}
            onChange={(e) => onRam(Number(e.target.value))}
          />
          <div className="ticks">
            <span>2 ГБ</span><span>6 ГБ</span><span>10 ГБ</span><span>16 ГБ</span>
          </div>
        </div>

        <div className="set-row">
          <label className="field-label">Аргументы JVM</label>
          <textarea
            className="textarea"
            value={jvmArgs}
            spellCheck={false}
            onChange={(e) => onJvmArgs(e.target.value)}
          />
        </div>

        <div>
          {TOGGLE_DEFS.map((d) => (
            <div className="toggle-row" key={d.id}>
              <div>
                <div className="tl">{d.label}</div>
                <div className="ts">{d.sub}</div>
              </div>
              <div className="switch" data-on={!!toggles[d.id]} onClick={() => onToggle(d.id)}>
                <div className="knob" />
              </div>
            </div>
          ))}
        </div>

        {/* Сохранить + путь к профилю */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          <button className="btn btn-primary" onClick={onSave}
                  style={{ alignSelf: 'flex-start', padding: '10px 24px' }}>
            <Icon d={Icons.check} size={14} /> Сохранить настройки
          </button>
          {saveStatus && (
            <div style={{
              fontFamily: 'var(--mono)', fontSize: 11,
              color: saveStatus.ok ? 'var(--ok)' : 'var(--warn)',
              padding: '6px 0'
            }}>
              {saveStatus.ok ? '✓' : '✗'} {saveStatus.msg}
            </div>
          )}
          {profilesPath && (
            <div style={{ fontFamily: 'var(--mono)', fontSize: 10, color: 'var(--muted-2)' }}>
              launcher_profiles.json: {profilesPath}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

const TOGGLE_DEFS = [
  { id: "closeOnPlay", label: "Закрывать лаунчер при запуске", sub: "освобождает память во время игры" },
  { id: "autoUpdate", label: "Проверять обновления автоматически", sub: "при каждом запуске установщика" },
  { id: "preRelease", label: "Канал pre-release", sub: "нестабильные сборки с новым лором" },
];

/* ============================================================
   ABOUT PANEL
   ============================================================ */
const DIMENSIONS = [
  { ix: "00", nm: "Ванилла → Эндер Дракон", df: "vanilla", st: 1 },
  { ix: "01", nm: "Twilight Forest", df: "tf", st: 2 },
  { ix: "02", nm: "Undergarden / Iceika", df: "explore", st: 2 },
  { ix: "03", nm: "Aether / Blue Skies / Voidscape", df: "progression", st: 3 },
  { ix: "04", nm: "Deeper & Darker", df: "resource", st: 4 },
  { ix: "4.5", nm: "Шахты", df: "custom", st: 4 },
  { ix: "05", nm: "Divine RPG", df: "endgame", st: 5 },
  { ix: "06", nm: "The Midnight", df: "horror", st: 5 },
  { ix: "07", nm: "Измерение Симуляции", df: "finale", st: 5 },
];

function AboutPanel({ build }) {
  return (
    <div className="panel">
      <h2 className="section-h">{build.title}</h2>
      <p className="section-sub">// {build.mc} · {build.loader} · «они находятся в симуляции»</p>

      <div className="about-grid">
        <div className="stat-card">
          <div className="sv">{build.mods}</div>
          <div className="sk">установленных модов</div>
        </div>
        <div className="stat-card">
          <div className="sv">{build.size}</div>
          <div className="sk">размер сборки</div>
        </div>
        <div className="stat-card">
          <div className="sv">{build.dims}</div>
          <div className="sk">измерений в прогрессии</div>
        </div>
        <div className="stat-card">
          <div className="sv">{build.bosses}</div>
          <div className="sk">боссов до финала</div>
        </div>

        <div className="about-full">
          <label className="field-label" style={{ marginTop: 6 }}>Схема прогрессии</label>
          <div className="dim-list">
            {DIMENSIONS.map((d) => (
              <div className="dim" key={d.ix}>
                <span className="ix">{d.ix}</span>
                <span className="nm">{d.nm}</span>
                <span className="stars">{"◆".repeat(d.st)}<span style={{ color: "var(--muted-2)" }}>{"◇".repeat(5 - d.st)}</span></span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, {
  InstallPanel, ChangelogPanel, SettingsPanel, AboutPanel,
  CHANGELOG, TOGGLE_DEFS, DIMENSIONS,
});
