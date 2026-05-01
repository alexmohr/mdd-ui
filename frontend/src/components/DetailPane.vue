<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { computed, ref, nextTick, watch } from "vue";
import { useAppStore } from "../stores/app";
import type { DetailSection, DetailContent, DetailRow, JumpTarget } from "../api/commands";
import { getNodePath, udsEncode, getNodeVariant } from "../api/commands";
import ByteGridView from "./ByteGridView.vue";

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

// Show UDS hex/byte-grid for request and response sections
const UDS_SECTION_TYPES = new Set(["Requests", "PosResponses", "NegResponses"]);

/** Extract the ODX short name from a display name like "0x22F190 - ReadVIN" → "ReadVIN". */
function extractServiceShortName(displayName: string): string {
  const sep = displayName.indexOf(" - ");
  return sep >= 0 ? displayName.slice(sep + 3) : displayName;
}

const udsServiceName = computed<string | null>(() => {
  if (!activeSection.value) return null;
  if (!UDS_SECTION_TYPES.has(activeSection.value.section_type)) return null;
  const text = store.selectedNode?.text;
  return text ? extractServiceShortName(text) : null;
});

const isRequestSection = computed(() => activeSection.value?.section_type === "Requests");

// ── Variant resolution for UDS operations ───────────────────────
// Walk the tree parent chain to find which variant the selected node belongs
// to, so the UDS translator uses the correct variant (not always "base").
const nodeVariant = ref<string | null>(null);

watch(
  () => store.selectedIndex,
  async (idx) => {
    nodeVariant.value = null;
    if (idx == null) return;
    try {
      nodeVariant.value = await getNodeVariant(idx);
    } catch {
      // non-fatal — falls back to whatever variant is currently active
    }
  },
  { immediate: true },
);

// ── Const types that are read-only ──────────────────────────────
const CONST_TYPES = new Set([
  "CodedConst",
  "NrcConst",
  "PhysConst",
  "Reserved",
  "MatchingRequestParam",
]);

// ── Parse byte_pattern_rows for UDS hex assembly ────────────────
interface UdsParam {
  name: string;
  byteOffset: string;
  hex: string;
  paramType: string;
  isConst: boolean;
  bitLength: number;
}

function parseBitLength(bitRange: string): number {
  const range = /\[(\d+):(\d+)\]/.exec(bitRange);
  if (range) return Number(range[1]) - Number(range[2]) + 1;
  const single = /\[(\d+)\]/.exec(bitRange);
  if (single) return 1;
  return 8;
}

function parseBytePatternRows(rows: DetailRow[]): UdsParam[] {
  return rows
    .filter((r) => r.row_type !== "Header")
    .map((r) => {
      const cells = r.cells;
      const paramType = cells[5]?.text ?? "";
      const hex = cells[2]?.text ?? "";
      const bitRange = cells[1]?.text ?? "";
      return {
        name: cells[4]?.text ?? "",
        byteOffset: cells[0]?.text ?? "",
        hex,
        paramType,
        isConst: CONST_TYPES.has(paramType) || hex.startsWith("0x"),
        bitLength: parseBitLength(bitRange),
      };
    });
}

const udsParams = computed<UdsParam[]>(() => {
  const rows = activeSection.value?.byte_pattern_rows;
  return rows ? parseBytePatternRows(rows) : [];
});

// Editable decimal values for variable params (keyed by param name)
const editableValues = ref<Record<string, string>>({});

// CDA-based UDS hex (declared early so the immediate watch below can reference it)
const cdaUdsHex = ref<string | null>(null);

function initEditableValues() {
  const next: Record<string, string> = {};
  for (const p of udsParams.value) {
    if (!p.isConst) {
      next[p.name] = editableValues.value[p.name] ?? "";
    }
  }
  editableValues.value = next;
}

watch(() => activeSection.value?.byte_pattern_rows, () => {
  initEditableValues();
  cdaUdsHex.value = null;
}, { immediate: true });

// ── Map param name → const/editable for table row identification ─
const nonConstParamNames = computed<Set<string>>(() => {
  const s = new Set<string>();
  for (const p of udsParams.value) {
    if (!p.isConst) s.add(p.name);
  }
  return s;
});

/** Get the Short Name of a main-table row (column 0). */
function rowParamName(row: DetailRow): string {
  return row.cells[0]?.text ?? "";
}

function isRowEditable(row: DetailRow): boolean {
  return nonConstParamNames.value.has(rowParamName(row));
}

// ── Value helpers for editable params ────────────────────────────
function bitLengthForParam(name: string): number {
  return udsParams.value.find((u) => u.name === name)?.bitLength ?? 8;
}

function maxValueForBits(bits: number): number {
  if (bits >= 32) return 0xFFFFFFFF;
  return (1 << bits) - 1;
}

function valuePlaceholder(name: string): string {
  const bl = bitLengthForParam(name);
  const max = maxValueForBits(bl);
  return bl <= 8 ? `0–${max} (${bl} bit${bl > 1 ? "s" : ""})` : `0–${max}`;
}

function sanitizeValue(name: string) {
  const raw = editableValues.value[name] ?? "";
  editableValues.value[name] = raw.replace(/[^0-9]/g, "");
}

function normalizeValue(name: string) {
  const raw = editableValues.value[name] ?? "";
  if (raw === "") return;
  const num = parseInt(raw, 10);
  if (isNaN(num)) { editableValues.value[name] = ""; return; }
  const max = maxValueForBits(bitLengthForParam(name));
  editableValues.value[name] = String(Math.min(num, max));
}

// ── Edited hex map for the byte grid tooltip/display ─────────────
const editedHexMap = computed<Record<string, string>>(() => {
  const m: Record<string, string> = {};
  for (const p of udsParams.value) {
    if (p.isConst) continue;
    const raw = editableValues.value[p.name] ?? "";
    if (raw === "") continue;
    const num = parseInt(raw, 10);
    if (isNaN(num)) continue;
    const byteCount = Math.ceil(p.bitLength / 8);
    m[p.name] = num.toString(16).toUpperCase().padStart(byteCount * 2, "0");
  }
  return m;
});

// ── UDS hex from const byte-pattern (fallback preview) ──────────
const constOnlyHex = computed(() => {
  if (udsParams.value.length === 0) return "";
  const byteMap = new Map<number, string>();
  for (const p of udsParams.value) {
    if (!p.isConst || !p.hex.startsWith("0x")) continue;
    let hexOnly = p.hex.replace(/^0x/i, "");
    if (hexOnly.length === 0) continue;
    if (hexOnly.length % 2 !== 0) hexOnly = "0" + hexOnly;
    const byteStart = parseInt(p.byteOffset.trim().split("-")[0] ?? "0", 10);
    for (let i = 0; i < hexOnly.length / 2; i += 1) {
      byteMap.set(byteStart + i, hexOnly.slice(i * 2, i * 2 + 2).toUpperCase());
    }
  }
  if (byteMap.size === 0) return "";
  const maxByte = Math.max(...byteMap.keys());
  return Array.from({ length: maxByte + 1 }, (_, i) => byteMap.get(i) ?? "??").join(" ");
});

// ── CDA-based UDS payload generation ────────────────────────────
let cdaTimer: ReturnType<typeof setTimeout> | null = null;
const cdaError = ref<string | null>(null);

async function generateCdaUds() {
  const name = udsServiceName.value;
  cdaError.value = null;
  if (!name) { cdaUdsHex.value = null; return; }
  const params: Record<string, unknown> = {};
  let hasAny = false;
  for (const p of udsParams.value) {
    if (p.isConst) continue;
    const raw = editableValues.value[p.name] ?? "";
    if (raw === "") continue;
    params[p.name] = raw;
    hasAny = true;
  }
  if (!hasAny) { cdaUdsHex.value = null; return; }
  // Wait for the UDS translator to finish loading before encoding
  await store.waitForUds();
  if (!store.udsReady) {
    cdaError.value = store.udsError ?? "UDS translator not available";
    cdaUdsHex.value = null;
    return;
  }
  try {
    const result = await udsEncode(name, params, nodeVariant.value);
    cdaUdsHex.value = result.hex_bytes;
  } catch (e) {
    cdaError.value = String(e);
    cdaUdsHex.value = null;
  }
}

watch(editableValues, () => {
  if (cdaTimer) clearTimeout(cdaTimer);
  cdaTimer = setTimeout(generateCdaUds, 400);
}, { deep: true });

watch(udsServiceName, () => { cdaUdsHex.value = null; });

const udsHexString = computed(() => cdaUdsHex.value || constOnlyHex.value);

// Value column index in the main param table
const VALUE_COL_INDEX = 5;

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
          class="text-sm text-neutral-400 leading-relaxed"
        >{{ line }}</div>
      </div>

      <!-- Tabs -->
      <div v-if="tabSections.length > 1" class="flex border-b border-neutral-800/60 bg-neutral-900 overflow-x-auto shrink-0">
        <button
          v-for="(section, i) in tabSections"
          :key="i"
          class="px-4 py-2 text-sm whitespace-nowrap border-b-2 transition-colors"
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
            class="text-sm text-neutral-300 leading-relaxed"
          >{{ line || "\u00A0" }}</p>
        </div>

        <!-- Table -->
        <div v-if="getTable(activeSection.content)" @contextmenu="onTableContextMenu($event, getTable(activeSection.content)!.header, sortedRows(getTable(activeSection.content)!.rows))">
          <table class="text-sm" style="table-layout: fixed;">
            <thead class="sticky top-0 bg-neutral-950 z-10">
              <tr class="border-b border-gray-800/80">
                <th
                  v-for="(cell, ci) in getTable(activeSection.content)!.header.cells"
                  :key="ci"
                  class="text-left px-3 py-2 text-xs text-neutral-500 font-medium uppercase tracking-wider cursor-pointer hover:text-neutral-200 select-none relative group"
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
                  <!-- Editable value input for Value column on non-const UDS rows -->
                  <template v-if="isRequestSection && ci === VALUE_COL_INDEX && isRowEditable(row)">
                    <input
                      v-model="editableValues[rowParamName(row)]"
                      class="w-full px-1 py-0.5 rounded bg-neutral-800 border border-neutral-700 text-neutral-200 font-mono text-xs placeholder-neutral-600 focus:outline-none focus:border-blue-500"
                      :placeholder="valuePlaceholder(rowParamName(row))"
                      @input="sanitizeValue(rowParamName(row))"
                      @blur="normalizeValue(rowParamName(row))"
                      @click.stop
                    />
                  </template>
                  <template v-else>
                    <template v-for="p in [cellParts(cell.text)]" :key="0">
                      <span v-if="p.badge" class="inline-flex items-center justify-center rounded px-1 py-px text-[9px] font-semibold leading-none mr-1 shrink-0" :class="`${p.badge.bg} ${p.badge.fg}`">{{ p.badge.label }}</span>{{ p.text }}
                    </template>
                  </template>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- UDS request hex – shown below the table for request sections -->
        <div v-if="isRequestSection && udsServiceName" class="px-3 pt-3 space-y-0.5">
          <div class="text-[10px] uppercase tracking-wide font-medium text-neutral-500">UDS Request</div>
          <div v-if="udsHexString" class="px-2 py-1.5 rounded bg-neutral-950 border border-neutral-700 text-green-300 text-xs font-mono break-all select-all">
            {{ udsHexString }}
          </div>
        </div>
        <div v-if="isRequestSection && cdaError" class="px-3 pt-2">
          <div class="px-2 py-1.5 rounded bg-red-900/30 border border-red-700/30 text-red-300 text-xs break-all">
            {{ cdaError }}
          </div>
        </div>

        <!-- Byte/bit grid – shown below the table when byte_pattern_rows are present -->
        <div
          v-if="activeSection.byte_pattern_rows && activeSection.byte_pattern_rows.length > 0"
          class="px-3 pt-4 pb-3 mt-6"
        >
          <ByteGridView
            :rows="activeSection.byte_pattern_rows"
            :on-navigate="nav"
            :edited-hex="editedHexMap"
          />
        </div>

        <!-- Composite -->
        <div v-if="!text(activeSection.content) && !getTable(activeSection.content) && composite(activeSection.content)" class="p-3 space-y-3">
          <div
            v-for="(sub, si) in composite(activeSection.content)"
            :key="si"
            class="rounded-lg border border-neutral-800/50 overflow-hidden"
          >
            <div class="px-3 py-1.5 bg-neutral-800/20 text-xs text-neutral-400 font-medium uppercase tracking-wider">
              {{ sub.title }}
            </div>
            <div v-if="text(sub.content)" class="px-3 py-2">
              <p
                v-for="(line, li) in text(sub.content)"
                :key="li"
                class="text-sm text-neutral-300"
              >{{ line || "\u00A0" }}</p>
            </div>
            <table v-else-if="getTable(sub.content)" class="w-full text-sm" style="table-layout: fixed;" @contextmenu="onTableContextMenu($event, getTable(sub.content)!.header, getTable(sub.content)!.rows)">
              <thead>
                <tr class="border-b border-neutral-800/50">
                  <th
                    v-for="(cell, ci) in getTable(sub.content)!.header.cells"
                    :key="ci"
                    class="text-left px-3 py-1.5 text-xs text-neutral-500 font-medium relative group select-none"
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
        class="fixed z-50 min-w-44 py-1 bg-neutral-900 border border-neutral-700 rounded-lg shadow-xl shadow-black/40 text-sm"
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
        class="fixed z-50 min-w-44 py-1 bg-neutral-900 border border-neutral-700 rounded-lg shadow-xl shadow-black/40 text-sm"
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
