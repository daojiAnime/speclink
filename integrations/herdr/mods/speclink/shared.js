// 面板和技能跳窗共用的 Herdr 小工具。
import { exec, focused, hc, panes, run, setShared, shared, spaces } from "herdr";
import { tr } from "./i18n.js";
import {
  agyPluginFolders,
  hasSpeclinkSkills,
  inside,
  parseConfigTools,
  parseStations,
  pluginFolders,
  resolveGitRoot,
  skillEntries,
  skillFolders,
  stampedPaths,
} from "./model.js";

const DEFAULT_AGENT_KEY = "speclink.defaultAgent";

// 读取持久化的默认 agent（优先会话内共享值，其次 localStorage）；null 表示自动模式。
export function loadPreferredAgent() {
  const fromShared = shared("speclink.agent");
  if (fromShared) return fromShared;
  try {
    const saved = localStorage.getItem(DEFAULT_AGENT_KEY) || null;
    if (saved) setShared("speclink.agent", saved);
    return saved;
  } catch {
    return null;
  }
}

// 保存默认 agent：写入 localStorage 并同步到 shared，传 null 则切回自动模式。
export function savePreferredAgent(key) {
  const next = key || null;
  try {
    if (next) localStorage.setItem(DEFAULT_AGENT_KEY, next);
    else localStorage.removeItem(DEFAULT_AGENT_KEY);
  } catch {
    // 某些检查环境无 localStorage 时忽略持久化。
  }
  setShared("speclink.agent", next);
}

// 读取项目 .speclink.yaml 中已配置的内置工具（转为 Herdr agent key 列表）。
export async function readProjectTools(root, on) {
  if (!root) return [];
  const r = await exec("sh", ["-c", "cat .speclink.yaml 2>/dev/null"], { cwd: root, on }).catch(() => null);
  return r?.code === 0 ? parseConfigTools(r.stdout) : [];
}

// 读取变更的单个产物正文（proposal / tasks / design），用于面板内建详情预览。
export async function readChangeArtifact(change, artifact, cwd, on) {
  const r = await exec("speclink", ["artifact", "cat", artifact, "--change", change, "--no-color"], { cwd, on }).catch((e) => ({
    code: 1,
    stdout: "",
    stderr: String(e.message ?? e),
  }));
  if (r.code !== 0) return { ok: false, error: firstLine(r.stderr) || tr("未找到 {0} 文档", artifact) };
  return { ok: true, text: r.stdout };
}

export const firstLine = (text) => text.trim().split("\n")[0];

// 目前 space 所在的資料夾：焦點所在的終端機，否則該 space 第一個終端機。spaces() 的 folder
// 只是開新終端機的起點，不一定是它現在的位置。
export function location() {
  const space = spaces().find((s) => s.active);
  const f = focused();
  const ps = panes();
  const pane = f?.pane ? ps.find((p) => p.pane === f.pane) : null;
  const tabs = new Set(space?.tabs.map((t) => t.id));
  const first = ps.find((p) => tabs.has(p.tab) && p.cwd)?.cwd;
  return {
    cwd: pane?.cwd || f?.cwd || first || space?.folder || null,
    on: space?.machine ?? null,
    branch: pane?.branch ?? space?.branch ?? null,
  };
}

// 主文件夹：在 worktree 里也要回到主 checkout，变更清单才完整；兼容 git submodule（common dir 不以 /.git 结尾）。
export async function projectRoot(cwd, on) {
  const r = await exec("git", ["rev-parse", "--path-format=absolute", "--git-common-dir"], { cwd, on });
  if (r.code !== 0) return cwd;
  const commonDir = r.stdout.trim();
  const direct = resolveGitRoot(commonDir);
  if (direct) return direct;
  const wt = await exec("git", ["config", "--file", `${commonDir}/config`, "--get", "core.worktree"], { cwd, on }).catch(() => null);
  const resolved = wt?.code === 0 ? resolveGitRoot(commonDir, wt.stdout.trim()) : null;
  if (resolved) return resolved;
  const top = await exec("git", ["rev-parse", "--show-toplevel"], { cwd, on }).catch(() => null);
  return top?.code === 0 ? top.stdout.trim() : cwd;
}

// 新分頁開在已經待在這個資料夾的 space，目前的 space 優先。
export function spaceFor(folder) {
  const all = spaces();
  const ps = panes();
  const holds = (s) => ps.some((p) => s.tabs.some((t) => t.id === p.tab) && inside(p.cwd, folder));
  return all.find((s) => s.active && holds(s)) ?? all.find(holds);
}

// 跑 speclink 动词并读取其 JSON。失败时抛出 CLI 的第一行信息。
export async function speclinkJson(verb, cwd, on) {
  const r = await exec("speclink", [...verb, "--json"], { cwd, on });
  if (r.code !== 0) throw new Error(firstLine(r.stderr) || tr("speclink {0} 退出码 {1}", verb.join(" "), r.code));
  return JSON.parse(r.stdout);
}

// 進行中的變更還剩幾個程式任務，手動任務（[M]）不算。list 和 plan 沒有這個數字，要問 instructions。
// 有 worktree 的要到 worktree 裡問，主資料夾的副本不會更新。查不到的不放進結果。
export async function codeRemaining(list, plan, root, on) {
  const folders = new Map(list.changes.map((c) => [c.name, c.worktree?.path ?? root]));
  const names = plan.changes.filter((c) => c.stage === "in-progress").map((c) => c.name);
  const answers = await Promise.all(
    names.map((name) =>
      speclinkJson(["instructions", "apply", "--change", name], folders.get(name) ?? root, on).catch(() => null),
    ),
  );
  return Object.fromEntries(names.flatMap((name, i) => (answers[i]?.progress ? [[name, answers[i].progress.codeRemaining]] : [])));
}

// 這個變更的 .meta.yaml，加上開著的工單。
const READ_STATIONS = `for d in speclink openspec; do
  c="$d/changes/$1"
  [ -d "$c" ] || continue
  cat "$c/.meta.yaml" 2>/dev/null; echo
  for s in review verify; do [ -e "$c/$s.md" ] && echo "open_ticket: $s"; done
  exit 0
done`;

// 檔案的指紋，規則和引擎的 content_fingerprint_bytes 相同：UTF-8 文字先把 CRLF 換成 LF，其他檔案看原始位元組。
// 不見的檔案不輸出。
const FINGERPRINT = String.raw`for my $f (@ARGV) { open(my $h, "<:raw", $f) or next; my $b = <$h> // ""; my $u = $b; $b =~ s/\r\n/\n/g if utf8::decode($u); print sha256_hex($b), "  $f\n" }`;

async function fingerprints(paths, cwd, on) {
  const r = await exec("perl", ["-MDigest::SHA=sha256_hex", "-0777", "-e", FINGERPRINT, "--", ...paths], { cwd, on }).catch(() => null);
  if (r?.code !== 0) return null;
  return Object.fromEntries(r.stdout.split("\n").filter(Boolean).map((line) => [line.slice(66), line.slice(0, 64)]));
}

// 品質關卡的章和工單。CLI 沒有給，只好讀檔；有 worktree 的要到 worktree 裡讀，主資料夾的副本不會更新。
// 提案中的變更還沒有章。讀不到時沒有章，封存時引擎還會再查一次。
export async function stationFacts(list, plan, root, on) {
  const folders = new Map(list.changes.map((c) => [c.name, c.worktree?.path ?? root]));
  const names = plan.changes.filter((c) => c.stage !== "proposed").map((c) => c.name);
  const facts = await Promise.all(
    names.map(async (name) => {
      const cwd = folders.get(name) ?? root;
      const r = await exec("sh", ["-c", READ_STATIONS, "sh", name], { cwd, on }).catch(() => null);
      const stations = parseStations(r?.stdout ?? "");
      const paths = stampedPaths(stations);
      return [name, { ...stations, hashes: paths.length > 0 ? await fingerprints(paths, cwd, on) : null }];
    }),
  );
  return Object.fromEntries(facts);
}

// hc run 回來時分頁可能還沒出現，等它出現才能改名。focus 是 true 的分頁，出現後畫面切過去。
const nameTabs = (waiting, cx, tries = 20) =>
  new Promise((done) => {
    const left = waiting.filter(({ address, name, focus }) => {
      const pane = panes().find((p) => p.terminal === address);
      if (pane) {
        run("tab.rename", { id: pane.tab, name });
        if (focus) run("pane.focus", { terminal: address });
      }
      return !pane;
    });
    if (left.length === 0 || tries === 0) return done();
    cx.timer.after(150, (cx) => nameTabs(left, cx, tries - 1).then(done));
  });

// 依序開新分頁並取名。中途失敗就停下來，已經開好的分頁留著，回傳失敗的結果；全部成功時回傳 null。
export async function openTabs(plans, cx) {
  const opened = [];
  let failed = null;
  for (const plan of plans) {
    const r = await hc(plan.hc);
    if (r.code !== 0) {
      failed = r;
      break;
    }
    opened.push({ address: JSON.parse(r.stdout).address, name: plan.tabName, focus: plan.focus });
  }
  await nameTabs(opened, cx);
  return failed;
}

// Herdr 沒有「只貼上不按 Enter」的指令，所以暫時當一個看不見的檢視者：hc attach 要真的終端機才肯跑，
// script 給它一個。送出括號貼上（程式會當成貼上，不是打字），再按 ctrl+\ 離開。
// 大小用終端機現在的大小，它才不會跳動。ctrl+\ 要和貼上分開送，同一次讀到時 hc 不會當成離開；
// alarm 確保失敗時不會一直掛著。script 的用法是 macOS 的，其他系統會失敗並回報。
const PASTE = `{ sleep 0.2; printf '\\033[200~%s\\033[201~' "$4"; sleep 0.15; printf '\\034'; } | perl -e 'alarm 3; exec @ARGV' script -q /dev/null sh -c 'stty rows "$2" cols "$3"; exec hc attach "$1"' sh "$1" "$2" "$3" >/dev/null 2>&1`;

// 照 placePlan 或 attachPlan 的結果貼進輸入框，再把焦點切過去。失敗時回傳結果，sendError 讀得懂。
export async function placeInInput(plan) {
  const { address } = plan.agent;
  const got = await hc(["get", address, "--json"]);
  if (got.code !== 0) return got;
  const { rows, cols } = JSON.parse(got.stdout);
  const r = await exec("sh", ["-c", PASTE, "sh", address, String(rows), String(cols), plan.text]);
  if (r.code !== 0) return { ...r, stderr: tr("无法粘贴到 {0}（退出码 {1}）", address, r.code) };
  // 附加名称不算开始做，所以只有技能指令带标签。
  if (plan.labels) await hc(["label", address, ...plan.labels]);
  run("pane.focus", { terminal: address });
  return null;
}

export function sendError(result) {
  try {
    return JSON.parse(result.stderr).error.message;
  } catch {
    return firstLine(result.stderr) || tr("hc 退出码 {0}", result.code);
  }
}

// 讀資料夾裡每個技能 SKILL.md 開頭的欄位。多行的值接在同一欄後面。位元組原樣輸出，JSON 還是 UTF-8。
// 參數一個資料夾一個：來源、名稱前綴、資料夾，用 tab 隔開。
const READ_SKILLS = String.raw`
my @out;
for my $spec (@ARGV) {
  my ($source, $prefix, $dir) = split /\t/, $spec, 3;
  (my $abs = $dir) =~ s{^~(?=/)}{$ENV{HOME}};
  for my $file (sort glob(qq{"$abs"/*/SKILL.md})) {
    open(my $fh, "<", $file) or next;
    my $first = <$fh>;
    next unless defined $first && $first =~ /^---\s*$/;
    my (%f, $key);
    while (my $line = <$fh>) {
      last if $line =~ /^---\s*$/;
      if ($line =~ /^([A-Za-z-]+):\s*(.*?)\s*$/) { $key = $1; $f{$key} = $2; }
      elsif (defined $key && $line =~ /^\s+(\S.*?)\s*$/) { $f{$key} .= " $1"; }
    }
    my ($base) = $file =~ m{([^/]+)/SKILL\.md$};
    push @out, { source => $source, name => $prefix . ($f{name} // $base), description => $f{description} // "", invocable => $f{"user-invocable"} // "", path => "$dir/$base" };
  }
}
print JSON::PP->new->encode(\@out);
`;

async function readJson(path, options) {
  const r = await exec("sh", ["-c", `cat "${path}"`], options).catch(() => null);
  try {
    return JSON.parse(r?.stdout ?? "");
  } catch {
    return null;
  }
}

// 這個 agent 的技能：我的技能頁的清單，和這個專案有沒有它用的 Speclink 技能。
export async function readSkills(key, root, on) {
  const options = { cwd: root, on };
  const folders = skillFolders(key);
  if (key === "claude") {
    const paths = ["$HOME/.claude/settings.json", ".claude/settings.json", ".claude/settings.local.json"];
    const [installed, ...settings] = await Promise.all(["$HOME/.claude/plugins/installed_plugins.json", ...paths].map((path) => readJson(path, options)));
    folders.push(...pluginFolders(installed, settings, root));
  } else if (key === "antigravity") {
    const [pluginOut, wsOut] = await Promise.all([
      exec("agy", ["plugin", "list", "--json"], options).catch(() => null),
      exec("sh", ["-c", 'for d in .agents/plugins/*; do [ -d "$d/skills" ] && basename "$d"; done'], options).catch(() => null),
    ]);
    let installed = null;
    try {
      if (pluginOut?.code === 0) installed = JSON.parse(pluginOut.stdout);
    } catch {
      installed = null;
    }
    const workspacePlugins =
      wsOut?.code === 0
        ? wsOut.stdout
            .split("\n")
            .map((s) => s.trim())
            .filter(Boolean)
            .map((name) => ({ name, enabled: true, workspace: true, path: `.agents/plugins/${name}` }))
        : [];
    folders.push(...agyPluginFolders(installed, workspacePlugins));
  }
  const specs = folders.map(({ source, prefix, dir }) => `${source}\t${prefix}\t${dir}`);
  const r = await exec("perl", ["-MJSON::PP", "-e", READ_SKILLS, "--", ...specs], options);
  if (r.code !== 0) throw new Error(firstLine(r.stderr) || tr("perl 退出码 {0}", r.code));
  const found = JSON.parse(r.stdout);
  return { skills: skillEntries(found), hasSpeclink: hasSpeclinkSkills(found) };
}
