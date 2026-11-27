// oh-my-pi (omp) 的 extension，做的事和 label-skill.mjs 相同：看送出的訊息和跑的 speclink 指令，幫這個終端機貼標籤。
// 規則在 model.js 的 hookLabels。設定方式見 integrations/herdr/README.md。不在 Herdr 裡或貼不上時什麼都不做。
import { execFileSync } from "node:child_process";
import { hookLabels } from "./model.js";

const terminal = process.env.HC_TERMINAL_ID;

function label(event: Parameters<typeof hookLabels>[0]) {
  try {
    const labels = hookLabels(event);
    if (labels) execFileSync("hc", ["label", terminal as string, ...labels], { stdio: "ignore", timeout: 3000 });
  } catch {
    // 貼不上標籤不能擋住使用者送出訊息。
  }
}

export default function (pi: any) {
  if (!terminal) return;
  pi.on("input", (event: any) => {
    label({ hook_event_name: "UserPromptSubmit", prompt: event.text });
  });
  pi.on("tool_call", (event: any) => {
    if (event.toolName === "bash") label({ tool_name: "Bash", tool_input: { command: event.input?.command } });
  });
  // oh-my-pi 的 session_start 事件不帶 reason 欄位；切新工作階段或恢復歷史都在 session_switch 觸發。
  pi.on("session_switch", (event: any) => {
    if (event.reason === "new" || event.reason === "resume") label({ hook_event_name: "SessionStart", source: "clear" });
  });
}
