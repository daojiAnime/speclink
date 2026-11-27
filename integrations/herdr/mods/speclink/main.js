// Speclink 面板：目前 space 的專案有哪些變更與討論、各自做到哪裡，以及按一下把技能交給對的
// agent。資料只來自 speclink CLI 的 --json；判斷邏輯都在 model.js。
import { View, div, image, svg } from "gpui-kit";
import { InputState, h_flex, v_flex } from "gpui-base";
import {
  agents,
  checking,
  closeOverlay,
  exec,
  focused,
  hc,
  knownAgents,
  notify,
  panes,
  props,
  run,
  shared,
  terminals,
  theme,
  watchFolder,
  window,
} from "herdr";
import {
  APP_ID,
  attachPlan,
  buildBoard,
  buildRoadmap,
  canOpenProject,
  changeActions,
  changeAgents,
  commandText,
  defaultAgent,
  deliveryHint,
  discussionActions,
  discussionAgents,
  unrecordedDiscussions,
  isUninitialized,
  joinAgents,
  lastOpenError,
  logoOpenAttempts,
  newTabPlan,
  newTabRun,
  rowChoices,
  speclinkUrl,
  startChoices,
  stationViews,
  waitingText,
  waveToStart,
} from "./model.js";
import ColumnSync from "./column-sync.js";
import { tr, watchLang } from "./i18n.js";
import SkillPicker from "./skill-picker.js";
import { codeRemaining, firstLine, location, openTabs, placeInInput, projectRoot, sendError, spaceFor, speclinkJson, stationFacts } from "./shared.js";

// 這個 mod 的其他畫面。speclink-new 開跳窗、dock-panel.mjs 放浮動區塊時，用 props 的 view 指定。
const VIEWS = { "skill-picker": SkillPicker, "column-sync": ColumnSync };
const ICONS = "mods/speclink/icons";
// 溢出摺疊的門檻與桌面 app 的系統匣面板相同。
const OVERFLOW_LIMIT = 5;
const STAGES = [
  { id: "proposed", label: "提案中", tone: "ansi3" },
  { id: "in-progress", label: "进行中", tone: "ansi4" },
  { id: "ready", label: "已就绪", tone: "ansi2" },
];
const AGENT_STATE = { working: "执行中", done: "已完成", idle: "空闲" };
// 等你的 agent 写出在等什么（见 waitingText）。
const stateText = (a) => (a.view === "blocked" ? waitingText(a) : (AGENT_STATE[a.view] ?? a.view));
// 图标和文字与桌面 app 的看板卡片相同：形状分站别（徽章是 review，盾牌是 verify），颜色分状态。
const STATION_BADGE = {
  review: {
    stamped: ["badge-check", "已审查"],
    stale: ["badge-alert", "已审查·之后有变动"],
    open: ["stamp", "审查中"],
    running: ["stamp", "审查中"],
  },
  verify: {
    stamped: ["shield-check", "已验证"],
    stale: ["shield-alert", "已验证·之后有变动"],
    open: ["shield", "验证中"],
    running: ["shield", "验证中"],
  },
};
const STATION_TONE = { stamped: "ansi5", stale: "ansi3", open: "ansi4", running: "ansi4" };

const icon = (name, size, color) => svg(`${ICONS}/${name}.svg`).size(size).flex_none().text_color(color);
const baseName = (path) => path.split("/").filter(Boolean).pop() ?? path;

// program.icon 是相對於 Herdr 設定資料夾的路徑：tint 表示單色圖示，其餘是圖片。
function programIcon(program, size, color) {
  if (!program?.icon) return div();
  return program.tint
    ? svg(program.icon).size(size).flex_none().text_color(color)
    : image(program.icon).w(size).h(size).flex_none();
}

export default class SpeclinkPanel extends View {
  init(_props, cx) {
    if (!checking()) watchLang(exec, cx, { div, InputState });
    const Other = VIEWS[props()?.view];
    if (Other) {
      this.other = new Other();
      this.other.init(cx);
      return;
    }
    this.place = location();
    this.root = null;
    this.board = null;
    this.error = null;
    this.uninitialized = false;
    this.known = null;
    this.copied = null;
    this.attaching = new Map();
    this.choosing = null;
    this.expanded = null;
    this.more = new Set();
    this.sending = new Set();
    this.starting = new Set();
    this.worktreePolicy = false;
    this.viewMode = "board";
    this.desktopApp = false;
    this.collapsed = new Set(JSON.parse(localStorage.getItem("collapsed") ?? "[]"));
    this.promotedOpen = localStorage.getItem("promotedOpen") === "true";
    if (!checking()) {
      this.loadDesktopApp(cx);
      this.reload(cx);
      this.loadKnown(cx);
      // worktree 裡的任務進度不在主資料夾的監看範圍內，要靠計時重抓。
      cx.timer.every(10000, (cx) => {
        this.follow(cx);
        this.reload(cx);
      });
    }
  }

  // 静默检查本机是否安装了 Speclink 桌面应用，未安装时点击行/Logo/路线图不触发 -10814 错误弹窗。
  loadDesktopApp(cx) {
    if (checking()) return;
    cx.spawn(async (cx) => {
      const r = await exec("open", ["-Ra", APP_ID]).catch(() => ({ code: 1 }));
      const hasApp = r?.code === 0;
      if (hasApp !== this.desktopApp) {
        this.desktopApp = hasApp;
        cx.notify();
      }
    });
  }

  toggleRoadmap(cx) {
    this.viewMode = this.viewMode === "roadmap" ? "board" : "roadmap";
    cx.notify();
  }

  openRoadmapView(cx) {
    this.viewMode = "roadmap";
    cx.notify();
  }

  // 焦點或資料夾變了就改跟新的專案。render 會隨 Herdr 的即時資料重跑，所以平常立刻跟上；
  // 視窗沒有畫面更新時（被其他視窗完全蓋住）render 不會跑，由計時補上。
  follow(cx) {
    if (checking()) return;
    const { cwd, on } = this.place;
    this.place = location();
    if (this.place.cwd === cwd && this.place.on === on) return;
    if (this.place.on !== on) this.loadKnown(cx);
    this.reload(cx);
  }

  // 檢查失敗時 knownAgents 會丟錯，Herdr 也不記住這次結果，所以過 10 秒再查；查到之前面板顯示「還在確認」。
  loadKnown(cx) {
    if (checking()) return;
    this.retryKnown?.cancel();
    cx.spawn(async (cx) => {
      this.known = await knownAgents({ on: this.place.on }).catch(() => null);
      if (!this.known) this.retryKnown = cx.timer.after(10000, (cx) => this.loadKnown(cx));
      cx.notify();
    });
  }

  reload(cx) {
    if (checking()) return;
    // 視窗看不到時（縮小、被完全蓋住、在別的桌面），計時器和資料夾監看還是會叫這裡，這時不問 CLI。
    // 記成 stale，回到畫面時 render 補抓；render 沒跑的話，下一次計時也會抓。
    this.stale = window()?.visible === false;
    if (this.stale) return;
    if (this.loading) {
      this.again = true;
      return;
    }
    const { cwd, on } = this.place;
    if (!cwd) return;
    this.loading = true;
    cx.spawn(async (cx) => {
      try {
        const root = await projectRoot(cwd, on);
        if (root !== this.root) {
          this.root = root;
          this.board = null;
          this.uninitialized = false;
          this.watch(root, on, cx);
        }
        const read = (...verb) => speclinkJson(verb, root, on);
        const [list, plan, discussions, archived, config] = await Promise.all([
          read("list"),
          read("plan"),
          read("discuss", "list"),
          read("discuss", "list", "--archived"),
          read("workflow-config", "show"),
        ]);
        const [remaining, facts] = await Promise.all([codeRemaining(list, plan, root, on), stationFacts(list, plan, root, on)]);
        // 封存的討論不上看板，只用來認出做完討論的 agent（見 unrecordedDiscussions）。
        this.board = {
          ...buildBoard(list, plan, discussions, remaining, facts),
          archived: archived.discussions.map((d) => d.slug),
        };
        this.worktreePolicy = config.worktree === true;
        this.error = null;
        this.uninitialized = false;
      } catch (e) {
        const message = String(e.message ?? e);
        this.uninitialized = isUninitialized(message);
        this.error = this.uninitialized ? null : message;
        // 切到沒有 Speclink 的分支時，舊的變更清單不能留著。
        if (this.uninitialized) this.board = null;
      }
      this.loading = false;
      cx.notify();
      if (this.again) {
        this.again = false;
        this.reload(cx);
      }
    });
  }

  // 規格資料夾有動靜就重抓。內容問 CLI，只有章 CLI 不給（見 stationFacts）。watchFolder 只看得到本機。
  // 換專案時取消舊專案的監看：Herdr 同時最多只等 64 個資料夾，不取消的話，舊的會一直佔著名額。
  watch(root, on, cx) {
    this.watching?.cancel();
    this.watching = null;
    if (on) return;
    cx.spawn(async (cx) => {
      const cfg = await exec("sh", ["-c", "sed -n 's/^spec_dir:[[:space:]]*//p' .speclink.yaml 2>/dev/null | head -n 1"], {
        cwd: root,
      }).catch(() => null);
      const customDir = cfg?.stdout.trim().replace(/^["']|["']$/g, "");
      const candidates = customDir ? [customDir] : ["speclink", "openspec"];
      const found = await exec("ls", ["-d", ...candidates], { cwd: root }).catch(() => null);
      const dir = found?.stdout.trim().split("\n")[0];
      if (!dir) return;
      try {
        while (this.root === root) {
          this.watching = watchFolder(`${root}/${dir}`);
          await this.watching;
          if (this.root === root) this.reload(cx);
        }
      } catch {
        // 監看失敗時還有計時重抓。
      }
    });
  }

  context() {
    const f = focused();
    return {
      root: this.root,
      agents: joinAgents(agents(), terminals(), panes()).map((a) =>
        this.sending.has(a.address) ? { ...a, status: "sending" } : a,
      ),
      focus: { tab: f?.tab ?? null, terminal: f?.terminal ?? null },
      space: f?.space ?? null,
      on: this.place.on,
      board: this.board,
      known: this.known,
      preferred: shared("speclink.agent") ?? null,
      worktreePolicy: this.worktreePolicy,
      starting: this.starting,
    };
  }

  deliver(delivery, cx) {
    this.expanded = null;
    if (delivery.kind === "send") {
      const { address } = delivery.agent;
      this.sending.add(address);
      cx.notify();
      cx.spawn(async (cx) => {
        const r = await hc(delivery.hc);
        this.sending.delete(address);
        if (r.code === 0) {
          // 面板才知道这个 agent 接下来在做什么。
          hc(["label", address, ...delivery.labels]);
          run("pane.focus", { terminal: address });
        } else notify({ title: tr("技能未发送"), body: sendError(r), level: "warning" });
        cx.notify();
      });
    } else if (delivery.kind === "new") {
      // 和開工一樣用 hc run 開分頁，標籤在建立時就貼好。技能鈕是「交給某個 agent」，所以畫面切到新分頁。
      const space = spaceFor(delivery.folder)?.id ?? focused()?.space;
      const live = joinAgents(agents(), terminals(), panes());
      const plan = { hc: newTabRun(delivery, { space, on: this.place.on, agents: live }), tabName: delivery.tabName, focus: true };
      if (delivery.change) this.markStarting(delivery.change, cx);
      cx.spawn(async (cx) => {
        const failed = await openTabs([plan], cx);
        if (failed) notify({ title: tr("开工失败"), body: sendError(failed), level: "warning" });
      });
    }
  }

  // 開工和技能跳窗的開新分頁相同：開在目前的工作區，畫面留在原分頁。
  startTabs(plans, cx) {
    this.choosing = null;
    for (const plan of plans) this.markStarting(plan.change, cx);
    cx.spawn(async (cx) => {
      const failed = await openTabs(plans, cx);
      if (failed) notify({ title: tr("开工失败"), body: sendError(failed), level: "warning" });
    });
  }

  // agent 開好並回報狀態要幾秒，這段時間列上還看不到它，再按一次開工會多開一個。
  markStarting(change, cx) {
    this.starting.add(change);
    cx.notify();
    cx.timer.after(15000, (cx) => {
      this.starting.delete(change);
      cx.notify();
    });
  }

  // 叫 Speclink 桌面 app。open 只有 macOS 有；沒裝 app 時 open 也會失敗。
  openInApp(args, cx) {
    this.openFirstInApp([args], cx);
  }

  // 依次试 open，成功就停；全部失败才弹通知，正文是最后一次有内容的错误。
  openFirstInApp(attempts, cx) {
    cx.spawn(async () => {
      const error = await lastOpenError(attempts, exec);
      if (error !== null) notify({ title: tr("无法打开 Speclink"), body: firstLine(error), level: "warning" });
    });
  }

  // 判斷這個 space 的專案能不能用網址開時看的兩件事。
  projectTarget() {
    return { hasBoard: Boolean(this.board), on: this.place.on };
  }

  copy(text, cx) {
    cx.write_to_clipboard(text);
    this.copied = text;
    cx.notify();
    cx.timer.after(1200, (cx) => {
      if (this.copied === text) this.copied = null;
      cx.notify();
    });
  }

  toggleSection(id, cx) {
    if (id === "promoted") {
      this.promotedOpen = !this.promotedOpen;
      localStorage.setItem("promotedOpen", String(this.promotedOpen));
    } else {
      if (this.collapsed.has(id)) this.collapsed.delete(id);
      else this.collapsed.add(id);
      localStorage.setItem("collapsed", JSON.stringify([...this.collapsed]));
    }
    cx.notify();
  }

  copyButton(id, text, label, t) {
    return div()
      .id(`copy-${id}`)
      .flex_none()
      .p(3)
      .rounded(4)
      .cursor_pointer()
      .hover((s) => s.bg(t.hover))
      .tooltip(label)
      .on_click((_e, cx) => {
        cx.stop_propagation();
        this.copy(text, cx);
      })
      .child(icon(this.copied === text ? "check" : "copy", 12, t.muted));
  }

  chip({ id, label, ready, primary, tip, click }, t) {
    return h_flex()
      .id(id)
      .h(22)
      .px(7)
      .items_center()
      .rounded(5)
      .text_size(11.5)
      .when(primary, (el) => el.border_2().border_color(t.accent).font_semibold())
      .when(!primary, (el) => el.border_1().border_color(t.input))
      .when(ready, (el) => el.cursor_pointer().hover((s) => s.bg(t.hover)))
      .when(!ready, (el) => el.opacity(0.45))
      // 灰掉的鈕也要擋下點擊，否則會變成點到列本身。
      .on_click((_e, cx) => {
        cx.stop_propagation();
        if (ready) click(cx);
      })
      .when(Boolean(tip), (el) => el.tooltip(tip))
      .child(label);
  }

  skillButton(id, action, t) {
    const d = action.delivery;
    const tip = action.waitingFor
      ? tr("等待 {0} 先完成", action.waitingFor.join(tr("、")))
      : d.text
        ? `${deliveryHint(d)}：${d.text}`
        : deliveryHint(d);
    return this.chip(
      {
        id: `skill-${id}-${action.skill}`,
        // 写实际会发送的技能。有 worktree 时 apply 会换成 apply-with-worktree。
        label: tr(d?.skill ?? action.skill),
        ready: d && (d.kind === "send" || d.kind === "new"),
        primary: action.primary,
        tip,
        click: (cx) => this.deliver(d, cx),
      },
      t,
    );
  }

  // 开工的一种方式。
  startChoice(id, { label, plan }, t) {
    const ready = plan.kind === "tab";
    return this.chip(
      {
        id: `start-${id}-${label}`,
        label: tr(label),
        ready,
        tip: ready ? tr("在当前工作区新建标签页并运行：{0}", plan.text) : plan.reason,
        click: (cx) => this.startTabs([plan], cx),
      },
      t,
    );
  }

  // 只有一种方式就直接开工，有两种时按下去才展开选项。
  startButton(id, target, choices, t) {
    if (choices.length === 1) return this.startChoice(id, choices[0], t);
    return this.chip(
      {
        id: `start-${id}`,
        label: tr("开工"),
        ready: true,
        tip: tr("选择使用 worktree 或直接 apply"),
        click: (cx) => {
          this.choosing = target;
          cx.notify();
        },
      },
      t,
    );
  }

  // 粘贴约需 0.7 秒，所以点击后要先有反馈。粘贴中再点一次不算，否则名称会贴两次。
  attach(plan, name, cx) {
    if (this.attaching.has(name)) return;
    this.attaching.set(name, "pasting");
    cx.notify();
    cx.spawn(async (cx) => {
      const failed = await placeInInput(plan);
      if (failed) {
        this.attaching.delete(name);
        notify({ title: tr("未能放入输入框"), body: sendError(failed), level: "warning" });
      } else {
        this.attaching.set(name, "done");
        cx.timer.after(1200, (cx) => {
          this.attaching.delete(name);
          cx.notify();
        });
      }
      cx.notify();
    });
  }

  // 把名称接到当前标签页输入框，不发送。
  attachButton(id, name, ctx, t) {
    const plan = attachPlan({ name, agents: ctx.agents, focus: ctx.focus });
    const ready = plan.kind !== "none";
    const state = this.attaching.get(name);
    return div()
      .id(`attach-${id}`)
      .flex_none()
      .p(3)
      .rounded(4)
      .when(ready, (el) => el.cursor_pointer().hover((s) => s.bg(t.hover)))
      .when(!ready && !state, (el) => el.opacity(0.45))
      .tooltip(ready ? tr("附加到当前标签页输入框") : plan.reason)
      .on_click((_e, cx) => {
        cx.stop_propagation();
        if (ready) this.attach(plan, name, cx);
      })
      .child(
        state === "pasting"
          ? icon("spinner", 12, t.working).spin(900)
          : icon(state === "done" ? "check" : "message-plus", 12, t.muted),
      );
  }

  // 展开这一行其他可用的技能。没有其他技能的行不放这个按钮。
  moreButton(id, target, t) {
    const open = this.expanded === target;
    return h_flex()
      .id(`more-${id}`)
      .flex_none()
      .h(22)
      .px(5)
      .items_center()
      .rounded(5)
      .text_color(t.muted)
      .cursor_pointer()
      .when(open, (el) => el.bg(t.selected))
      .hover((s) => s.bg(t.hover))
      .tooltip(tr("其他技能"))
      .on_click((_e, cx) => {
        cx.stop_propagation();
        this.expanded = open ? null : target;
        cx.notify();
      })
      .child("⋯");
  }

  // 每行只留建议的下一步，其他的按 ⋯ 才展开。能开工的行多一个开工按钮。
  skillRow({ id, actions, start, target, hint }, t) {
    // 开工要选方式时，这一行只放选项。
    if (start?.length > 1 && this.choosing === target) {
      return h_flex()
        .gap(4)
        .items_center()
        .children(start.map((choice) => this.startChoice(id, choice, t)))
        .child(
          this.chip(
            {
              id: `start-${id}-cancel`,
              label: tr("取消"),
              ready: true,
              click: (cx) => {
                this.choosing = null;
                cx.notify();
              },
            },
            t,
          ),
        );
    }
    const { next, beside, others } = rowChoices(actions, start);
    return v_flex()
      .gap(4)
      .child(
        h_flex()
          .gap(4)
          .items_center()
          .when(Boolean(start), (el) => el.child(this.startButton(id, target, start, t)))
          .child(this.skillButton(id, next, t))
          .children(beside.map((action) => this.skillButton(id, action, t)))
          .when(others.length > 0, (el) => el.child(this.moreButton(id, target, t)))
          .when(Boolean(hint), (el) =>
            el.child(div().flex_1().min_w_0().truncate().text_size(11).text_color(t.dim).child(hint)),
          ),
      )
      .when(this.expanded === target && others.length > 0, (el) =>
        el.child(h_flex().flex_wrap().gap(4).items_center().children(others.map((action) => this.skillButton(id, action, t)))),
      );
  }

  // 和 Herdr 內建的 Agents 清單同一套：做事中是轉動的圓弧，閒置是空心圈，做完是實心點，等回答是膠囊。
  agentStatus(a, t) {
    const { view } = a;
    if (view === "working") return icon("spinner", 12, t.working).mx(1).spin(900);
    if (view === "blocked") {
      return div()
        .flex()
        .flex_none()
        .h(18)
        .px(6)
        .items_center()
        .rounded(5)
        .bg(`${t.blocked}26`)
        .text_size(10.5)
        .font_semibold()
        .text_color(t.blocked)
        .child(tr(waitingText(a)));
    }
    if (view === "idle") return div().size(7).mx(3).flex_none().rounded(4).border(1.5).border_color(t.idle);
    return div().size(7).mx(3).flex_none().rounded(4).bg(t[view] ?? t.dim);
  }

  // 目前分頁對應的那一列：左邊一條色帶和淡淡的底色。
  markCurrent(el, t) {
    return el.pl(7).border_l(3).border_color(t.accent).bg(`${t.accent}12`);
  }

  agentChip(id, a, here, t) {
    return h_flex()
      .id(`agent-${id}-${a.address}`)
      .gap(5)
      .px(5)
      .py(2)
      .items_center()
      .rounded(5)
      .text_size(11)
      .cursor_pointer()
      .hover((s) => s.bg(t.hover))
      .tooltip(tr("切换到此 agent（{0}）", tr(stateText(a))))
      .on_click((_e, cx) => {
        cx.stop_propagation();
        run("agent.focus", { terminal: a.address });
      })
      .child(programIcon(a.program, 12, t.muted))
      .child(div().flex_1().min_w_0().truncate().text_color(here ? t.foreground : t.muted).child(`${a.space_name} / ${a.tab_name}`))
      .when(here, (el) =>
        el.child(div().flex_none().px(5).rounded(4).bg(`${t.accent}26`).text_size(10).text_color(t.accent).child(tr("当前标签页"))),
      )
      .child(this.agentStatus(a, t));
  }

  // 点行本身叫 Speclink 开详情。行上的按钮都会挡下点击；未安装桌面应用或远程项目时不触发无效 URL。
  openOnClick(el, url, t) {
    if (this.place.on || !this.desktopApp) return el;
    return el
      .cursor_pointer()
      .hover((s) => s.bg(t.hover))
      .on_click((_e, cx) => this.openInApp([url], cx));
  }

  waveNumber(row, t) {
    return div()
      .id(`wave-${row.name}`)
      .flex_none()
      .text_size(11)
      .font_semibold()
      .text_color(t.accent)
      .px(3)
      .rounded(3)
      .cursor_pointer()
      .hover((s) => s.bg(t.raised))
      .tooltip(tr("在路线图查看波次"))
      .on_click((_e, cx) => {
        cx.stop_propagation();
        this.openRoadmapView(cx);
      })
      .child(String(row.wave));
  }

  // agent 正在跑的站会呼吸，和工单开着但没人在跑的站分得开。
  stationBadges(row, ctx, t) {
    return stationViews(row, ctx.agents).map(({ station, state }) => {
      const [name, label] = STATION_BADGE[station][state];
      return div()
        .id(`station-${row.name}-${station}`)
        .flex_none()
        .when(state === "running", (el) => el.loop("opacity", { from: 0.35, to: 1, period: 1400 }))
        .tooltip(tr(label))
        .child(icon(name, 13, t[STATION_TONE[state]]));
    });
  }

  changeRow(row, stage, ctx, t) {
    const actions = changeActions(row, ctx);
    const start = startChoices(row, ctx);
    const blocked = row.blockedBy.length > 0;
    const hint = blocked ? tr("等待 {0}", row.blockedBy.join(tr("、"))) : "";
    const percent = row.total > 0 ? Math.round((row.completed / row.total) * 100) : 0;
    const working = changeAgents(row, ctx.agents);
    const el = v_flex()
      .id(`change-${row.name}`)
      .px(10)
      .py(8)
      .gap(5)
      .when(working.some((a) => a.tab === ctx.focus.tab), (el) => this.markCurrent(el, t))
      .child(
        h_flex()
          .gap(6)
          .items_center()
          .when(blocked, (el) => el.opacity(0.6))
          .child(this.waveNumber(row, t))
          .child(div().flex_1().min_w_0().truncate().child(row.name))
          .children(this.stationBadges(row, ctx, t))
          .when(Boolean(row.worktree), (el) =>
            el.child(
              div()
                .id(`branch-${row.name}`)
                .flex_none()
                .tooltip(`worktree：${row.worktree?.branch}`)
                .child(icon("branch", 12, t.ansi4)),
            ),
          )
          .when(row.total > 0, (el) =>
            el.child(div().flex_none().text_size(11).text_color(t.muted).child(`${row.completed}/${row.total}`)),
          )
          .child(this.attachButton(`change-${row.name}`, row.name, ctx, t))
          .child(this.copyButton(`change-${row.name}`, row.name, tr("复制名称"), t)),
      )
      .when(row.total > 0, (el) =>
        el.child(
          div()
            .h(4)
            .rounded(2)
            .bg(t.raised)
            .overflow_hidden()
            .when(blocked, (bar) => bar.opacity(0.6))
            .child(div().h_full().rounded(2).w(`${percent}%`).bg(t[stage.tone])),
        ),
      )
      .children(working.map((a) => this.agentChip(`change-${row.name}`, a, a.tab === ctx.focus.tab, t)))
      .child(this.skillRow({ id: `change-${row.name}`, actions, start, target: row.name, hint }, t));
    return this.openOnClick(el, speclinkUrl("change", this.root, row.name), t);
  }

  discussionRow(d, ctx, t) {
    const actions = discussionActions(d, ctx);
    const hint = d.cuts.length > 0 ? tr("还有 {0} 刀未转出", d.cuts.length) : "";
    const working = discussionAgents(d, ctx.agents);
    const el = v_flex()
      .id(`discussion-${d.slug}`)
      .px(10)
      .py(8)
      .gap(5)
      .when(working.some((a) => a.tab === ctx.focus.tab), (el) => this.markCurrent(el, t))
      .child(
        h_flex()
          .gap(6)
          .items_center()
          .child(
            v_flex()
              .flex_1()
              .min_w_0()
              .child(div().truncate().child(d.slug))
              .child(div().truncate().text_size(11).text_color(t.muted).child(d.topic)),
          )
          .child(this.attachButton(`discussion-${d.slug}`, d.slug, ctx, t))
          .child(this.copyButton(`discussion-${d.slug}`, d.slug, tr("复制 slug"), t)),
      )
      .children(working.map((a) => this.agentChip(`discussion-${d.slug}`, a, a.tab === ctx.focus.tab, t)))
      .child(this.skillRow({ id: `discussion-${d.slug}`, actions, target: d.slug, hint }, t));
    return this.openOnClick(el, speclinkUrl("discussion", this.root, d.slug), t);
  }

  // 还没记录的讨论。名称用 agent 的标题；记录建立后，agent 会移到正式那一行。
  unrecordedRow(a, ctx, t) {
    const here = a.tab === ctx.focus.tab;
    return v_flex()
      .id(`discussion-unrecorded-${a.address}`)
      .px(10)
      .py(8)
      .gap(5)
      .when(here, (el) => this.markCurrent(el, t))
      .child(
        v_flex()
          .min_w_0()
          .child(div().truncate().child(a.title || a.tab_name))
          .child(div().truncate().text_size(11).text_color(t.muted).child(tr("讨论中，尚未记录"))),
      )
      .child(this.agentChip(`discussion-unrecorded-${a.address}`, a, here, t));
  }

  // 超过门槛的行收进「还有 N 个…」。
  overflow(id, rows, t) {
    if (rows.length <= OVERFLOW_LIMIT) return rows;
    const open = this.more.has(id);
    return [
      ...(open ? rows : rows.slice(0, OVERFLOW_LIMIT)),
      div()
        .id(`more-${id}`)
        .px(10)
        .py(6)
        .text_size(12)
        .text_color(t.muted)
        .cursor_pointer()
        .hover((s) => s.bg(t.hover))
        .on_click((_e, cx) => {
          if (open) this.more.delete(id);
          else this.more.add(id);
          cx.notify();
        })
        .child(open ? tr("收起") : tr("还有 {0} 个…", rows.length - OVERFLOW_LIMIT)),
    ];
  }

  card({ id, tone, label, rows, open }, t) {
    return v_flex()
      .flex_none()
      .rounded(10)
      .border_1()
      .border_color(t.border)
      .bg(t.surface)
      .overflow_hidden()
      .child(
        h_flex()
          .id(`card-${id}`)
          .h(30)
          .px(10)
          .gap(6)
          .items_center()
          .cursor_pointer()
          .hover((s) => s.bg(t.hover))
          .when(open && rows.length > 0, (el) => el.border_b_1().border_color(t.border))
          .on_click((_e, cx) => this.toggleSection(id, cx))
          .child(div().w(8).h(8).flex_none().rounded_full().bg(tone))
          .child(div().font_semibold().text_size(12).child(tr(label)))
          .child(div().px(6).rounded_full().bg(t.raised).text_size(11).text_color(t.muted).child(String(rows.length)))
          .child(div().flex_1())
          .child(icon(open ? "chevron-down" : "chevron-right", 13, t.muted)),
      )
      .when(open, (el) => el.children(this.roundLast(this.overflow(id, rows, t))));
  }

  // 卡片的 overflow_hidden 只裁矩形。最後一列要自己圓角，底色才不會從卡片的圓角露出來。
  roundLast(rows) {
    return rows.map((row, i) => (i === rows.length - 1 ? row.rounded_b(9) : row));
  }

  // 整波開工不再多問一次：同一列已經寫出會開哪些變更。
  waveBar({ wave, plans }, t) {
    return h_flex()
      .flex_none()
      .gap(8)
      .px(10)
      .py(6)
      .items_center()
      .rounded(10)
      .border_1()
      .border_color(t.border)
      .bg(t.surface)
      .child(
        div()
          .flex_1()
          .min_w_0()
          .text_size(11.5)
          .text_color(t.muted)
          .child(tr("第 {0} 波可以一起开工：{1}", wave, plans.map((p) => p.change).join(tr("、")))),
      )
      .child(
        h_flex()
          .id("start-wave")
          .flex_none()
          .h(22)
          .px(7)
          .items_center()
          .rounded(5)
          .border_1()
          .border_color(t.input)
          .text_size(11.5)
          .cursor_pointer()
          .hover((s) => s.bg(t.hover))
          .tooltip(tr("新建 {0} 个标签页，各运行一个 agent", plans.length))
          .on_click((_e, cx) => this.startTabs(plans, cx))
          .child(tr("整波开工")),
      );
  }

  // 开新标签页跑 speclink init，不交给 agent：它要在真的终端逐题问装哪些技能，答完才写入文件。
  // 写出文件夹，按之前看得到会装在哪里。
  initCard(t) {
    return v_flex()
      .flex_none()
      .px(10)
      .py(8)
      .gap(6)
      .rounded(10)
      .border_1()
      .border_color(t.border)
      .bg(t.surface)
      .child(div().font_semibold().text_size(12).child(tr("此项目尚未初始化 Speclink")))
      .child(div().text_size(11).text_color(t.muted).child(this.root))
      .child(
        h_flex().child(
          h_flex()
            .id("init")
            .h(22)
            .px(7)
            .items_center()
            .rounded(5)
            .border_2()
            .border_color(t.accent)
            .font_semibold()
            .text_size(11.5)
            .cursor_pointer()
            .hover((s) => s.bg(t.hover))
            .tooltip(tr("在新标签页运行 speclink init"))
            .on_click(() => run("tab.new", { name: "speclink init", folder: this.root, argv: ["speclink", "init"] }))
            .child(tr("初始化")),
        ),
      );
  }

  body(ctx, t) {
    const { stages, discussions, promoted, skipped } = this.board;
    const wave = waveToStart(stages, ctx);
    const cards = wave ? [this.waveBar(wave, t)] : [];
    for (const stage of STAGES) {
      cards.push(
        this.card(
          {
            id: stage.id,
            tone: t[stage.tone],
            label: stage.label,
            rows: stages[stage.id].map((row) => this.changeRow(row, stage, ctx, t)),
            open: !this.collapsed.has(stage.id),
          },
          t,
        ),
      );
    }
    cards.push(
      this.card(
        {
          id: "discussions",
          tone: t.ansi5,
          label: "讨论",
          rows: [
            ...unrecordedDiscussions(this.board, ctx.agents, ctx.root).map((a) => this.unrecordedRow(a, ctx, t)),
            ...discussions.map((d) => this.discussionRow(d, ctx, t)),
          ],
          open: !this.collapsed.has("discussions"),
        },
        t,
      ),
    );
    if (promoted.length > 0) {
      cards.push(
        this.card(
          {
            id: "promoted",
            tone: t.muted,
            label: "已转出",
            rows: promoted.map((d) => this.discussionRow(d, ctx, t)),
            open: this.promotedOpen,
          },
          t,
        ),
      );
    }
    if (skipped.length > 0) {
      cards.push(
        div()
          .flex_none()
          .px(4)
          .text_size(11)
          .text_color(t.blocked)
          .child(tr("以下变更无法读取 metadata：{0}", skipped.map((s) => s.change).join(tr("、")))),
      );
    }
    return cards;
  }

  roadmapChangeRow(row, ctx, t) {
    const stageMeta = STAGES.find((s) => s.id === row.stage) ?? STAGES[0];
    const actions = changeActions(row, ctx);
    const start = startChoices(row, ctx);
    const blocked = row.blockedBy.length > 0;
    const percent = row.total > 0 ? Math.round((row.completed / row.total) * 100) : 0;
    const working = changeAgents(row, ctx.agents);
    const hintParts = [];
    if (blocked) hintParts.push(tr("依赖：{0}", row.blockedBy.join(tr("、"))));
    if (row.unlocks?.length > 0) hintParts.push(tr("解锁：{0}", row.unlocks.join(tr("、"))));
    const depLine = hintParts.join(" · ");
    const el = v_flex()
      .id(`roadmap-change-${row.name}`)
      .px(10)
      .py(8)
      .gap(5)
      .when(working.some((a) => a.tab === ctx.focus.tab), (el) => this.markCurrent(el, t))
      .child(
        h_flex()
          .gap(6)
          .items_center()
          .when(blocked, (el) => el.opacity(0.65))
          .child(div().w(7).h(7).flex_none().rounded_full().bg(t[stageMeta.tone]))
          .child(div().flex_1().min_w_0().truncate().child(row.name))
          .child(
            div()
              .flex_none()
              .px(5)
              .py(1)
              .rounded(4)
              .bg(t.raised)
              .text_size(10.5)
              .text_color(t[stageMeta.tone])
              .child(tr(stageMeta.label)),
          )
          .children(this.stationBadges(row, ctx, t))
          .when(row.total > 0, (el) =>
            el.child(div().flex_none().text_size(11).text_color(t.muted).child(`${row.completed}/${row.total}`)),
          )
          .child(this.copyButton(`roadmap-${row.name}`, row.name, tr("复制名称"), t)),
      )
      .when(row.total > 0, (el) =>
        el.child(
          div()
            .h(4)
            .rounded(2)
            .bg(t.raised)
            .overflow_hidden()
            .when(blocked, (bar) => bar.opacity(0.65))
            .child(div().h_full().rounded(2).w(`${percent}%`).bg(t[stageMeta.tone])),
        ),
      )
      .when(Boolean(depLine), (el) =>
        el.child(div().text_size(11).text_color(blocked ? t.blocked : t.muted).child(depLine)),
      )
      .children(working.map((a) => this.agentChip(`roadmap-${row.name}`, a, a.tab === ctx.focus.tab, t)))
      .child(this.skillRow({ id: `roadmap-${row.name}`, actions, start, target: row.name, hint: "" }, t));
    return this.openOnClick(el, speclinkUrl("change", this.root, row.name), t);
  }

  roadmapCutRow(cut, ctx, t) {
    const key = defaultAgent(ctx) ?? "claude";
    const args = `--from-discussion ${cut.slug} ${cut.label}`;
    const cmd = commandText(key, "propose", args);
    const plan = newTabPlan({
      ...ctx,
      skill: "propose",
      target: { kind: "discussion", name: cut.slug, args },
      note: "",
    });
    const ready = plan.kind === "tab";
    return v_flex()
      .id(`roadmap-cut-${cut.slug}-${cut.label}`)
      .px(10)
      .py(8)
      .gap(4)
      .child(
        h_flex()
          .gap(6)
          .items_center()
          .child(div().flex_1().min_w_0().truncate().font_medium().child(cut.label))
          .child(
            this.chip(
              {
                id: `propose-cut-${cut.slug}-${cut.label}`,
                label: tr("生成提案"),
                ready,
                primary: ready,
                tip: ready ? tr("在当前工作区新建标签页并运行：{0}", plan.text) : plan.reason,
                click: (cx) => this.startTabs([plan], cx),
              },
              t,
            ),
          )
          .child(this.copyButton(`cut-${cut.slug}-${cut.label}`, cmd, tr("复制提案指令"), t)),
      )
      .child(div().truncate().text_size(11).text_color(t.muted).child(tr("来源：{0}", `${cut.slug}${cut.topic ? ` · ${cut.topic}` : ""}`)));
  }

  roadmapBody(ctx, t) {
    const { waves, unpromotedCuts } = buildRoadmap(this.board);
    const changeCount = waves.reduce((sum, w) => sum + w.changes.length, 0);
    const wave = waveToStart(this.board.stages, ctx);
    const items = [
      h_flex()
        .id("roadmap-summary")
        .flex_none()
        .gap(6)
        .px(10)
        .py(7)
        .items_center()
        .rounded(10)
        .border_1()
        .border_color(t.border)
        .bg(t.surface)
        .child(
          div()
            .flex_1()
            .min_w_0()
            .text_size(11.5)
            .text_color(t.muted)
            .child(tr("共 {0} 个波次 · {1} 个变更 · {2} 项待转出切片", waves.length, changeCount, unpromotedCuts.length)),
        )
        .when(this.desktopApp && canOpenProject(this.projectTarget()), (el) =>
          el.child(
            this.chip(
              {
                id: "roadmap-desktop",
                label: tr("在桌面端打开"),
                ready: true,
                click: (cx) => this.openInApp([speclinkUrl("roadmap", this.root)], cx),
              },
              t,
            ),
          ),
        )
        .child(
          this.chip(
            {
              id: "roadmap-back",
              label: tr("返回看板"),
              ready: true,
              click: (cx) => {
                this.viewMode = "board";
                cx.notify();
              },
            },
            t,
          ),
        ),
    ];
    if (wave) items.push(this.waveBar(wave, t));
    if (waves.length === 0 && unpromotedCuts.length === 0) {
      items.push(
        v_flex()
          .flex_none()
          .px(10)
          .py(10)
          .gap(4)
          .rounded(10)
          .border_1()
          .border_color(t.border)
          .bg(t.surface)
          .child(div().font_semibold().text_size(12).child(tr("暂无路线图条目")))
          .child(div().text_size(11).text_color(t.muted).child(tr("先发起讨论或创建变更后，波次、依赖与待转出切片会呈现在这里。"))),
      );
      return items;
    }
    for (const w of waves) {
      items.push(
        this.card(
          {
            id: `roadmap-wave-${w.wave}`,
            tone: w.wave === 1 ? t.ansi2 : t.accent,
            label: tr("第 {0} 波", w.wave),
            rows: w.changes.map((row) => this.roadmapChangeRow(row, ctx, t)),
            open: !this.collapsed.has(`roadmap-wave-${w.wave}`),
          },
          t,
        ),
      );
    }
    if (unpromotedCuts.length > 0) {
      items.push(
        this.card(
          {
            id: "roadmap-cuts",
            tone: t.ansi5,
            label: "待转出切片",
            rows: unpromotedCuts.map((cut) => this.roadmapCutRow(cut, ctx, t)),
            open: !this.collapsed.has("roadmap-cuts"),
          },
          t,
        ),
      );
    }
    return items;
  }

  headerButton(id, iconName, label, t, onClick) {
    return h_flex()
      .id(id)
      .h(24)
      .px(7)
      .gap(4)
      .items_center()
      .rounded(5)
      .border_1()
      .border_color(t.input)
      .text_size(11.5)
      .cursor_pointer()
      .hover((s) => s.bg(t.hover))
      .on_mouse_down("left", (_e, cx) => cx.stop_propagation())
      .on_click((_e, cx) => onClick(cx))
      .child(icon(iconName, 12, t.foreground))
      .child(label);
  }

  // Logo：安装了 Speclink 桌面 app 时可切过去；仅使用 Herdr + CLI 时点击返回看板并刷新。
  logoButton(t) {
    const target = this.projectTarget();
    const attempts = logoOpenAttempts({ root: this.root, ...target });
    return div()
      .id("open-app")
      .flex_none()
      .p(3)
      .rounded(5)
      .cursor_pointer()
      .hover((s) => s.bg(t.hover))
      .tooltip(
        this.desktopApp
          ? canOpenProject(target)
            ? tr("在 Speclink 应用中打开此项目")
            : tr("打开 Speclink 应用")
          : tr("刷新 Speclink 面板"),
      )
      .on_mouse_down("left", (_e, cx) => cx.stop_propagation())
      .on_click((_e, cx) => {
        if (this.desktopApp) {
          this.openFirstInApp(attempts, cx);
        } else {
          this.viewMode = "board";
          this.reload(cx);
          cx.notify();
        }
      })
      .child(image(`${ICONS}/logo.svg`).w(21).h(24));
  }

  render(cx) {
    if (this.other) return this.other.render(cx);
    const t = theme();
    this.follow(cx);
    // 每次都读 window()，窗口回到画面时 render 才会重跑。
    if (window()?.visible !== false && this.stale) this.reload(cx);
    const ctx = this.context();
    const overlay = props()?.overlay;
    const roadmapActive = this.viewMode === "roadmap";
    return v_flex()
      .id("speclink-panel")
      .size_full()
      // 没有 overlay 就是停靠在布局里，底色和侧边栏一样，卡片才浮得出来。
      .bg(overlay ? t.popover : t.chrome)
      .text_size(12.5)
      .text_color(t.foreground)
      .child(
        h_flex()
          .flex_none()
          .h(46)
          .px(10)
          .gap(6)
          .items_center()
          .child(this.logoButton(t))
          .child(
            v_flex()
              .flex_1()
              .min_w_0()
              .child(div().truncate().font_semibold().child(this.root ? baseName(this.root) : "Speclink"))
              .child(
                div()
                  .truncate()
                  .text_size(11)
                  .text_color(t.muted)
                  .child([tr("当前工作区"), this.place.branch].filter(Boolean).join(" · ")),
              ),
          )
          // 读得到项目时显示「路线图」切换按钮，内置展示波次、依赖与待转出切片。
          .when(Boolean(this.board), (el) =>
            el.child(
              this.headerButton("roadmap", "roadmap", tr("路线图"), t, (cx) => this.toggleRoadmap(cx))
                .tooltip(roadmapActive ? tr("返回看板") : tr("查看项目路线图"))
                .when(roadmapActive, (btn) => btn.border_2().border_color(t.accent).font_semibold()),
            ),
          )
          // 还没初始化时，自己装的技能还是能用。
          .child(this.headerButton("skills", "zap", tr("技能"), t, () => run("mod.open", { name: "speclink-new" })))
          .when(Boolean(overlay), (el) =>
            el.child(
              div()
                .id("close")
                .p(4)
                .rounded(5)
                .cursor_pointer()
                .hover((s) => s.bg(t.hover))
                .on_mouse_down("left", (_e, cx) => cx.stop_propagation())
                .on_click(() => closeOverlay(overlay))
                .child(icon("x", 13, t.muted)),
            ),
          ),
      )
      .child(
        v_flex()
          .id("speclink-body")
          .flex_1()
          .min_h(0)
          .overflow_y_scrollbar()
          .px(8)
          .pb(8)
          .gap(8)
          // 窗口只能从标题栏拖拽，内容区的按压留给滚动与按钮。
          .on_mouse_down("left", (_e, cx) => cx.stop_propagation())
          .when(Boolean(this.error), (el) =>
            el.child(div().flex_none().px(4).text_size(11.5).text_color(t.blocked).child(this.error)),
          )
          .when(this.uninitialized, (el) => el.child(this.initCard(t)))
          .when(!this.board && !this.error && !this.uninitialized, (el) =>
            el.child(div().flex_none().px(4).text_size(11.5).text_color(t.dim).child(tr("加载中…"))),
          )
          .when(Boolean(this.board), (el) =>
            el.children(roadmapActive ? this.roadmapBody(ctx, t) : this.body(ctx, t)),
          ),
      );
  }
}
