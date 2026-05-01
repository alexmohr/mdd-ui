<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<!--
  Visual byte/bit memory-map grid for a "Byte Pattern" detail section.

  Default view: one row per byte with a proportional colour band showing which
  fields occupy that byte.  Click a byte row to expand the bit-level detail
  (8-column grid) for that byte while the rest of the message stays visible.
-->

<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount } from "vue";
import type { DetailRow, JumpTarget } from "../api/commands";

const props = withDefaults(defineProps<{
  rows: DetailRow[];
  title?: string;
  onNavigate?: (target: JumpTarget) => void;
  /** Overridden hex values for editable (non-const) params, keyed by name. */
  editedHex?: Record<string, string>;
}>(), { title: "Byte Pattern" });

// ── Parse helpers ──────────────────────────────────────────────────────────

interface Field {
  byteStart: number;
  byteEnd: number;   // inclusive
  bitHi: number;     // within the START byte, MSB=7
  bitLo: number;     // within the START byte
  multiBytes: boolean;
  name: string;
  hex: string;
  binary: string;
  paramType: string;
  jumpTarget: JumpTarget | null;
  indent: number;
}

// Parse "[7:0]", "[3:2]", "[5]", "" → {hi, lo}
function parseBits(bits: string): { hi: number; lo: number } {
  const range = /\[(\d+):(\d+)\]/.exec(bits);
  if (range) return { hi: Number(range[1]), lo: Number(range[2]) };
  const single = /\[(\d+)\]/.exec(bits);
  if (single) { const b = Number(single[1]); return { hi: b, lo: b }; }
  return { hi: 7, lo: 0 };
}

// Parse "0", "1-3", "2" → {start, end}
function parseOffset(offset: string): { start: number; end: number } {
  const range = /^(\d+)-(\d+)$/.exec(offset.trim());
  if (range) return { start: Number(range[1]), end: Number(range[2]) };
  const n = parseInt(offset.trim(), 10);
  return isNaN(n) ? { start: 0, end: 0 } : { start: n, end: n };
}

const fields = computed<Field[]>(() =>
  props.rows
    .filter(r => r.row_type !== "Header")
    .map(r => {
      const cells = r.cells;
      const offset = cells[0]?.text ?? "0";
      const bits   = cells[1]?.text ?? "";
      const hex    = cells[2]?.text ?? "";
      const binary = cells[3]?.text ?? "";
      const name   = cells[4]?.text ?? "";
      const type_  = cells[5]?.text ?? "";
      const jump   = cells[4]?.jump_target ?? null;

      const { start, end } = parseOffset(offset);
      const { hi, lo }     = parseBits(bits);

      // Use edited value when available
      const editedVal = props.editedHex?.[name];
      const effectiveHex = editedVal !== undefined && editedVal !== ""
        ? (editedVal.length > 0 ? `0x${editedVal}` : hex)
        : hex;

      return {
        byteStart: start,
        byteEnd: end,
        bitHi: hi,
        bitLo: lo,
        multiBytes: end > start,
        name,
        hex: effectiveHex,
        binary,
        paramType: type_,
        jumpTarget: jump,
        indent: r.indent,
      };
    }),
);

const totalBytes = computed(() => {
  if (fields.value.length === 0) return 0;
  return Math.max(...fields.value.map(f => f.byteEnd)) + 1;
});

const PALETTE = [
  "bg-blue-500/20 border-blue-400/40 text-blue-200",
  "bg-emerald-500/20 border-emerald-400/40 text-emerald-200",
  "bg-amber-500/20 border-amber-400/40 text-amber-200",
  "bg-violet-500/20 border-violet-400/40 text-violet-200",
  "bg-rose-500/20 border-rose-400/40 text-rose-200",
  "bg-cyan-500/20 border-cyan-400/40 text-cyan-200",
  "bg-lime-500/20 border-lime-400/40 text-lime-200",
  "bg-orange-500/20 border-orange-400/40 text-orange-200",
  "bg-fuchsia-500/20 border-fuchsia-400/40 text-fuchsia-200",
  "bg-teal-500/20 border-teal-400/40 text-teal-200",
];

const paletteMap = computed(() => {
  const m = new Map<string, number>();
  let idx = 0;
  for (const f of fields.value) {
    if (!m.has(f.name)) {
      m.set(f.name, idx % PALETTE.length);
      idx = idx + 1;
    }
  }
  return m;
});

function fieldColor(f: Field): string {
  return PALETTE[paletteMap.value.get(f.name) ?? 0];
}

// ── Grid cell model ────────────────────────────────────────────────────────
// rows[byteLabel][colIndex 0-7], where col 0 = bit 7, col 7 = bit 0.
// null = consumed by a previous colspan. Multi-byte fields are collapsed
// into a single row labelled "start–end" to avoid repetitive empty rows.

type GridCell =
  | { type: "empty" }
  | { type: "field"; field: Field; colSpan: number; firstInRow: boolean }
  | null;

interface GridRow {
  byteLabel: string;
  cells: GridCell[];
}

const grid = computed<GridRow[]>(() => {
  const total = totalBytes.value;
  const result: GridRow[] = [];
  let b = 0;

  while (b < total) {
    const byteFields = fields.value.filter(f => f.byteStart <= b && f.byteEnd >= b);
    const multiField = byteFields.find(f => f.multiBytes);

    if (multiField) {
      // Collapse all bytes of this field into one labelled row
      const end = multiField.byteEnd;
      const cells: GridCell[] = [
        { type: "field", field: multiField, colSpan: 8, firstInRow: true },
        null, null, null, null, null, null, null,
      ];
      result.push({ byteLabel: end > b ? `${b}\u2013${end}` : `${b}`, cells });
      b = end + 1;
      continue;
    }

    // Single-byte row: place sub-byte fields by bit position
    const cells: GridCell[] = Array.from({ length: 8 }, () => ({ type: "empty" as const }));
    for (const f of byteFields) {
      const hi = Math.min(f.bitHi, 7);
      const lo = Math.max(f.bitLo, 0);
      const span = hi - lo + 1;
      const colStart = 7 - hi;
      cells[colStart] = { type: "field", field: f, colSpan: span, firstInRow: true };
      for (let c = colStart + 1; c < colStart + span; c = c + 1) cells[c] = null;
    }
    result.push({ byteLabel: `${b}`, cells });
    b = b + 1;
  }

  return result;
});

// ── Selection ─────────────────────────────────────────────────────────────
const selected = ref<Field | null>(null);

function select(f: Field) {
  selected.value = selected.value?.name === f.name ? null : f;
}

// ── Hover cross-highlight ─────────────────────────────────────────────────
const hovered = ref<Field | null>(null);
let hoverTimer: ReturnType<typeof setTimeout> | null = null;

function onCellEnter(f: Field) {
  hovered.value = f;
  // Tooltip delay
  if (hoverTimer) clearTimeout(hoverTimer);
  hoverTimer = setTimeout(() => { tooltipVisible.value = true; }, 100);
}

function onGridLeave() {
  hovered.value = null;
  tooltipVisible.value = false;
  if (hoverTimer) { clearTimeout(hoverTimer); hoverTimer = null; }
}

function cellDimmed(f: Field): boolean {
  if (!hovered.value) return false;
  if (selected.value?.name === f.name) return false;
  return hovered.value.name !== f.name;
}

function cellHighlighted(f: Field): boolean {
  if (!hovered.value) return false;
  return hovered.value.name === f.name;
}

// ── Tooltip ───────────────────────────────────────────────────────────────
const tooltipVisible = ref(false);
const tooltipX = ref(0);
const tooltipY = ref(0);
const gridRef = ref<HTMLElement | null>(null);

function onCellMouseMove(event: MouseEvent) {
  tooltipX.value = event.clientX + 12;
  tooltipY.value = event.clientY - 40;
}

// ── Collapse for large messages ───────────────────────────────────────────
const COLLAPSE_THRESHOLD = 8;
const expanded = ref(false);

watch(() => props.rows, () => { expanded.value = false; });

const visibleGrid = computed(() => {
  if (expanded.value || grid.value.length <= COLLAPSE_THRESHOLD) return grid.value;
  return grid.value.slice(0, COLLAPSE_THRESHOLD);
});

const isCollapsible = computed(() => grid.value.length > COLLAPSE_THRESHOLD);

// ── Keyboard navigation ───────────────────────────────────────────────────
// Sorted field list for navigation: by byteStart asc, then bitHi desc
const sortedFields = computed(() =>
  [...fields.value].sort((a, b) => {
    if (a.byteStart !== b.byteStart) return a.byteStart - b.byteStart;
    return b.bitHi - a.bitHi;
  }),
);

function onKeydown(e: KeyboardEvent) {
  const sorted = sortedFields.value;
  if (sorted.length === 0) return;

  if (e.key === "Escape") {
    selected.value = null;
    e.preventDefault();
    return;
  }

  if (e.key === "Enter" && selected.value?.jumpTarget && props.onNavigate) {
    props.onNavigate(selected.value.jumpTarget);
    e.preventDefault();
    return;
  }

  if (e.key === "ArrowDown" || e.key === "ArrowRight") {
    e.preventDefault();
    if (!selected.value) { selected.value = sorted[0]; return; }
    const idx = sorted.findIndex(f => f.name === selected.value!.name);
    const next = sorted[(idx + 1) % sorted.length];
    if (next) selected.value = next;
    return;
  }

  if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
    e.preventDefault();
    if (!selected.value) { selected.value = sorted[sorted.length - 1]; return; }
    const idx = sorted.findIndex(f => f.name === selected.value!.name);
    const prev = sorted[(idx - 1 + sorted.length) % sorted.length];
    if (prev) selected.value = prev;
    return;
  }
}

// ── Copy to clipboard ─────────────────────────────────────────────────────
const copiedField = ref<string | null>(null);

async function copyValue(value: string, label: string) {
  try {
    await navigator.clipboard.writeText(value);
    copiedField.value = label;
    setTimeout(() => { copiedField.value = null; }, 1500);
  } catch { /* clipboard not available */ }
}

onBeforeUnmount(() => {
  if (hoverTimer) clearTimeout(hoverTimer);
});
</script>

<template>
  <div
    ref="gridRef"
    class="select-none text-xs font-mono bg-neutral-900/30 rounded-xl p-3 focus:outline-none focus:ring-1 focus:ring-neutral-700"
    tabindex="0"
    @keydown="onKeydown"
  >
    <!-- Section header -->
    <div class="flex items-center gap-2 px-1 pb-2 border-b border-neutral-800/50 mb-2">
      <svg xmlns="http://www.w3.org/2000/svg" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-neutral-600"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/></svg>
      <span class="text-[11px] text-neutral-500 font-medium uppercase tracking-wide">{{ title }}</span>
    </div>

    <!-- Bit grid -->
    <div class="overflow-x-auto rounded-lg relative" @mouseleave="onGridLeave">
      <table class="w-full border-collapse" style="table-layout: fixed; min-width: 360px;">
        <thead>
          <tr class="bg-neutral-900/60 border-b border-neutral-800">
            <th class="px-2 py-1 text-neutral-600 text-[10px] font-medium text-right border-r border-neutral-800" style="width: 52px;">byte</th>
            <th
              v-for="b in [7,6,5,4,3,2,1,0]"
              :key="b"
              class="py-1 text-neutral-600 text-[10px] font-medium text-center"
              style="width: 11.5%;"
            >{{ b }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in visibleGrid"
            :key="row.byteLabel"
            class="border-b border-neutral-800/50 last:border-0"
          >
            <td class="px-2 py-0.5 text-neutral-600 text-[10px] text-right border-r border-neutral-800 align-middle leading-tight select-none">
              {{ row.byteLabel }}
            </td>
            <template v-for="(cell, ci) in row.cells" :key="ci">
              <template v-if="cell !== null">
                <td
                  v-if="cell.type === 'empty'"
                  class="border border-neutral-800/40 h-7 empty-cell"
                />
                <td
                  v-else
                  :colspan="cell.colSpan"
                  class="border cursor-pointer h-7 px-1 align-middle transition-all duration-100"
                  :class="[
                    fieldColor(cell.field),
                    selected?.name === cell.field.name
                      ? 'ring-1 ring-inset ring-white/30'
                      : 'hover:brightness-125',
                    cellDimmed(cell.field) ? 'opacity-30' : '',
                    cellHighlighted(cell.field) && selected?.name !== cell.field.name ? 'brightness-130' : '',
                  ]"
                  @click="select(cell.field)"
                  @mouseenter="onCellEnter(cell.field)"
                  @mousemove="onCellMouseMove"
                >
                  <span
                    v-if="cell.firstInRow"
                    class="truncate block text-[10px] leading-tight"
                    :class="cell.colSpan < 3 ? 'text-center text-[9px]' : ''"
                  >
                    <template v-if="cell.colSpan >= 4">
                      <span class="opacity-80">{{ cell.field.name }}</span>
                      <span v-if="cell.field.hex?.startsWith('0x')" class="ml-1 opacity-50">{{ cell.field.hex }}</span>
                    </template>
                    <template v-else-if="cell.colSpan >= 2">
                      <span class="opacity-70">{{ cell.field.name.slice(0, 6) }}</span>
                    </template>
                    <template v-else>
                      <span class="opacity-60 text-[8px]">{{ cell.field.name.slice(0, 2) }}</span>
                    </template>
                  </span>
                </td>
              </template>
            </template>
          </tr>
        </tbody>
      </table>

    </div>

    <!-- Custom tooltip (teleported to body to avoid overflow clipping) -->
    <Teleport to="body">
      <div
        v-if="tooltipVisible && hovered"
        class="fixed pointer-events-none bg-neutral-800 border border-neutral-700 rounded-md px-2 py-1.5 text-[10px] font-mono shadow-lg z-[9999] space-y-0.5 max-w-[200px]"
        :style="{ left: tooltipX + 'px', top: tooltipY + 'px' }"
      >
        <div class="text-neutral-100 font-semibold truncate">{{ hovered.name }}</div>
        <div class="text-neutral-400">
          <span class="opacity-60">Hex</span> <span class="text-neutral-200">{{ hovered.hex || '—' }}</span>
        </div>
        <div class="text-neutral-400">
          <span class="opacity-60">Bits</span> <span class="text-neutral-200">[{{ hovered.bitHi }}:{{ hovered.bitLo }}]</span>
        </div>
        <div v-if="hovered.paramType" class="text-neutral-500 truncate">{{ hovered.paramType }}</div>
      </div>
    </Teleport>

    <!-- Collapse toggle -->
    <div v-if="isCollapsible" class="text-center py-1.5">
      <button
        class="text-[10px] text-neutral-500 hover:text-neutral-300 transition-colors"
        @click="expanded = !expanded"
      >
        {{ expanded ? 'Collapse' : `Show all ${grid.length} rows` }}
      </button>
    </div>

    <!-- Detail card for selected field -->
    <transition
      enter-active-class="transition-all duration-150 ease-out"
      enter-from-class="opacity-0 -translate-y-1"
      leave-active-class="transition-all duration-100 ease-in"
      leave-to-class="opacity-0 -translate-y-1"
    >
      <div
        v-if="selected"
        class="mt-2 rounded-lg border px-3 py-2 space-y-1"
        :class="fieldColor(selected)"
      >
        <div class="flex items-center justify-between gap-2">
          <!-- Navigate-to-field: clickable name if jumpTarget exists -->
          <span
            v-if="selected.jumpTarget && onNavigate"
            class="font-semibold text-[11px] text-neutral-100 cursor-pointer hover:underline hover:text-white transition-colors"
            @click="onNavigate(selected.jumpTarget!)"
          >{{ selected.name }} ↗</span>
          <span v-else class="font-semibold text-[11px] text-neutral-100">{{ selected.name }}</span>
          <span class="text-[10px] opacity-60">{{ selected.paramType }}</span>
        </div>
        <div class="grid grid-cols-3 gap-x-4 gap-y-0.5 text-[10px] opacity-80">
          <div>
            <span class="opacity-60">Bytes </span>
            <span class="font-mono">{{ selected.byteStart === selected.byteEnd ? selected.byteStart : `${selected.byteStart}–${selected.byteEnd}` }}</span>
          </div>
          <div>
            <span class="opacity-60">Bits </span>
            <span class="font-mono">[{{ selected.bitHi }}:{{ selected.bitLo }}]</span>
          </div>
          <div class="flex items-center gap-1">
            <span class="opacity-60">Hex </span>
            <span class="font-mono">{{ selected.hex || '—' }}</span>
            <!-- Copy hex button -->
            <button
              v-if="selected.hex"
              class="opacity-40 hover:opacity-80 transition-opacity ml-0.5"
              @click.stop="copyValue(selected.hex, 'hex')"
            >
              <svg v-if="copiedField !== 'hex'" xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-emerald-400"><polyline points="20 6 9 17 4 12"/></svg>
            </button>
          </div>
          <div v-if="selected.binary" class="col-span-3 flex items-center gap-1">
            <span class="opacity-60">Binary </span>
            <span class="font-mono tracking-widest">{{ selected.binary }}</span>
            <!-- Copy binary button -->
            <button
              class="opacity-40 hover:opacity-80 transition-opacity ml-0.5"
              @click.stop="copyValue(selected.binary, 'binary')"
            >
              <svg v-if="copiedField !== 'binary'" xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-emerald-400"><polyline points="20 6 9 17 4 12"/></svg>
            </button>
          </div>
        </div>
        <!-- Keyboard hint -->
        <div class="text-[9px] text-neutral-600 pt-0.5">
          ↑↓ navigate · Enter jump · Esc close
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.empty-cell {
  background-color: rgb(10 10 10); /* neutral-950 */
  background-image: radial-gradient(circle, rgb(38 38 38 / 0.5) 1px, transparent 1px);
  background-size: 6px 6px;
}
</style>
