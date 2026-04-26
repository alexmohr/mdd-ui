<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { computed, ref, nextTick } from "vue";
import { useAppStore } from "../stores/app";
import type { DetailSection, DetailContent, DetailRow, JumpTarget } from "../api/commands";
import { getNodePath } from "../api/commands";

const store = useAppStore();

const headerSection = computed<DetailSection | null>(() => {
  const s = store.detailSections;
  if (s.length > 1 && s[0].render_as_header && "PlainText" in s[0].content) return s[0];
  return null;
});

const tabSections = computed<DetailSection[]>(() =>
  headerSection.value ? store.detailSections.slice(1) : store.detailSections,
);

const activeSection = computed<DetailSection | null>(() =>
  tabSections.value[store.selectedTab] ?? null,
);

function text(c: DetailContent): string[] | null { return "PlainText" in c ? c.PlainText : null; }
function getTable(c: DetailContent) { return "Table" in c ? c.Table : null; }
function composite(c: DetailContent): DetailSection[] | null { return "Composite" in c ? c.Composite : null; }

function diffCls(s: string | null): string {
  if (s === "Added") return "bg-emerald-500/5 border-l-2 border-l-emerald-500/40";
  if (s === "Removed") return "bg-red-500/5 border-l-2 border-l-red-500/30 opacity-60 line-through";
  if (s === "Modified") return "bg-amber-500/5 border-l-2 border-l-amber-500/30";
  return "";
}

async function nav(t: JumpTarget | null) { if (t) await store.navigateTo(t); }

// --- Cell badge parsing ---
// Detects embedded prefix badges such as "[DOP] name" or "[Struct] name" in cell text.
const TEXT_BADGE_RE = /^\[([A-Za-z][A-Za-z0-9_]*)\] /;

type Badge = { label: string; bg: string; fg: string };

const CELL_BADGES: Record<string, Badge> = {
  // DOP variant types (pink – matches tree DOP badges)
  DOP:      { label: "DOP",  bg: "bg-pink-500/20",    fg: "text-pink-300" },
  DTC:      { label: "DTC",  bg: "bg-red-500/20",     fg: "text-red-300" },
  Struct:   { label: "STRC", bg: "bg-fuchsia-500/20", fg: "text-fuchsia-300" },
  SField:   { label: "SF",   bg: "bg-purple-500/20",  fg: "text-purple-300" },
  DynLen:   { label: "DYN",  bg: "bg-yellow-500/20",  fg: "text-yellow-300" },
  EoPdu:    { label: "EOP",  bg: "bg-emerald-500/20", fg: "text-emerald-300" },
  Mux:      { label: "MUX",  bg: "bg-orange-500/20",  fg: "text-orange-300" },
  EnvData:  { label: "ENV",  bg: "bg-teal-500/20",    fg: "text-teal-300" },
  EnvDesc:  { label: "EDD",  bg: "bg-sky-500/20",     fg: "text-sky-300" },
  // ComParam classes (sky – matches tree CP child badges)
  TIMING:   { label: "TMG",  bg: "bg-sky-500/20",     fg: "text-sky-300" },
  BUSCOM:   { label: "BUS",  bg: "bg-orange-500/20",  fg: "text-orange-300" },
  TPCOM:    { label: "TPC",  bg: "bg-teal-500/20",    fg: "text-teal-300" },
  COM:      { label: "COM",  bg: "bg-violet-500/20",  fg: "text-violet-300" },
  ECU_COMM: { label: "ECUC", bg: "bg-lime-500/20",    fg: "text-lime-300" },
  ERRH:     { label: "ERR",  bg: "bg-rose-500/20",    fg: "text-rose-300" },
  TEST:     { label: "TEST", bg: "bg-cyan-500/20",    fg: "text-cyan-300" },
  UNIQ:     { label: "UNQ",  bg: "bg-indigo-500/20",  fg: "text-indigo-300" },
  Audience: { label: "AUD",  bg: "bg-amber-500/20",   fg: "text-amber-300" },
};

/** Split a cell text value into an optional badge + display text. */
function cellParts(raw: string): { badge: Badge | null; text: string } {
  const m = TEXT_BADGE_RE.exec(raw);
  if (!m) return { badge: null, text: raw };
  const cls = m[1];
  const badge = CELL_BADGES[cls] ?? { label: cls.slice(0, 4).toUpperCase(), bg: "bg-sky-500/20", fg: "text-sky-300" };
  return { badge, text: raw.slice(m[0].length) };
}

// --- Column sorting (persisted per section key) ---
type SortState = { col: number; asc: boolean };
const sortStates = ref<Map<string, SortState>>(new Map());
const colWidthMap = ref<Map<string, number[]>>(new Map());

function sectionKey(): string {
  return `${store.selectedIndex ?? ''}-${store.selectedTab}`;
}

function currentSort(): SortState | undefined {
  return sortStates.value.get(sectionKey());
}

function toggleSort(colIdx: number) {
  const key = sectionKey();
  const cur = sortStates.value.get(key);
  if (cur && cur.col === colIdx) {
    sortStates.value.set(key, { col: colIdx, asc: !cur.asc });
  } else {
    sortStates.value.set(key, { col: colIdx, asc: true });
  }
}

function parseNum(s: string): number {
  if (/^0x[0-9a-fA-F]+$/i.test(s)) return parseInt(s, 16);
  return parseFloat(s);
}

function effectiveSort(): SortState {
  const cur = currentSort();
  if (cur) return cur;
  const tbl = activeSection.value ? getTable(activeSection.value.content) : null;
  if (tbl) {
    const byteIdx = tbl.header.cells.findIndex(c => /^byte$/i.test(c.text.trim()));
    if (byteIdx >= 0) return { col: byteIdx, asc: true };
  }
  return { col: 0, asc: true };
}

function sortedRows(rows: DetailRow[]): DetailRow[] {
  const s = effectiveSort();
  const { col, asc } = s;
  return [...rows].sort((a, b) => {
    const at = a.cells[col]?.text ?? "";
    const bt = b.cells[col]?.text ?? "";
    const an = parseNum(at), bn = parseNum(bt);
    const cmp = (!isNaN(an) && !isNaN(bn)) ? an - bn : at.localeCompare(bt);
    return asc ? cmp : -cmp;
  });
}

// --- Column resize (persisted per section key) ---
function onColResize(e: MouseEvent, colIdx: number, key: string) {
  e.preventDefault();
  const startX = e.clientX;
  const startW = (colWidthMap.value.get(key) ?? [])[colIdx] || 120;
  const onMove = (ev: MouseEvent) => {
    const delta = ev.clientX - startX;
    const newW = Math.max(40, startW + delta);
    const arr = [...(colWidthMap.value.get(key) ?? [])];
    arr[colIdx] = newW;
    colWidthMap.value.set(key, arr);
  };
  const onUp = () => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  };
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}

function colStyle(colIdx: number, key: string): Record<string, string> {
  const w = (colWidthMap.value.get(key) ?? [])[colIdx];
  return w ? { width: w + "px", minWidth: w + "px" } : {};
}

// --- Tab context menu ---
interface TabCtx { x: number; y: number; section: DetailSection }
const tabCtxMenu = ref<TabCtx | null>(null);

function onTabContextMenu(e: MouseEvent, section: DetailSection) {
  e.preventDefault();
  tabCtxMenu.value = { x: e.clientX, y: e.clientY, section };
  nextTick(() => window.addEventListener("click", closeTabCtx, { once: true }));
}
function closeTabCtx() { tabCtxMenu.value = null; }

function sectionToMarkdown(section: DetailSection): string {
  const parts: string[] = [`## ${section.title}`, ""];
  const t = text(section.content);
  const tbl = getTable(section.content);
  const comp = composite(section.content);
  if (t) {
    parts.push(...t);
  } else if (tbl) {
    parts.push(tableToMarkdown(tbl.header, tbl.rows));
  } else if (comp) {
    for (const sub of comp) {
      parts.push(`### ${sub.title}`, "");
      const st = text(sub.content);
      const stbl = getTable(sub.content);
      if (st) parts.push(...st, "");
      else if (stbl) parts.push(tableToMarkdown(stbl.header, stbl.rows), "");
    }
  }
  return parts.join("\n");
}

async function tabCtxAction(action: string) {
  const ctx = tabCtxMenu.value;
  tabCtxMenu.value = null;
  if (!ctx) return;
  if (action === "copyMarkdown") {
    await navigator.clipboard.writeText(sectionToMarkdown(ctx.section));
  }
}

// --- Table context menu ---
interface TableCtx {
  x: number;
  y: number;
  header: DetailRow;
  rows: DetailRow[];
}
const tableCtxMenu = ref<TableCtx | null>(null);

function onTableContextMenu(e: MouseEvent, header: DetailRow, rows: DetailRow[]) {
  e.preventDefault();
  tableCtxMenu.value = { x: e.clientX, y: e.clientY, header, rows };
  nextTick(() => window.addEventListener("click", closeTableCtx, { once: true }));
}
function closeTableCtx() { tableCtxMenu.value = null; }

function tableToMarkdown(header: DetailRow, rows: DetailRow[]): string {
  const hCells = header.cells.map(c => c.text);
  const sep = hCells.map(h => "-".repeat(Math.max(3, h.length)));
  const lines = [
    "| " + hCells.join(" | ") + " |",
    "| " + sep.join(" | ") + " |",
  ];
  for (const row of rows) {
    const cells = row.cells.map(c => c.text.replace(/\|/g, "\\|"));
    lines.push("| " + cells.join(" | ") + " |");
  }
  return lines.join("\n");
}

async function tableCtxAction(action: string) {
  const ctx = tableCtxMenu.value;
  tableCtxMenu.value = null;
  if (!ctx) return;
  switch (action) {
    case "copyMarkdown":
      await navigator.clipboard.writeText(tableToMarkdown(ctx.header, ctx.rows));
      break;
    case "copyPath": {
      if (store.selectedIndex !== null) {
        const path = await getNodePath(store.selectedIndex);
        await navigator.clipboard.writeText(path);
      }
      break;
    }
  }
}
</script>

<template>
  <div class="flex flex-col h-full bg-neutral-950">
    <!-- Empty state -->
    <div v-if="!store.selectedNode" class="flex-1 flex items-center justify-center">
      <div class="text-center text-neutral-600 text-sm">
        <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" class="mx-auto mb-3 text-gray-800"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/><path d="m16 15-3-3 3-3"/></svg>
        Select a node
      </div>
    </div>

    <template v-else>
      <!-- Header info -->
      <div
        v-if="headerSection && text(headerSection.content)"
        class="px-4 py-2.5 border-b border-neutral-800/60 bg-neutral-900"
      >
        <div
          v-for="(line, i) in text(headerSection.content)"
          :key="i"
          class="text-[1em] text-neutral-400 leading-relaxed"
        >{{ line }}</div>
      </div>

      <!-- Tabs -->
      <div v-if="tabSections.length > 1" class="flex border-b border-neutral-800/60 bg-neutral-900 overflow-x-auto shrink-0">
        <button
          v-for="(section, i) in tabSections"
          :key="i"
          class="px-4 py-2 text-[1em] whitespace-nowrap border-b-2 transition-colors"
          :class="i === store.selectedTab
            ? 'border-blue-500 text-neutral-100'
            : 'border-transparent text-neutral-500 hover:text-neutral-300'"
          @click="store.setSelectedTab(i)"
          @contextmenu.prevent="onTabContextMenu($event, section)"
        >{{ section.title }}</button>
      </div>

      <!-- Content -->
      <div v-if="activeSection" class="flex-1 overflow-auto">
        <!-- Plain text -->
        <div v-if="text(activeSection.content)" class="p-4 space-y-1">
          <p
            v-for="(line, i) in text(activeSection.content)"
            :key="i"
            class="text-[1em] text-neutral-300 leading-relaxed"
          >{{ line || "\u00A0" }}</p>
        </div>

        <!-- Table -->
        <div v-else-if="getTable(activeSection.content)" @contextmenu="onTableContextMenu($event, getTable(activeSection.content)!.header, sortedRows(getTable(activeSection.content)!.rows))">
          <table class="text-[1em]" style="table-layout: fixed;">
            <thead class="sticky top-0 bg-neutral-950 z-10">
              <tr class="border-b border-gray-800/80">
                <th
                  v-for="(cell, ci) in getTable(activeSection.content)!.header.cells"
                  :key="ci"
                  class="text-left px-3 py-2 text-[0.85em] text-neutral-500 font-medium uppercase tracking-wider cursor-pointer hover:text-neutral-200 select-none relative group"
                  :style="colStyle(ci, sectionKey())"
                  @click="toggleSort(ci)"
                >
                  <span>{{ cell.text }}</span>
                  <span v-if="effectiveSort().col === ci" class="ml-1 text-blue-400">{{ effectiveSort().asc ? '▲' : '▼' }}</span>
                  <span
                    class="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-blue-500/40 opacity-0 group-hover:opacity-100"
                    @mousedown="onColResize($event, ci, sectionKey())"
                    @click.stop
                  />
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(row, ri) in sortedRows(getTable(activeSection.content)!.rows)"
                :key="ri"
                class="border-b border-neutral-800/30 hover:bg-neutral-800/20 transition-colors"
                :class="diffCls(row.diff_status)"
              >
                <td
                  v-for="(cell, ci) in row.cells"
                  :key="ci"
                  class="px-3 py-1.5 text-neutral-300"
                  :class="{
                    'text-blue-400 cursor-pointer hover:text-blue-300 hover:underline': cell.jump_target,
                    'text-neutral-100 font-medium': cell.cell_type === 'ParameterName',
                    'truncate': !store.wrapTableText,
                    'break-words whitespace-normal': store.wrapTableText,
                  }"
                  :style="{ ...(ci === 0 && row.indent > 0 ? { paddingLeft: `${row.indent * 10 + 12}px` } : {}), ...colStyle(ci, sectionKey()) }"
                  @click="nav(cell.jump_target)"
                >
                  <template v-for="p in [cellParts(cell.text)]" :key="0">
                    <span v-if="p.badge" class="inline-flex items-center justify-center rounded px-1 py-px text-[9px] font-semibold leading-none mr-1 shrink-0" :class="`${p.badge.bg} ${p.badge.fg}`">{{ p.badge.label }}</span>{{ p.text }}
                  </template>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Composite -->
        <div v-else-if="composite(activeSection.content)" class="p-3 space-y-3">
          <div
            v-for="(sub, si) in composite(activeSection.content)"
            :key="si"
            class="rounded-lg border border-neutral-800/50 overflow-hidden"
          >
            <div class="px-3 py-1.5 bg-neutral-800/20 text-[0.85em] text-neutral-400 font-medium uppercase tracking-wider">
              {{ sub.title }}
            </div>
            <div v-if="text(sub.content)" class="px-3 py-2">
              <p
                v-for="(line, li) in text(sub.content)"
                :key="li"
                class="text-[1em] text-neutral-300"
              >{{ line || "\u00A0" }}</p>
            </div>
            <table v-else-if="getTable(sub.content)" class="w-full text-[1em]" style="table-layout: fixed;" @contextmenu="onTableContextMenu($event, getTable(sub.content)!.header, getTable(sub.content)!.rows)">
              <thead>
                <tr class="border-b border-neutral-800/50">
                  <th
                    v-for="(cell, ci) in getTable(sub.content)!.header.cells"
                    :key="ci"
                    class="text-left px-3 py-1.5 text-[0.85em] text-neutral-500 font-medium relative group select-none"
                    :style="colStyle(ci, sectionKey() + '-' + si)"
                  >
                    {{ cell.text }}
                    <span
                      class="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-blue-500/40 opacity-0 group-hover:opacity-100"
                      @mousedown="onColResize($event, ci, sectionKey() + '-' + si)"
                      @click.stop
                    />
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(row, ri) in getTable(sub.content)!.rows"
                  :key="ri"
                  class="border-b border-neutral-800/20"
                  :class="diffCls(row.diff_status)"
                >
                  <td
                    v-for="(cell, ci) in row.cells"
                    :key="ci"
                    class="px-3 py-1 text-neutral-300"
                    :class="{
                      'text-blue-400 cursor-pointer hover:underline': cell.jump_target,
                      'truncate': !store.wrapTableText,
                      'break-words whitespace-normal': store.wrapTableText,
                    }"
                    @click="nav(cell.jump_target)"
                  >
                    <template v-for="p in [cellParts(cell.text)]" :key="0">
                      <span v-if="p.badge" class="inline-flex items-center justify-center rounded px-1 py-px text-[9px] font-semibold leading-none mr-1 shrink-0" :class="`${p.badge.bg} ${p.badge.fg}`">{{ p.badge.label }}</span>{{ p.text }}
                    </template>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <div v-else class="flex-1 flex items-center justify-center text-neutral-600 text-xs">
        No details available
      </div>
    </template>
    <!-- Tab context menu -->
    <Teleport to="body">
      <div
        v-if="tabCtxMenu"
        class="fixed z-50 min-w-44 py-1 bg-neutral-900 border border-neutral-700 rounded-lg shadow-xl shadow-black/40 text-[1em]"
        :style="{ left: tabCtxMenu.x + 'px', top: tabCtxMenu.y + 'px' }"
      >
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors" @click="tabCtxAction('copyMarkdown')">
          Copy as Markdown
        </button>
      </div>
    </Teleport>
    <!-- Table context menu -->
    <Teleport to="body">
      <div
        v-if="tableCtxMenu"
        class="fixed z-50 min-w-44 py-1 bg-neutral-900 border border-neutral-700 rounded-lg shadow-xl shadow-black/40 text-[1em]"
        :style="{ left: tableCtxMenu.x + 'px', top: tableCtxMenu.y + 'px' }"
      >
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors" @click="tableCtxAction('copyMarkdown')">
          Copy table as Markdown
        </button>
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors" @click="tableCtxAction('copyPath')">
          Copy path
        </button>
      </div>
    </Teleport>
  </div>
</template>
