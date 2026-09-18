import { useCallback, useMemo, useRef, useState } from "react";
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { useStore, type SortKey } from "../state/store";
import type { ModEntry, ProfileEntry } from "../ipc/commands";
import { comparePackNames, formatBytes, formatDate } from "../util/format";
import ContextMenu, { type MenuItem } from "../components/ContextMenu";

interface Row {
  key: string;
  entry: ProfileEntry;
  mod: ModEntry | undefined;
  title: string;
  /** Position in the profile's entries array (load order). */
  index: number;
  updated: number;
  hidden: boolean;
  tags: string[];
}

const COLUMNS: { key: SortKey; label: string; className: string }[] = [
  { key: "order", label: "#", className: "w-12 text-right" },
  { key: "title", label: "Mod", className: "" },
  { key: "source", label: "Source", className: "w-24" },
  { key: "type", label: "Type", className: "w-16" },
  { key: "size", label: "Size", className: "w-20 text-right" },
  { key: "updated", label: "Updated", className: "w-28" },
];

export default function ModList() {
  const profile = useStore((s) => s.activeProfile());
  const modsByKey = useStore((s) => s.modsByKey);
  const workshop = useStore((s) => s.workshop);
  const meta = useStore((s) => s.meta);
  const filters = useStore((s) => s.filters);
  const setFilters = useStore((s) => s.setFilters);
  const sort = useStore((s) => s.sort);
  const setSort = useStore((s) => s.setSort);
  const selected = useStore((s) => s.selected);
  const focused = useStore((s) => s.focused);
  const select = useStore((s) => s.select);
  const toggleMods = useStore((s) => s.toggleMods);
  const moveMods = useStore((s) => s.moveMods);
  const sortProfileAlpha = useStore((s) => s.sortProfileAlpha);
  const enabledToTop = useStore((s) => s.enabledToTop);
  const gameRunning = useStore((s) => s.gameRunning);
  const setMeta = useStore((s) => s.setMeta);
  const lastClicked = useRef<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; row: Row } | null>(null);

  const menuItems = (row: Row): MenuItem[] => {
    const keys = selected.includes(row.key) ? selected : [row.key];
    const mm = meta.mods[row.key];
    const n = keys.length > 1 ? ` (${keys.length})` : "";
    return [
      { label: `Enable${n}`, disabled: gameRunning, onClick: () => void toggleMods(keys, true) },
      { label: `Disable${n}`, disabled: gameRunning, onClick: () => void toggleMods(keys, false) },
      { separator: true, label: "" },
      {
        label: "Open in Workshop",
        disabled: !row.mod?.workshopId,
        onClick: () => void openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${row.mod?.workshopId}`),
      },
      { label: "Show file in Explorer", disabled: !row.mod, onClick: () => row.mod && void revealItemInDir(row.mod.path) },
      { separator: true, label: "" },
      { label: mm?.hidden ? "Unhide" : "Hide", onClick: () => void setMeta(row.key, { hidden: !mm?.hidden }) },
      { label: "Copy key", onClick: () => void navigator.clipboard?.writeText(row.key) },
    ];
  };

  const allTags = useMemo(() => {
    const t = new Set<string>();
    for (const m of Object.values(meta.mods)) m.tags.forEach((x) => t.add(x));
    return [...t].sort();
  }, [meta]);

  const rows: Row[] = useMemo(() => {
    const q = filters.search.trim().toLowerCase();
    const out: Row[] = [];
    profile.entries.forEach((entry, index) => {
      const mod = modsByKey[entry.key];
      const mm = meta.mods[entry.key];
      const hidden = !!mm?.hidden;
      if (hidden && !filters.showHidden) return;
      const ws = mod?.workshopId ? workshop[mod.workshopId] : undefined;
      const title = ws?.title || mod?.file.replace(/\.pack$/i, "") || entry.key;
      if (filters.source !== "all" && mod && mod.source !== filters.source) return;
      if (filters.type !== "all" && mod && mod.packType !== filters.type) return;
      if (filters.enabled === "enabled" && !entry.enabled) return;
      if (filters.enabled === "disabled" && entry.enabled) return;
      if (filters.tag && !(mm?.tags ?? []).includes(filters.tag)) return;
      if (q && !title.toLowerCase().includes(q) && !(mod?.file ?? "").toLowerCase().includes(q) && !entry.key.toLowerCase().includes(q)) return;
      out.push({ key: entry.key, entry, mod, title, index, updated: ws?.timeUpdated || mod?.mtime || 0, hidden, tags: mm?.tags ?? [] });
    });
    if (sort.key !== "order") {
      const cmp = (a: Row, b: Row): number => {
        switch (sort.key) {
          case "title":
            return comparePackNames(a.title, b.title);
          case "file":
            return comparePackNames(a.mod?.file ?? a.key, b.mod?.file ?? b.key);
          case "source":
            return (a.mod?.source ?? "").localeCompare(b.mod?.source ?? "");
          case "type":
            return (a.mod?.packType ?? "").localeCompare(b.mod?.packType ?? "");
          case "size":
            return (a.mod?.size ?? 0) - (b.mod?.size ?? 0);
          case "updated":
            return a.updated - b.updated;
          default:
            return 0;
        }
      };
      out.sort((a, b) => cmp(a, b) * sort.dir || a.index - b.index);
    }
    return out;
  }, [profile, modsByKey, workshop, meta, filters, sort]);

  const modRows = rows.filter((r) => !r.mod || r.mod.packType !== "movie");
  const movieRows = rows.filter((r) => r.mod?.packType === "movie");
  const canDrag = sort.key === "order" && !gameRunning;

  const onRowClick = useCallback(
    (row: Row, e: React.MouseEvent) => {
      const visible = rows.map((r) => r.key);
      if (e.shiftKey && lastClicked.current) {
        const a = visible.indexOf(lastClicked.current);
        const b = visible.indexOf(row.key);
        if (a >= 0 && b >= 0) {
          const [lo, hi] = a < b ? [a, b] : [b, a];
          select(visible.slice(lo, hi + 1), row.key);
          return;
        }
      }
      if (e.ctrlKey || e.metaKey) {
        const next = selected.includes(row.key) ? selected.filter((k) => k !== row.key) : [...selected, row.key];
        select(next, row.key);
      } else {
        select([row.key], row.key);
      }
      lastClicked.current = row.key;
    },
    [rows, selected, select],
  );

  const onToggle = useCallback(
    (row: Row, checked: boolean) => {
      const keys = selected.includes(row.key) && selected.length > 1 ? selected : [row.key];
      void toggleMods(keys, checked);
    },
    [selected, toggleMods],
  );

  /** Move the selection one step up/down in the load order (only mod packs move). */
  const nudge = useCallback(
    (dir: -1 | 1) => {
      const entries = profile.entries;
      const keys = selected.filter((k) => modsByKey[k]?.packType !== "movie");
      if (keys.length === 0) return;
      const ks = new Set(keys);
      const idx = entries.map((e, i) => (ks.has(e.key) ? i : -1)).filter((i) => i >= 0);
      const rest = entries.filter((e) => !ks.has(e.key));
      const target = dir < 0 ? Math.min(...idx) - 1 : Math.max(...idx) + 1;
      if (target < 0 || target >= entries.length) return;
      const restIdx = rest.findIndex((e) => e.key === entries[target].key);
      if (restIdx < 0) return;
      void moveMods(keys, dir < 0 ? restIdx : restIdx + 1);
    },
    [profile, selected, modsByKey, moveMods],
  );

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.altKey && e.key === "ArrowUp") {
      e.preventDefault();
      nudge(-1);
    } else if (e.altKey && e.key === "ArrowDown") {
      e.preventDefault();
      nudge(1);
    } else if (e.key === " " && selected.length) {
      e.preventDefault();
      const first = profile.entries.find((x) => x.key === selected[0]);
      void toggleMods(selected, !(first?.enabled ?? false));
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      select(rows.map((r) => r.key), focused);
    }
  };

  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));
  const onDragEnd = (ev: DragEndEvent) => {
    const { active, over } = ev;
    if (!over || active.id === over.id) return;
    const activeKey = String(active.id);
    const overKey = String(over.id);
    const keys = selected.includes(activeKey) ? selected.filter((k) => modsByKey[k]?.packType !== "movie") : [activeKey];
    const ks = new Set(keys);
    if (ks.has(overKey)) return;
    const entries = profile.entries;
    const from = entries.findIndex((e) => e.key === activeKey);
    const to = entries.findIndex((e) => e.key === overKey);
    const rest = entries.filter((e) => !ks.has(e.key));
    const restIdx = rest.findIndex((e) => e.key === overKey);
    if (restIdx < 0) return;
    void moveMods(keys, from < to ? restIdx + 1 : restIdx);
  };

  const enabledCount = profile.entries.filter((e) => e.enabled).length;

  return (
    <section className="flex-1 min-w-0 flex flex-col" onKeyDown={onKeyDown} tabIndex={0}>
      <div className="flex items-center gap-2 px-2 py-1.5 border-b border-edge text-[12px] overflow-x-auto">
        <input
          value={filters.search}
          onChange={(e) => setFilters({ search: e.target.value })}
          placeholder="Search title / file"
          className="w-44 shrink-0"
        />
        <select value={filters.source} onChange={(e) => setFilters({ source: e.target.value as typeof filters.source })}>
          <option value="all">All sources</option>
          <option value="workshop">Workshop</option>
          <option value="data">data/</option>
        </select>
        <select value={filters.type} onChange={(e) => setFilters({ type: e.target.value as typeof filters.type })}>
          <option value="all">Mods + movies</option>
          <option value="mod">Mod packs</option>
          <option value="movie">Movie packs</option>
        </select>
        <select value={filters.enabled} onChange={(e) => setFilters({ enabled: e.target.value as typeof filters.enabled })}>
          <option value="all">Enabled + disabled</option>
          <option value="enabled">Enabled</option>
          <option value="disabled">Disabled</option>
        </select>
        {allTags.length > 0 && (
          <select value={filters.tag ?? ""} onChange={(e) => setFilters({ tag: e.target.value || null })}>
            <option value="">Any tag</option>
            {allTags.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        )}
        <label className="flex items-center gap-1 cursor-pointer whitespace-nowrap">
          <input type="checkbox" checked={filters.showHidden} onChange={(e) => setFilters({ showHidden: e.target.checked })} />
          hidden
        </label>
        <div className="flex-1" />
        <span className="text-textMuted whitespace-nowrap" title={`${rows.length} shown of ${profile.entries.length}`}>
          {enabledCount} enabled / {profile.entries.length}
        </span>
        <button
          className="btn"
          disabled={gameRunning}
          title="Reorder the whole profile alphabetically by file name (the CA launcher's default order)"
          onClick={() => confirm("Sort the entire load order alphabetically by pack file name?") && void sortProfileAlpha()}
        >
          Sort A→Z
        </button>
        <button className="btn" disabled={gameRunning} title="Move every enabled mod above the disabled ones" onClick={() => void enabledToTop()}>
          Enabled to top
        </button>
      </div>

      <div className="flex-1 overflow-auto">
        <table className="w-full border-collapse text-[12px]">
          <thead className="sticky top-0 bg-panelHeader z-10">
            <tr className="text-left text-textMuted">
              <th className="w-8 px-2 py-1 font-normal" />
              {COLUMNS.map((c) => (
                <th key={c.key} className={`px-2 py-1 font-normal ${c.className}`}>
                  <button className="hover:text-text" onClick={() => setSort(c.key)}>
                    {c.label}
                    {sort.key === c.key ? (sort.dir === 1 ? " ▲" : " ▼") : ""}
                  </button>
                </th>
              ))}
            </tr>
          </thead>
          <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
            <SortableContext items={modRows.map((r) => r.key)} strategy={verticalListSortingStrategy}>
              <tbody>
                {modRows.map((row) => (
                  <ModRow
                    key={row.key}
                    row={row}
                    selected={selected.includes(row.key)}
                    focused={focused === row.key}
                    canDrag={canDrag}
                    disabled={gameRunning}
                    onClick={onRowClick}
                    onToggle={onToggle}
                    onContextMenu={(r, e) => {
                      if (!selected.includes(r.key)) select([r.key], r.key);
                      setMenu({ x: e.clientX, y: e.clientY, row: r });
                    }}
                  />
                ))}
              </tbody>
            </SortableContext>
          </DndContext>
          {movieRows.length > 0 && (
            <tbody>
              <tr>
                <td colSpan={COLUMNS.length + 1} className="px-2 pt-3 pb-1 text-[11px] text-warn border-t border-edge">
                  Movie packs — always load after every mod pack; order between them is fixed by the engine.
                </td>
              </tr>
              {movieRows.map((row) => (
                <ModRow
                  key={row.key}
                  row={row}
                  selected={selected.includes(row.key)}
                  focused={focused === row.key}
                  canDrag={false}
                  disabled={gameRunning}
                  onClick={onRowClick}
                  onToggle={onToggle}
                  onContextMenu={(r, e) => {
                    if (!selected.includes(r.key)) select([r.key], r.key);
                    setMenu({ x: e.clientX, y: e.clientY, row: r });
                  }}
                />
              ))}
            </tbody>
          )}
        </table>
        {rows.length === 0 && <div className="p-6 text-center text-textMuted text-[12px]">No packs match the current filters.</div>}
      </div>
      {menu && <ContextMenu x={menu.x} y={menu.y} items={menuItems(menu.row)} onClose={() => setMenu(null)} />}
    </section>
  );
}

function ModRow({
  row,
  selected,
  focused,
  canDrag,
  disabled,
  onClick,
  onToggle,
  onContextMenu,
}: {
  row: Row;
  selected: boolean;
  focused: boolean;
  canDrag: boolean;
  disabled: boolean;
  onClick: (row: Row, e: React.MouseEvent) => void;
  onToggle: (row: Row, checked: boolean) => void;
  onContextMenu: (row: Row, e: React.MouseEvent) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: row.key, disabled: !canDrag });
  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : undefined,
  };
  const missing = !row.mod;
  const isMovie = row.mod?.packType === "movie";
  return (
    <tr
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...(canDrag ? listeners : {})}
      onClick={(e) => onClick(row, e)}
      onContextMenu={(e) => {
        e.preventDefault();
        onContextMenu(row, e);
      }}
      className={`border-t border-edge/40 ${selected ? "bg-selected" : "hover:bg-hover"} ${focused ? "outline outline-1 outline-accent/60 -outline-offset-1" : ""} ${
        canDrag ? "cursor-grab" : ""
      } ${row.entry.enabled ? "" : "text-textMuted"}`}
    >
      <td className="px-2 py-0.5" onClick={(e) => e.stopPropagation()}>
        <input
          type="checkbox"
          checked={row.entry.enabled}
          disabled={disabled || missing}
          onChange={(e) => onToggle(row, e.target.checked)}
          onPointerDown={(e) => e.stopPropagation()}
        />
      </td>
      <td className="px-2 py-0.5 text-right text-textMuted tabular-nums">{isMovie ? "" : row.index + 1}</td>
      <td className="px-2 py-0.5">
        <div className="flex items-center gap-1.5 min-w-0">
          <span className={`truncate ${missing ? "line-through" : ""}`} title={row.mod?.file ?? row.key}>
            {row.title}
          </span>
          {missing && <span className="badge border-danger text-danger">missing</span>}
          {row.hidden && <span className="badge border-edge text-textMuted">hidden</span>}
          {row.tags.map((t) => (
            <span key={t} className="badge border-accent/50 text-accent">
              {t}
            </span>
          ))}
        </div>
        {row.mod && row.title !== row.mod.file.replace(/\.pack$/i, "") && (
          <div className="text-[10px] text-textMuted truncate">{row.mod.file}</div>
        )}
      </td>
      <td className="px-2 py-0.5">
        {row.mod && (
          <span className={`badge ${row.mod.source === "workshop" ? "border-accent/50 text-accent" : "border-edge text-textMuted"}`}>
            {row.mod.source === "workshop" ? "Workshop" : "data/"}
          </span>
        )}
      </td>
      <td className="px-2 py-0.5">
        {row.mod && (
          <span className={`badge ${isMovie ? "border-warn text-warn" : "border-edge text-textMuted"}`}>{isMovie ? "MOVIE" : "mod"}</span>
        )}
      </td>
      <td className="px-2 py-0.5 text-right tabular-nums text-textMuted">{row.mod ? formatBytes(row.mod.size) : ""}</td>
      <td className="px-2 py-0.5 text-textMuted">{formatDate(row.updated)}</td>
    </tr>
  );
}
