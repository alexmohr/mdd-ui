<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { ref, nextTick, computed, watch } from "vue";
import { useAppStore } from "../stores/app";
import type { VisibleNode } from "../api/commands";
import { getNodePath } from "../api/commands";

const store = useAppStore();
const scrollContainer = ref<HTMLElement | null>(null);

watch(
  () => store.selectedIndex,
  async () => {
    await nextTick();
    if (!scrollContainer.value) return;
    const el = scrollContainer.value.querySelector<HTMLElement>(`[data-index="${store.selectedIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }
);

const canSort = computed(() =>
  store.fileLoaded && store.selectedNode !== null && store.selectedNode.is_sortable
);

// --- Context menu ---
const ctxMenu = ref<{ x: number; y: number; node: VisibleNode } | null>(null);

function onContextMenu(e: MouseEvent, node: VisibleNode) {
  e.preventDefault();
  ctxMenu.value = { x: e.clientX, y: e.clientY, node };
  nextTick(() => window.addEventListener("click", closeCtx, { once: true }));
}
function closeCtx() { ctxMenu.value = null; }

async function ctxAction(action: string) {
  const node = ctxMenu.value?.node;
  ctxMenu.value = null;
  if (!node) return;
  switch (action) {
    case "select": await store.selectNode(node.index); break;
    case "expand": if (node.has_children) await store.toggleExpand(node.index); break;
    case "expandAll": await store.expandAll(); break;
    case "collapseAll": await store.collapseAll(); break;
    case "goToParent":
      // Navigate to parent inherited-from (uses goBack as approximation for now)
      await store.selectNode(node.index);
      break;
    case "copyName":
      await navigator.clipboard.writeText(node.text);
      break;
    case "copyPath": {
      const path = await getNodePath(node.index);
      await navigator.clipboard.writeText(path);
      break;
    }
    case "sort": await store.toggleSort(node.index); break;
  }
}

// --- Badges ---
type Badge = { label: string; bg: string; fg: string };
const INH_BADGE: Badge = { label: "INH", bg: "bg-amber-500/15", fg: "text-amber-400" };

// Matches text-embedded prefix badges such as "[TIMING] name" or "[Audience] name".
const TEXT_BADGE_RE = /^\[([A-Za-z][A-Za-z0-9_]*)\] /;

// Maps known embedded prefix labels to styled badges.
const EMBEDDED_BADGES: Record<string, Badge> = {
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

/** Return the badge for an embedded text prefix, or null if none is present. */
function textPrefixBadge(node: VisibleNode): Badge | null {
  const m = TEXT_BADGE_RE.exec(node.text);
  if (!m) return null;
  const cls = m[1];
  return EMBEDDED_BADGES[cls] ?? { label: cls.slice(0, 4).toUpperCase(), bg: "bg-sky-500/20", fg: "text-sky-300" };
}

/** Strip an embedded text prefix (e.g. "[TIMING] ") before display. */
function nodeDisplayText(node: VisibleNode): string {
  const m = TEXT_BADGE_RE.exec(node.text);
  return m ? node.text.slice(m[0].length) : node.text;
}

function nodeBadges(node: VisibleNode): Badge[] {
  const badges: Badge[] = [];

  // Node-type badge
  switch (node.node_type) {
    case "Service":
    case "ParentRefService": badges.push({ label: "SVC",  bg: "bg-violet-500/20",  fg: "text-violet-300" }); break;
    case "Job":              badges.push({ label: "JOB",  bg: "bg-violet-500/15",  fg: "text-violet-300/70" }); break;
    case "Request":          badges.push({ label: "REQ",  bg: "bg-teal-500/20",    fg: "text-teal-300" }); break;
    case "PosResponse":      badges.push({ label: "R+",   bg: "bg-emerald-500/20", fg: "text-emerald-300" }); break;
    case "NegResponse":      badges.push({ label: "R-",   bg: "bg-rose-500/20",    fg: "text-rose-300" }); break;
    case "FunctionalClass":  badges.push({ label: "FC",   bg: "bg-orange-500/20",  fg: "text-orange-300" }); break;
    case "Sdg":              badges.push({ label: "SDG",  bg: "bg-lime-500/20",    fg: "text-lime-300" }); break;
    case "Dop":              badges.push({ label: "DOP",  bg: "bg-pink-500/20",    fg: "text-pink-300" }); break;
    case "ParentRefs":       badges.push({ label: "REF",  bg: "bg-cyan-500/20",    fg: "text-cyan-300" }); break;
    case "DopNormal":        badges.push({ label: "DOP",  bg: "bg-pink-500/20",    fg: "text-pink-300" });    break;
    case "DopDtc":           badges.push({ label: "DTC",  bg: "bg-red-500/20",     fg: "text-red-300" });     break;
    case "DopStructure":     badges.push({ label: "STRC", bg: "bg-fuchsia-500/20", fg: "text-fuchsia-300" }); break;
    case "DopStaticField":   badges.push({ label: "SF",   bg: "bg-purple-500/20",  fg: "text-purple-300" });  break;
    case "DopDynamic":       badges.push({ label: "DYN",  bg: "bg-yellow-500/20",  fg: "text-yellow-300" });  break;
    case "DopEndOfPdu":      badges.push({ label: "EOP",  bg: "bg-green-500/20",   fg: "text-green-300" });   break;
    case "DopMux":           badges.push({ label: "MUX",  bg: "bg-blue-500/20",   fg: "text-blue-300" });    break;
    case "DopEnvData":       badges.push({ label: "ENV",  bg: "bg-indigo-500/20", fg: "text-indigo-300" });  break;
    case "DopEnvDataDesc":   badges.push({ label: "EDD",  bg: "bg-sky-500/20",     fg: "text-sky-300" });     break;
    default: break;
  }

  // INH badge always comes last
  if (node.node_type === "ParentRefService") badges.push(INH_BADGE);

  if (badges.length > 0) return badges;

  // Embedded prefix badge: "[CLASS] name" patterns (ComParam classes, Audience, …)
  const tb = textPrefixBadge(node);
  if (tb) return [tb];

  // Infer badge from text for DOP category children and service-list headers
  const t = node.text.toLowerCase();
  const infer = (b: Badge) => [b];
  if (t.startsWith("diag-comms"))       return infer({ label: "DC",  bg: "bg-violet-500/15", fg: "text-violet-300/70" });
  if (t.startsWith("requests"))          return infer({ label: "REQ", bg: "bg-teal-500/15",   fg: "text-teal-300/70" });
  if (t.startsWith("pos-response") || t.startsWith("positive response"))
                                          return infer({ label: "R+",  bg: "bg-emerald-500/15", fg: "text-emerald-300/70" });
  if (t.startsWith("neg-response") || t.startsWith("negative response"))
                                          return infer({ label: "R-",  bg: "bg-rose-500/15",   fg: "text-rose-300/70" });
  if (t.startsWith("functional classes")) return infer({ label: "FC",  bg: "bg-orange-500/15", fg: "text-orange-300/70" });
  if (t.startsWith("comparam"))          return infer({ label: "CP",  bg: "bg-cyan-500/15",   fg: "text-cyan-300/70" });
  if (t.startsWith("state chart"))       return infer({ label: "SC",  bg: "bg-amber-500/15",  fg: "text-amber-300/70" });
  if (t.startsWith("sdgs"))             return infer({ label: "SDG", bg: "bg-lime-500/15",   fg: "text-lime-300/70" });
  // DOP sub-categories
  if (t.startsWith("structures"))        return infer({ label: "STRC", bg: "bg-fuchsia-500/15", fg: "text-fuchsia-300/70" });
  if (t.startsWith("data object"))       return infer({ label: "DOP",  bg: "bg-pink-500/15",    fg: "text-pink-300/70" });
  if (t.startsWith("dtc dop"))           return infer({ label: "DTC",  bg: "bg-red-500/15",     fg: "text-red-300/70" });
  if (t.startsWith("env data descs"))    return infer({ label: "EDD",  bg: "bg-sky-500/15",     fg: "text-sky-300/70" });
  if (t.startsWith("env data"))          return infer({ label: "ENV",  bg: "bg-indigo-500/15", fg: "text-indigo-300/70" });
  if (t.startsWith("static field"))      return infer({ label: "SF",   bg: "bg-purple-500/15",  fg: "text-purple-300/70" });
  if (t.startsWith("dynamic"))           return infer({ label: "DYN",  bg: "bg-yellow-500/15",  fg: "text-yellow-300/70" });
  if (t.startsWith("end of pdu"))        return infer({ label: "EOP",  bg: "bg-green-500/15",  fg: "text-green-300/70" });
  if (t.startsWith("mux"))              return infer({ label: "MUX",  bg: "bg-blue-500/15",   fg: "text-blue-300/70" });

  return badges;
}

function diffBadge(status: string | null): Badge | null {
  switch (status) {
    case "Added":    return { label: "+",  bg: "bg-emerald-500/20", fg: "text-emerald-300" };
    case "Removed":  return { label: "-",  bg: "bg-red-500/20",     fg: "text-red-300" };
    case "Modified": return { label: "~",  bg: "bg-amber-500/20",   fg: "text-amber-300" };
    default:         return null;
  }
}

function nodeTextClass(node: VisibleNode): string {
  if (node.diff_status === "Removed") return "text-neutral-500 line-through";
  if (node.diff_status === "Unchanged") return "text-neutral-600";
  if (node.node_type === "SectionHeader") return "text-white";
  if (node.node_type === "Container") return "text-neutral-100";
  if (node.node_type === "ParentRefService") return "text-neutral-500";
  if (node.node_type === "ParentRefs" || node.node_type === "Dop") return "text-neutral-200";
  return "text-neutral-300";
}

async function onClick(node: VisibleNode) {
  await store.selectNode(node.index);
}
async function onDblClick(node: VisibleNode) {
  if (node.has_children) await store.toggleExpand(node.index);
}
async function onChevronClick(e: Event, node: VisibleNode) {
  e.stopPropagation();
  if (node.has_children) await store.toggleExpand(node.index);
}

// --- Keyboard navigation ---
async function onKeydown(e: KeyboardEvent) {
  if (store.nodes.length === 0) return;
  if (e.key === "ArrowDown" || e.key === "j") {
    e.preventDefault();
    const curPos = store.nodes.findIndex((n: VisibleNode) => n.index === store.selectedIndex);
    const next = Math.min(curPos + 1, store.nodes.length - 1);
    if (next >= 0) await store.selectNode(store.nodes[next].index);
  } else if (e.key === "ArrowUp" || e.key === "k") {
    e.preventDefault();
    const curPos = store.nodes.findIndex((n: VisibleNode) => n.index === store.selectedIndex);
    const prev = Math.max(curPos - 1, 0);
    await store.selectNode(store.nodes[prev].index);
  } else if (e.key === "ArrowRight" || e.key === "l") {
    if (store.selectedNode && store.selectedNode.has_children && !store.selectedNode.expanded)
      await store.toggleExpand(store.selectedNode.index);
  } else if (e.key === "ArrowLeft" || e.key === "h") {
    if (store.selectedNode && store.selectedNode.has_children && store.selectedNode.expanded)
      await store.toggleExpand(store.selectedNode.index);
  } else if (e.key === " ") {
    e.preventDefault();
    if (store.selectedNode && store.selectedNode.has_children)
      await store.toggleExpand(store.selectedNode.index);
  }
}
</script>

<template>
  <div class="flex flex-col h-full bg-neutral-950 outline-none select-none" tabindex="0" @keydown="onKeydown">
    <!-- Header -->
    <div class="flex items-center h-8 px-2 border-b border-neutral-800/50 shrink-0 gap-1 bg-neutral-900/50">
      <span class="text-[0.8em] text-neutral-500 font-semibold uppercase tracking-widest flex-1">Explorer</span>
      <button class="p-1 rounded-md text-neutral-600 hover:text-neutral-300 hover:bg-neutral-800 transition-all" title="Search (/)" @click="store.searchActive = true">
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
      </button>
      <button class="p-1 rounded-md transition-all relative" :disabled="!canSort" :title="`Sort (s) — ${store.sortLabel}`" :class="canSort ? 'text-neutral-600 hover:text-neutral-300 hover:bg-neutral-800' : 'text-neutral-800 cursor-not-allowed'" @click="canSort && store.toggleSort()">
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 16 4 4 4-4"/><path d="M7 20V4"/><path d="m21 8-4-4-4 4"/><path d="M17 4v16"/></svg>
        <span class="absolute bottom-0 -right-1.5 text-[9px] font-bold leading-none bg-neutral-950 px-px rounded-sm pointer-events-none" :class="canSort ? 'text-blue-400' : 'text-neutral-700'">{{ store.sortLabel }}</span>
      </button>
      <button class="p-1 rounded-md text-neutral-600 hover:text-neutral-300 hover:bg-neutral-800 transition-all" title="Expand all (e)" @click="store.expandAll()">
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/></svg>
      </button>
      <button class="p-1 rounded-md text-neutral-600 hover:text-neutral-300 hover:bg-neutral-800 transition-all" title="Collapse all (c)" @click="store.collapseAll()">
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 20 5-5 5 5"/><path d="m7 4 5 5 5-5"/></svg>
      </button>
    </div>


    <!-- Node list -->
    <div ref="scrollContainer" class="flex-1 overflow-y-auto overflow-x-hidden py-0.5">
      <div v-if="store.nodes.length === 0" class="text-neutral-700 text-center text-xs mt-12">
        No nodes loaded
      </div>
      <div
        v-for="node in store.nodes"
        :key="node.index"
        :data-index="node.index"
        class="flex items-center cursor-pointer transition-all duration-100 group gap-1 relative"
        :style="{ height: store.rowHeightPx + 'px', paddingLeft: `${node.depth * 14 + 6}px` }"
        :class="node.index === store.selectedIndex
          ? 'bg-blue-500/10 border-l-2 border-l-blue-500/70'
          : 'hover:bg-neutral-800/60 border-l-2 border-l-transparent'"
        @click="onClick(node)"
        @dblclick="onDblClick(node)"
        @contextmenu="onContextMenu($event, node)"
      >
        <!-- Chevron -->
        <span
          v-if="node.has_children"
          class="w-4 h-4 flex items-center justify-center text-neutral-600 group-hover:text-neutral-400 shrink-0 transition-transform"
          :class="node.expanded ? 'rotate-0' : '-rotate-90'"
          @click="onChevronClick($event, node)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="m7 10 5 5 5-5z"/></svg>
        </span>
        <span v-else class="w-4 shrink-0" />

        <!-- Diff badge -->
        <span
          v-if="diffBadge(node.diff_status)"
          class="inline-flex items-center justify-center rounded py-px text-[9px] font-bold leading-none shrink-0 w-8"
          :class="`${diffBadge(node.diff_status)!.bg} ${diffBadge(node.diff_status)!.fg}`"
        >{{ diffBadge(node.diff_status)!.label }}</span>

        <!-- Type badges -->
        <span
          v-for="(badge, bi) in nodeBadges(node)"
          :key="bi"
          class="inline-flex items-center justify-center rounded py-px text-[9px] font-semibold leading-none shrink-0 w-8"
          :class="`${badge.bg} ${badge.fg}`"
        >{{ badge.label }}</span>

        <!-- Label -->
        <span class="truncate text-sm leading-tight" :class="nodeTextClass(node)">{{ nodeDisplayText(node) }}</span>
      </div>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <div
        v-if="ctxMenu"
        class="fixed z-50 min-w-48 py-1.5 bg-neutral-900/95 backdrop-blur-xl border border-neutral-700/80 rounded-xl shadow-2xl shadow-black/50 text-sm"
        :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
      >
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-blue-500/10 hover:text-neutral-100 transition-colors rounded-md mx-0.5 max-w-[calc(100%-4px)]" @click="ctxAction('select')">
          View details
        </button>
        <button
          v-if="ctxMenu.node.has_children"
          class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-blue-500/10 hover:text-neutral-100 transition-colors rounded-md mx-0.5 max-w-[calc(100%-4px)]"
          @click="ctxAction('expand')"
        >
          {{ ctxMenu.node.expanded ? 'Collapse' : 'Expand' }}
        </button>
        <div class="h-px bg-neutral-800/80 my-1.5 mx-2" />
        <button
          v-if="ctxMenu.node.node_type === 'ParentRefService'"
          class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-blue-500/10 hover:text-neutral-100 transition-colors rounded-md mx-0.5 max-w-[calc(100%-4px)]"
          @click="ctxAction('goToParent')"
        >
          Go to parent definition
        </button>
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-blue-500/10 hover:text-neutral-100 transition-colors rounded-md mx-0.5 max-w-[calc(100%-4px)]" @click="ctxAction('copyName')">
          Copy name
        </button>
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-blue-500/10 hover:text-neutral-100 transition-colors rounded-md mx-0.5 max-w-[calc(100%-4px)]" @click="ctxAction('copyPath')">
          Copy path
        </button>
        <div class="h-px bg-neutral-800/80 my-1.5 mx-2" />
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-blue-500/10 hover:text-neutral-100 transition-colors rounded-md mx-0.5 max-w-[calc(100%-4px)]" @click="ctxAction('sort')">
          Sort tree
        </button>
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-blue-500/10 hover:text-neutral-100 transition-colors rounded-md mx-0.5 max-w-[calc(100%-4px)]" @click="ctxAction('expandAll')">
          Expand all
        </button>
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-blue-500/10 hover:text-neutral-100 transition-colors rounded-md mx-0.5 max-w-[calc(100%-4px)]" @click="ctxAction('collapseAll')">
          Collapse all
        </button>
      </div>
    </Teleport>
  </div>
</template>
