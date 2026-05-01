<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<!-- SOVD detail widget for a specific request or response. -->
<!-- Uses the UDS hex assembled by the parent (DetailPane) for conversions. -->
<!-- Coded consts are always read-only in the SOVD view. -->

<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import * as api from "../api/commands";
import type { ServiceSchemaResult } from "../api/commands";

const props = defineProps<{
  serviceName: string;
  sectionType?: string;
  udsHex?: string;
  variantName?: string | null;
}>();

const schema = ref<ServiceSchemaResult | null>(null);
const schemaLoading = ref(false);
const sovdPath = ref<string | null>(null);
const error = ref<string | null>(null);
const busy = ref(false);

const isRequestSection = computed(() => props.sectionType === "Requests");

// ── SOVD schema fields ──────────────────────────────────────────
interface SovdField {
  name: string;
  isConst: boolean;
  constValue: unknown;
  type: string;
}

function extractSchemaFields(schemaObj: unknown): SovdField[] {
  if (!schemaObj || typeof schemaObj !== "object") return [];
  const s = schemaObj as Record<string, unknown>;

  let fieldProps: Record<string, unknown> | undefined;

  if (s.properties && typeof s.properties === "object") {
    const outerProps = s.properties as Record<string, unknown>;
    const keys = Object.keys(outerProps);
    if (keys.length === 1) {
      const inner = outerProps[keys[0]] as Record<string, unknown> | undefined;
      if (inner?.properties && typeof inner.properties === "object") {
        fieldProps = inner.properties as Record<string, unknown>;
      }
    }
    if (!fieldProps) {
      fieldProps = outerProps;
    }
  }

  if (!fieldProps) return [];

  return Object.entries(fieldProps).map(([name, def]) => {
    const d = (def ?? {}) as Record<string, unknown>;
    const hasConst = "const" in d;
    return {
      name,
      isConst: hasConst,
      constValue: hasConst ? d.const : undefined,
      type: (d.type as string) ?? "string",
    };
  });
}

const sovdFields = computed<SovdField[]>(() => {
  if (!schema.value) return [];
  const schemaObj = isRequestSection.value
    ? schema.value.request_schema
    : schema.value.response_schema;
  return schemaObj ? extractSchemaFields(schemaObj) : [];
});

// Editable SOVD values for non-const fields
const editableSovd = ref<Record<string, string>>({});

function initEditableSovd() {
  const next: Record<string, string> = {};
  for (const f of sovdFields.value) {
    if (!f.isConst) {
      next[f.name] = editableSovd.value[f.name] ?? "";
    }
  }
  editableSovd.value = next;
}

// Live SOVD payload preview
const sovdPayloadPreview = computed(() => {
  if (sovdFields.value.length === 0) return null;
  const json: Record<string, unknown> = {};
  for (const f of sovdFields.value) {
    if (f.isConst) {
      json[f.name] = f.constValue;
    } else {
      const val = editableSovd.value[f.name] ?? "";
      const num = Number(val);
      json[f.name] = val !== "" && !isNaN(num) ? num : val;
    }
  }
  return JSON.stringify(json, null, 2);
});

// Effective SOVD path (from schema or lookup)
const effectiveSovdPath = computed(() => schema.value?.sovd_path ?? sovdPath.value);

// ── SOVD → UDS conversion ───────────────────────────────────────
const sovdToUdsHex = ref<string | null>(null);

async function doSovdToUds() {
  error.value = null;
  sovdToUdsHex.value = null;
  busy.value = true;
  try {
    const json: Record<string, unknown> = {};
    for (const f of sovdFields.value) {
      if (f.isConst) {
        json[f.name] = f.constValue;
      } else {
        json[f.name] = editableSovd.value[f.name] ?? "";
      }
    }
    const result = await api.sovdToUds(props.serviceName, json, props.variantName);
    sovdToUdsHex.value = result.hex_bytes;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

// ── UDS → SOVD conversion ───────────────────────────────────────
const udsToSovdJson = ref<string | null>(null);

async function doUdsToSovd() {
  const hex = props.udsHex ?? "";
  if (!hex || hex.includes("??")) return;
  error.value = null;
  udsToSovdJson.value = null;
  busy.value = true;
  try {
    const result = await api.udsToSovd(props.serviceName, hex, isRequestSection.value, props.variantName);
    udsToSovdJson.value = JSON.stringify(result.json, null, 2);
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

// ── Schema + SOVD path loading ──────────────────────────────────
async function loadSchema() {
  schemaLoading.value = true;
  try {
    schema.value = await api.serviceSchema(props.serviceName, props.variantName);
  } catch {
    // Schema unavailable — try to get at least the SOVD path
    try {
      const matches = await api.sovdLookup(props.serviceName);
      const exact = matches.find((m) => m.name === props.serviceName);
      sovdPath.value = exact?.sovd_path ?? matches[0]?.sovd_path ?? null;
    } catch {
      // non-fatal
    }
  } finally {
    schemaLoading.value = false;
  }
}

// ── Lifecycle ───────────────────────────────────────────────────
watch(() => props.serviceName, () => {
  schema.value = null;
  sovdPath.value = null;
  error.value = null;
  sovdToUdsHex.value = null;
  udsToSovdJson.value = null;
  editableSovd.value = {};
  loadSchema();
});

watch(sovdFields, () => {
  initEditableSovd();
});

onMounted(() => {
  loadSchema();
});
</script>

<template>
  <div class="border border-neutral-800 rounded-lg overflow-hidden">
    <!-- Header -->
    <div class="flex items-center gap-2 px-3 py-1.5 bg-neutral-800/30 border-b border-neutral-800">
      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-neutral-500"><path d="m7 16 4-4-4-4"/><path d="m17 8-4 4 4 4"/><path d="M3 12h4"/><path d="M17 12h4"/></svg>
      <span class="text-[11px] font-medium text-neutral-400">SOVD</span>
    </div>

    <div class="p-3 space-y-3">
      <!-- Error -->
      <div v-if="error" class="px-2 py-1.5 rounded bg-red-900/30 border border-red-700/30 text-red-300 text-xs break-all">
        {{ error }}
      </div>

      <div v-if="schemaLoading" class="text-xs text-neutral-600">Loading…</div>
      <template v-else>
        <!-- SOVD URL -->
        <div v-if="effectiveSovdPath" class="flex items-center gap-2 mb-2">
          <span class="text-[10px] text-neutral-600 uppercase tracking-wide shrink-0">
            {{ isRequestSection ? 'POST' : 'GET' }}
          </span>
          <code class="text-[11px] text-blue-400 font-mono truncate">{{ effectiveSovdPath }}</code>
        </div>

        <!-- Schema-based SOVD fields -->
        <template v-if="sovdFields.length > 0">
          <div class="text-[10px] text-neutral-600 uppercase tracking-wide font-medium mb-1">
            SOVD {{ isRequestSection ? 'Request Payload' : 'Response' }}
          </div>

          <div v-for="f in sovdFields" :key="f.name" class="flex items-center gap-2 py-0.5">
            <span class="text-[11px] text-neutral-500 w-28 shrink-0 truncate font-mono">{{ f.name }}</span>
            <div v-if="f.isConst" class="flex-1 px-2 py-0.5 rounded bg-neutral-800/60 border border-neutral-700/50 text-neutral-500 text-xs font-mono cursor-default">
              {{ f.constValue }}
              <span class="ml-1 text-[10px] text-neutral-700">(const)</span>
            </div>
            <input
              v-else
              v-model="editableSovd[f.name]"
              class="flex-1 px-2 py-0.5 rounded bg-neutral-800 border border-neutral-700 text-neutral-200 text-xs font-mono placeholder-neutral-600 focus:outline-none focus:border-blue-500"
              :placeholder="f.type"
            />
            <span class="text-[10px] text-neutral-700 italic shrink-0">{{ f.type }}</span>
          </div>
        </template>

        <!-- SOVD payload preview -->
        <div v-if="sovdPayloadPreview" class="mt-2 space-y-0.5">
          <div class="text-[10px] text-neutral-600 uppercase tracking-wide font-medium">
            {{ isRequestSection ? 'Request Body (POST)' : 'Response Body' }}
          </div>
          <pre class="px-2 py-1.5 rounded bg-neutral-950 border border-neutral-800 text-blue-300/80 text-[10px] font-mono whitespace-pre-wrap break-all max-h-32 overflow-y-auto">{{ sovdPayloadPreview }}</pre>
        </div>

        <!-- Convert buttons (only with schema) -->
        <div v-if="sovdFields.length > 0" class="flex gap-2 mt-2">
          <button
            v-if="isRequestSection"
            class="px-3 py-1.5 rounded text-xs font-medium transition-colors"
            :class="busy ? 'bg-neutral-700 text-neutral-500 cursor-not-allowed' : 'bg-blue-600 hover:bg-blue-500 text-white'"
            :disabled="busy"
            @click="doSovdToUds"
          >
            {{ busy ? '…' : 'SOVD → UDS Bytes' }}
          </button>
          <button
            class="px-3 py-1.5 rounded text-xs font-medium transition-colors"
            :class="busy || !udsHex || udsHex.includes('??') ? 'bg-neutral-700 text-neutral-500 cursor-not-allowed' : 'bg-emerald-700 hover:bg-emerald-600 text-white'"
            :disabled="busy || !udsHex || udsHex.includes('??')"
            @click="doUdsToSovd"
          >
            {{ busy ? '…' : 'UDS → SOVD' }}
          </button>
        </div>

        <!-- Conversion results -->
        <div v-if="sovdToUdsHex" class="mt-2 space-y-0.5">
          <div class="text-[10px] text-neutral-600 uppercase tracking-wide font-medium">UDS from SOVD</div>
          <div class="px-2 py-1.5 rounded bg-neutral-950 border border-neutral-700 text-green-300 text-xs font-mono break-all">
            {{ sovdToUdsHex }}
          </div>
        </div>
        <div v-if="udsToSovdJson" class="mt-2 space-y-0.5">
          <div class="text-[10px] text-neutral-600 uppercase tracking-wide font-medium">SOVD from UDS</div>
          <pre class="px-2 py-1.5 rounded bg-neutral-950 border border-neutral-700 text-green-300 text-[11px] font-mono whitespace-pre-wrap break-all max-h-48 overflow-y-auto">{{ udsToSovdJson }}</pre>
        </div>

        <!-- No data at all -->
        <div v-if="!effectiveSovdPath && !sovdPayloadPreview" class="text-xs text-neutral-600">
          SOVD data not available for this service
        </div>
      </template>
    </div>
  </div>
</template>
