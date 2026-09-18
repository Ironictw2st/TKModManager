import { useCallback, useMemo, useRef, useState } from "react";
import { askConfirm, askText } from "../components/Dialogs";
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { updatedSince, useStore, type SortKey } from "../state/store";
import type { ModEntry, ProfileEntry } from "../ipc/commands";
import { comparePackNames, formatBytes, formatDate } from "../util/format";
import ContextMenu, { type MenuItem } from "../components/ContextMenu";
import { collapsedKeys, groupMembers, groupState, isSeparator } from "../state/separators";
import { modStatus, seRequirement, seSourceText, STATUS_LABEL, STATUS_RANK, versionLess, type ModStatus, type SeRequirement } from "../util/status";

interface ModRowData {
  kind: "mod";
  key: string;
  entry: ProfileEntry;
  mod: ModEntry | undefined;
  title: string;
  /** Position in the profile's entries array (load order). */
  index: number;
  /** 1-based position among pack entries (separators not counted). */
  order: number;
  updated: number;
  isUpdated: boolean;
  hidden: boolean;
  tags: string[];
  status: ModStatus;
  se: SeRequirement;
  /** Enabled, needs the extender, and this launch would not provide it. */
  seUnmet: boolean;
}

interface SepRowData {
  kind: "sep";
  key: string;
  entry: ProfileEntry;
  index: number;
  members: number;
  enabledMembers: number;
  state: "all" | "none" | "some" | "empty";
}

type Row = ModRowData | SepRowData;

const COLUMNS: { key: SortKey; label: string; className: string }[] = [
  { key: "order", label: "#", className: "w-12 text-right" },
  { key: "title", label: "Mod", className: "" },
  { key: "source", label: "Source", className: "w-24" },
  { key: "type", label: "Type", className: "w-16" },
  { key: "size", label: "Size", className: "w-20 text-right" },
  { key: "updated", label: "Updated", className: "w-28" },
];

const SOURCE_LABEL: Record<ModEntry["source"], string> = { workshop: "Workshop", data: "data/", folder: "Folder" };

const DOT_STYLE: Record<ModStatus["kind"], string> = {
  pending: "bg-danger",
  old: "bg-warn",
  ok: "bg-ok",
  unknown: "border border-textMuted",
  local: "border border-textMuted",
};

const LEGEND = [
  "Status",
  "● green: up to date",
  "● amber: last updated before the current game build",
  "● red: update pending (Steam has a newer version)",
  "○ grey: local file or no update data",
  "SE: needs the script extender (red when this launch would not provide it)",
].join("\n");

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
  const moveBlock = useStore((s) => s.moveBlock);
  const sortProfileAlpha = useStore((s) => s.sortProfileAlpha);
  const enabledToTop = useStore((s) => s.enabledToTop);
  const gameRunning = useStore((s) => s.gameRunning);
  const setMeta = useStore((s) => s.setMeta);
  const addSeparator = useStore((s) => s.addSeparator);
  const renameSeparator = useStore((s) => s.renameSeparator);
  const removeSeparator = useStore((s) => s.removeSeparator);
  const toggleGroup = useStore((s) => s.toggleGroup);
  const setCollapsed = useStore((s) => s.setCollapsed);
  const seScan = useStore((s) => s.seScan);
  const dll = useStore((s) => s.dll);
  const cutoff = useStore((s) => s.outdatedCutoff());
  const lastClicked = useRef<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; row: Row } | null>(null);

  const allTags = useMemo(() => {
    const t = new Set<string>();
    for (const m of Object.values(meta.mods)) m.tags.forEach((x) => t.add(x));
    return [...t].sort();
  }, [meta]);

  // Separators only make sense in load order with no search narrowing the list.
  const groupsVisible = sort.key === "order" && !filters.search.trim();

  const rows: Row[] = useMemo(() => {
    const q = filters.search.trim().toLowerCase();
    const folded = groupsVisible ? collapsedKeys(profile.entries) : new Set<string>();
    const out: Row[] = [];
    let order = 0;
    profile.entries.forEach((entry, index) => {
      if (isSeparator(entry)) {
        if (!groupsVisible) return;
        const members = groupMembers(profile.entries, entry.key).filter((k) => modsByKey[k]);
        const on = members.filter((k) => profile.entries.find((e) => e.key === k)?.enabled).length;
        out.push({
          kind: "sep",
          key: entry.key,
          entry,
          index,
          members: members.length,
          enabledMembers: on,
          state: groupState(profile.entries, entry.key, (k) => !!modsByKey[k]),
        });
        return;
      }
      const mod = modsByKey[entry.key];
      if (mod?.packType !== "movie") order += 1;
      if (folded.has(entry.key) && mod?.packType !== "movie") return;
      const mm = meta.mods[entry.key];
      const hidden = !!mm?.hidden;
      if (hidden && !filters.showHidden) return;
      const ws = mod?.workshopId ? workshop[mod.workshopId] : undefined;
      const title = ws?.title || mod?.file.replace(/\.pack$/i, "") || entry.key;
      const isUpdated = entry.enabled && updatedSince(profile.lastPlayed, mod, ws);
      const status = modStatus(mod, ws, cutoff);
      const se = seRequirement(seScan[entry.key], mm);
      const have = dll?.selected?.version;
      const seUnmet = se.required && entry.enabled && (!profile.dll || !have || (!!se.minVersion && versionLess(have, se.minVersion)));
      if (filters.status === "pending" && status.kind !== "pending") return;
      if (filters.status === "old" && status.kind !== "old") return;
      if (filters.status === "se" && !se.required) return;
      if (filters.source !== "all" && mod && mod.source !== filters.source) return;
      if (filters.type !== "all" && mod && mod.packType !== filters.type) return;
      if (filters.enabled === "enabled" && !entry.enabled) return;
      if (filters.enabled === "disabled" && entry.enabled) return;
      if (filters.enabled === "updated" && !isUpdated) return;
      if (filters.tag && !(mm?.tags ?? []).includes(filters.tag)) return;
      if (q && !title.toLowerCase().includes(q) && !(mod?.file ?? "").toLowerCase().includes(q) && !entry.key.toLowerCase().includes(q)) return;
      out.push({
        kind: "mod",
        key: entry.key,
        entry,
        mod,
        title,
        index,
        order,
        updated: ws?.timeUpdated || mod?.mtime || 0,
        isUpdated,
        hidden,
        tags: mm?.tags ?? [],
        status,
        se,
        seUnmet,
      });
    });
    if (sort.key !== "order") {
      const mods = out.filter((r): r is ModRowData => r.kind === "mod");
      const cmp = (a: ModRowData, b: ModRowData): number => {
        switch (sort.key) {
          case "status":
            return STATUS_RANK[a.status.kind] - STATUS_RANK[b.status.kind] || Number(b.se.required) - Number(a.se.required);
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
      return mods.sort((a, b) => cmp(a, b) * sort.dir || a.index - b.index);
    }
    return out;
  }, [profile, modsByKey, workshop, meta, filters, sort, groupsVisible, seScan, dll, cutoff]);

  const mainRows = rows.filter((r) => r.kind === "sep" || r.mod?.packType !== "movie");
  const movieRows = rows.filter((r): r is ModRowData => r.kind === "mod" && r.mod?.packType === "movie");
  const canDrag = sort.key === "order" && !gameRunning;
  const updatedCount = rows.filter((r) => r.kind === "mod" && r.isUpdated).length;

  const onRowClick = useCallback(
    (row: Row, e: React.MouseEvent) => {
      const visible = rows.map((r) => r.key);
      if (e.shiftKey && lastClicked.current) {
        const a = visible.indexOf(lastClicked.current);
        const b = visible.indexOf(row.key);
        if (a >= 0 && b >= 0) {
          const [lo, hi] = a < b ? [a, b] : [b, a];
          select(visible.slice(lo, hi + 1).filter((k) => !k.startsWith("sep:")), row.key);
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
    (row: ModRowData, checked: boolean) => {
      const keys = selected.includes(row.key) && selected.length > 1 ? selected : [row.key];
      void toggleMods(keys, checked);
    },
    [selected, toggleMods],
  );

  const movable = useCallback(
    (keys: string[]) => keys.filter((k) => k.startsWith("sep:") || modsByKey[k]?.packType !== "movie"),
    [modsByKey],
  );

  /** Move the selection one step up/down in the load order. */
  const nudge = useCallback(
    (dir: -1 | 1) => {
      const entries = profile.entries;
      const keys = movable(selected).filter((k) => !k.startsWith("sep:"));
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
    [profile, selected, movable, moveMods],
  );

  const onKeyDown = (e: React.KeyboardEvent) => {
    if ((e.target as HTMLElement).tagName === "INPUT") return;
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
      select(rows.filter((r) => r.kind === "mod").map((r) => r.key), focused);
    }
  };

  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));
  const onDragEnd = (ev: DragEndEvent) => {
    const { active, over } = ev;
    if (!over || active.id === over.id) return;
    const activeKey = String(active.id);
    const keys = activeKey.startsWith("sep:") ? [activeKey] : selected.includes(activeKey) ? movable(selected) : [activeKey];
    void moveBlock(keys, String(over.id));
  };

  const newSeparator = async (beforeKey: string | null) => {
    const label = await askText("Separator / group name:", "New group");
    if (label?.trim()) void addSeparator(label.trim(), beforeKey);
  };

  const menuItems = (row: Row): MenuItem[] => {
    if (row.kind === "sep") {
      return [
        { label: "Enable group", disabled: gameRunning, onClick: () => void toggleGroup(row.key, true) },
        { label: "Disable group", disabled: gameRunning, onClick: () => void toggleGroup(row.key, false) },
        { separator: true, label: "" },
        {
          label: "Rename…",
          onClick: async () => {
            const label = await askText("Group name:", row.entry.label ?? "");
            if (label?.trim()) void renameSeparator(row.key, label.trim());
          },
        },
        { label: "Add separator above…", onClick: () => void newSeparator(row.key) },
        { label: "Remove separator (keeps its mods)", onClick: () => void removeSeparator(row.key) },
      ];
    }
    const keys = selected.includes(row.key) ? selected : [row.key];
    const mm = meta.mods[row.key];
    const n = keys.length > 1 ? ` (${keys.length})` : "";
    return [
      { label: `Enable${n}`, disabled: gameRunning, onClick: () => void toggleMods(keys, true) },
      { label: `Disable${n}`, disabled: gameRunning, onClick: () => void toggleMods(keys, false) },
      { separator: true, label: "" },
      { label: "Add separator above…", disabled: !groupsVisible, onClick: () => void newSeparator(row.key) },
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

  const openMenu = (r: Row, e: React.MouseEvent) => {
    if (r.kind === "mod" && !selected.includes(r.key)) select([r.key], r.key);
    setMenu({ x: e.clientX, y: e.clientY, row: r });
  };

  const enabledCount = profile.entries.filter((e) => e.enabled && !isSeparator(e)).length;
  const packCount = profile.entries.filter((e) => !isSeparator(e)).length;

  return (
    <section className="flex-1 min-w-0 flex flex-col outline-none" onKeyDown={onKeyDown} tabIndex={0}>
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1 px-2 py-1.5 border-b border-edge text-[12px]">
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
          <option value="folder">Folders</option>
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
          <option value="updated">Updated since last played</option>
        </select>
        <select value={filters.status} onChange={(e) => setFilters({ status: e.target.value as typeof filters.status })} title={LEGEND}>
          <option value="all">Any status</option>
          <option value="pending">Update pending</option>
          <option value="old">Older than game patch</option>
          <option value="se">Needs script extender</option>
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
        {updatedCount > 0 && filters.enabled !== "updated" && (
          <button className="badge border-ok text-ok whitespace-nowrap" onClick={() => setFilters({ enabled: "updated" })} title="Enabled mods changed since this profile was last played">
            {updatedCount} updated
          </button>
        )}
        <span className="text-textMuted whitespace-nowrap" title={`${rows.filter((r) => r.kind === "mod").length} shown of ${packCount}`}>
          {enabledCount} enabled / {packCount}
        </span>
        <button className="btn" disabled={!groupsVisible} title="Add a separator; the mods below it form a group you can toggle, fold and drag" onClick={() => void newSeparator(focused)}>
          + Group
        </button>
        <button
          className="btn"
          disabled={gameRunning}
          title="Sort alphabetically by pack file name (the CA launcher's default order), inside each group"
          onClick={async () => (await askConfirm("Sort the load order alphabetically by pack file name (within each group)?", "Sort")) && void sortProfileAlpha()}
        >
          Sort A→Z
        </button>
        <button className="btn" disabled={gameRunning} title="Move enabled mods above disabled ones, inside each group" onClick={() => void enabledToTop()}>
          Enabled to top
        </button>
      </div>

      <div className="flex-1 overflow-auto">
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
        <SortableContext items={mainRows.map((r) => r.key)} strategy={verticalListSortingStrategy}>
        <table className="w-full border-collapse text-[12px]">
          <thead className="sticky top-0 bg-panelHeader z-10">
            <tr className="text-left text-textMuted">
              <th className="w-8 px-2 py-1 font-normal" />
              <th className="w-12 px-1 py-1 font-normal text-left" title={LEGEND}>
                <button className="hover:text-text" onClick={() => setSort("status")}>
                  ●{sort.key === "status" ? (sort.dir === 1 ? " ▲" : " ▼") : ""}
                </button>
              </th>
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
              <tbody>
                {mainRows.map((row) =>
                  row.kind === "sep" ? (
                    <SeparatorRow
                      key={row.key}
                      row={row}
                      canDrag={canDrag}
                      disabled={gameRunning}
                      onToggle={(on) => void toggleGroup(row.key, on)}
                      onCollapse={() => void setCollapsed(row.key, !row.entry.collapsed)}
                      onRename={(label) => void renameSeparator(row.key, label)}
                      onContextMenu={openMenu}
                    />
                  ) : (
                    <ModRow
                      key={row.key}
                      row={row}
                      selected={selected.includes(row.key)}
                      focused={focused === row.key}
                      canDrag={canDrag}
                      disabled={gameRunning}
                      onClick={onRowClick}
                      onToggle={onToggle}
                      onContextMenu={openMenu}
                    />
                  ),
                )}
              </tbody>
          {movieRows.length > 0 && (
            <tbody>
              <tr>
                <td colSpan={COLUMNS.length + 2} className="px-2 pt-3 pb-1 text-[11px] text-warn border-t border-edge">
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
                  onContextMenu={openMenu}
                />
              ))}
            </tbody>
          )}
        </table>
        </SortableContext>
        </DndContext>
        {rows.length === 0 && <div className="p-6 text-center text-textMuted text-[12px]">No packs match the current filters.</div>}
      </div>
      {menu && <ContextMenu x={menu.x} y={menu.y} items={menuItems(menu.row)} onClose={() => setMenu(null)} />}
    </section>
  );
}

function SeparatorRow({
  row,
  canDrag,
  disabled,
  onToggle,
  onCollapse,
  onRename,
  onContextMenu,
}: {
  row: SepRowData;
  canDrag: boolean;
  disabled: boolean;
  onToggle: (enabled: boolean) => void;
  onCollapse: () => void;
  onRename: (label: string) => void;
  onContextMenu: (row: Row, e: React.MouseEvent) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: row.key, disabled: !canDrag });
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState(row.entry.label ?? "");
  const style: React.CSSProperties = { transform: CSS.Transform.toString(transform), transition, opacity: isDragging ? 0.5 : undefined };
  const commit = () => {
    setEditing(false);
    if (text.trim() && text.trim() !== row.entry.label) onRename(text.trim());
  };
  return (
    <tr
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...(canDrag && !editing ? listeners : {})}
      onContextMenu={(e) => {
        e.preventDefault();
        onContextMenu(row, e);
      }}
      className={`border-t border-edge bg-panelAlt ${canDrag ? "cursor-grab" : ""}`}
    >
      <td className="px-2 py-1" onClick={(e) => e.stopPropagation()}>
        <input
          type="checkbox"
          checked={row.state === "all"}
          ref={(el) => {
            if (el) el.indeterminate = row.state === "some";
          }}
          disabled={disabled || row.state === "empty"}
          onChange={(e) => onToggle(e.target.checked)}
          onPointerDown={(e) => e.stopPropagation()}
          title="Enable / disable the whole group"
        />
      </td>
      <td colSpan={COLUMNS.length + 1} className="px-1 py-1">
        <div className="flex items-center gap-2">
          <button
            className="w-5 text-textMuted hover:text-text"
            onClick={onCollapse}
            onPointerDown={(e) => e.stopPropagation()}
            title={row.entry.collapsed ? "Expand" : "Collapse"}
          >
            {row.entry.collapsed ? "▸" : "▾"}
          </button>
          {editing ? (
            <input
              autoFocus
              value={text}
              onChange={(e) => setText(e.target.value)}
              onBlur={commit}
              onKeyDown={(e) => {
                if (e.key === "Enter") commit();
                if (e.key === "Escape") setEditing(false);
              }}
              onPointerDown={(e) => e.stopPropagation()}
              className="w-64"
            />
          ) : (
            <span
              className="font-semibold tracking-wide text-accent"
              onDoubleClick={() => {
                setText(row.entry.label ?? "");
                setEditing(true);
              }}
              title="Double-click to rename; drag to move the whole group"
            >
              {row.entry.label || "Group"}
            </span>
          )}
          <span className="text-textMuted">
            {row.enabledMembers}/{row.members} enabled
          </span>
          <div className="flex-1 border-t border-edge/60" />
        </div>
      </td>
    </tr>
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
  row: ModRowData;
  selected: boolean;
  focused: boolean;
  canDrag: boolean;
  disabled: boolean;
  onClick: (row: Row, e: React.MouseEvent) => void;
  onToggle: (row: ModRowData, checked: boolean) => void;
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
      <td className="px-1 py-0.5 whitespace-nowrap">
        <span className="inline-flex items-center gap-1">
          <span className={`inline-block w-2.5 h-2.5 rounded-full ${DOT_STYLE[row.status.kind]}`} title={`${STATUS_LABEL[row.status.kind]}: ${row.status.text}`} />
          {row.se.required && (
            <span
              className={`text-[9px] leading-3 px-1 rounded border font-semibold ${row.seUnmet ? "border-danger bg-danger/20 text-danger" : "border-se/70 text-se"}`}
              title={`${seSourceText(row.se)}${row.seUnmet ? "\nNot available for this launch: turn on the script extender (right panel)" : ""}`}
            >
              SE
            </span>
          )}
        </span>
      </td>
      <td className="px-2 py-0.5 text-right text-textMuted tabular-nums">{isMovie ? "" : row.order}</td>
      <td className="px-2 py-0.5">
        <div className="flex items-center gap-1.5 min-w-0">
          <span className={`truncate ${missing ? "line-through" : ""}`} title={row.mod?.file ?? row.key}>
            {row.title}
          </span>
          {missing && <span className="badge border-danger text-danger">missing</span>}
          {row.isUpdated && (
            <span className="badge border-ok text-ok" title="Changed since this profile was last played">
              updated
            </span>
          )}
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
          <span
            className={`badge ${row.mod.source === "workshop" ? "border-accent/50 text-accent" : row.mod.source === "folder" ? "border-ok/60 text-ok" : "border-edge text-textMuted"}`}
            title={row.mod.dir}
          >
            {SOURCE_LABEL[row.mod.source]}
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
