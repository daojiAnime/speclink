#!/usr/bin/env node
/**
 * Antigravity CLI / App lifecycle hook for Speclink:
 * 1. Mirrors slash commands (`/speclink-*`) and mutating `speclink` CLI tool calls
 *    into Herdr pane labels (`hc label $HC_TERMINAL_ID ...`) when inside Herdr.
 * 2. Writes lightweight focus state under `$TMPDIR/speclink-agy-focus/<conv>.json`
 *    so external statusLine / title scripts can display the active `step · change`.
 *
 * Always exits 0 and emits `{}` so normal prompts/tool calls are never blocked.
 */
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const SKILL_PAIR = {
  discuss: "discussion",
  propose: "discussion",
  improve: "project",
  audit: "project",
  trace: "project",
  manual: "project",
  baseline: "project",
  config: "project",
  ingest: "change",
  apply: "change",
  "apply-with-worktree": "change",
  analyze: "change",
  drift: "change",
  quality: "change",
  review: "change",
  verify: "change",
  archive: "change",
  "archive+commit": "change",
  "worktree-merge": "change",
  commit: "change",
};

const NAME = String.raw`"?([A-Za-z0-9][^\s"';&|]*)"?`;
const BIN = String.raw`["']?(?:[^\s;&|()"']*/)?speclink["']?`;
const speclinkAt = (verb) =>
  new RegExp(String.raw`(?:^|[;&|(]\s*)(?:\w+=\S*\s+)*${BIN}\s+${verb}\s+${NAME}`, "m");

const CHANGE_COMMANDS = [
  "new\\s+change",
  "instructions\\s+apply\\s+--change",
  "task\\s+done\\s+--change",
  "in-progress\\s+add",
  "(?:review|verify)\\s+prepare",
  "archive",
].map(speclinkAt);

const DISCUSSION_COMMANDS = [
  "discuss\\s+new\\s+.*--slug",
  "discuss\\s+(?:context|add-round|conclude)",
].map(speclinkAt);

const FROM_DISCUSSION = new RegExp(
  String.raw`${BIN}\s+new\s+change\s+[^;&|\n]*--from-discussion[\s=]+${NAME}`,
);

function skillLabels(skill, name) {
  const first = skill.split("+")[0];
  const pair = SKILL_PAIR[skill];
  const labels = [`speclink.skill=speclink-${first}`];
  if (pair === "change") {
    return [...labels, ...(name ? [`speclink.change=${name}`] : []), "speclink.discussion="];
  }
  return [
    ...labels,
    "speclink.change=",
    `speclink.discussion=${pair === "discussion" && name ? name : ""}`,
  ];
}

function promptLabels(prompt) {
  if (typeof prompt !== "string" || !prompt) return null;
  const match = prompt
    .trimStart()
    .match(
      /^(?:\/skill:|\/|\$)speclink-([a-z-]+)(?:\s+\+\s+(?:\/skill:|\/|\$)speclink-[a-z-]+)?(?:\s+(?:--from-discussion\s+)?(\S+))?/,
    );
  if (!match || !Object.hasOwn(SKILL_PAIR, match[1])) return null;
  const [, skill, name] = match;
  const isChange = SKILL_PAIR[skill] === "change";
  const resolved = isChange && !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(name ?? "") ? undefined : name;
  return skillLabels(skill, resolved);
}

function expandAlias(text) {
  const alias = text.match(
    new RegExp(String.raw`(?:^|[;&|(\s])(\w+)=${BIN}(?=[\s;&|)]|$)`, "m"),
  )?.[1];
  return alias
    ? text.replace(new RegExp(String.raw`["']?\$\{?${alias}\}?["']?(?=\s)`, "g"), "speclink")
    : text;
}

function commandLabels(command) {
  if (typeof command !== "string" || !command) return null;
  const text = expandAlias(command.split("<<")[0]);
  const change = CHANGE_COMMANDS.map((p) => text.match(p)?.[1]).find(Boolean);
  if (change) {
    return [`speclink.change=${change}`, `speclink.discussion=${text.match(FROM_DISCUSSION)?.[1] ?? ""}`];
  }
  const discussion = DISCUSSION_COMMANDS.map((p) => text.match(p)?.[1]).find(Boolean);
  return discussion ? ["speclink.change=", `speclink.discussion=${discussion}`] : null;
}

function hookLabels(payload) {
  if (!payload || typeof payload !== "object") return null;
  const event = payload.hookEventName ?? payload.hook_event_name;
  if (event === "SessionStart") {
    return payload.source === "clear" || payload.source === "resume"
      ? ["speclink.change=", "speclink.discussion=", "speclink.skill="]
      : null;
  }
  const tool = payload.tool_name ?? payload.toolCall?.name;
  const input = payload.tool_input ?? payload.toolCall?.args;
  if (tool === "Skill") return promptLabels(`/${input?.skill ?? ""} ${input?.args ?? ""}`);
  if (tool === "Bash") return commandLabels(input?.command ?? "");
  if (tool === "run_command") return commandLabels(input?.CommandLine ?? input?.commandLine ?? "");
  if (!tool && (event === "UserPromptSubmit" || event === "PreInvocation" || !event)) {
    return promptLabels(payload.lastUserInput ?? payload.prompt ?? "");
  }
  return null;
}

function safeId(raw) {
  const cleaned = String(raw || "").replace(/[^A-Za-z0-9._-]/g, "_").slice(0, 120);
  return cleaned || null;
}

function writeFocusState(convId, labels) {
  const id = safeId(convId);
  if (!id || !Array.isArray(labels) || labels.length === 0) return;
  const dir = path.join(tmpdir(), "speclink-agy-focus");
  mkdirSync(dir, { recursive: true });
  const file = path.join(dir, `${id}.json`);
  let prev = {};
  try {
    prev = JSON.parse(readFileSync(file, "utf8"));
  } catch {
    prev = {};
  }
  const next = { ...prev, updatedAt: new Date().toISOString() };
  for (const item of labels) {
    const idx = item.indexOf("=");
    if (idx <= 0) continue;
    const key = item.slice(0, idx);
    const val = item.slice(idx + 1);
    if (key === "speclink.skill") next.skill = val || null;
    if (key === "speclink.change") next.change = val || null;
    if (key === "speclink.discussion") next.discussion = val || null;
  }
  const tmp = path.join(dir, `.${id}.${process.pid}.tmp`);
  writeFileSync(tmp, JSON.stringify(next));
  renameSync(tmp, file);
}

try {
  const raw = readFileSync(0, "utf8").trim();
  const payload = raw ? JSON.parse(raw) : null;
  const labels = hookLabels(payload);
  if (labels) {
    writeFocusState(payload?.conversationId ?? payload?.session_id, labels);
    const terminal = process.env.HC_TERMINAL_ID;
    if (terminal) {
      execFileSync("hc", ["label", terminal, ...labels], { stdio: "ignore", timeout: 3000 });
    }
  }
} catch {
  // Never interfere with Antigravity prompt or tool execution.
}

process.stdout.write("{}\n");
