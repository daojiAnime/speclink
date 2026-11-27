// 界面文字跟随系统语言：中文系统用简体中文，其他系统用英文。
// 面板自身的文字以中文为原文，Herdr 的文字以英文为原文，所以 tr() 查询的表根据语言决定，查不到则使用原文。
// 本文件不依赖 Herdr API，scripts/herdr/panel-model.test.mjs 会直接测试，并检查每句中文都有对应英文。

let lang = "zh";

export const setLang = (next) => {
  lang = next;
};

// macOS 的 AppleLanguages 形如：( "zh-Hans-CN" )。其他系统是 zh_CN.UTF-8。以第一个语言为准。
export const langFrom = (text) => (text.match(/[A-Za-z]{2,}/)?.[0].toLowerCase() === "zh" ? "zh" : "en");

const READ = 'defaults read -g AppleLanguages 2>/dev/null || echo "$LC_ALL $LC_MESSAGES $LANG"';

// Herdr 的 JS 环境没有 Intl 和 navigator，需要询问系统。exec 由调用方传入，本文件才不用依赖 Herdr API。
// 先使用上次缓存的结果，避免界面闪烁切换语言。
export function watchLang(exec, cx, ui) {
  lang = localStorage.getItem("lang") ?? lang;
  if (ui) translateHerdr(ui);
  cx.spawn(async (cx) => {
    const r = await exec("sh", ["-c", READ]).catch(() => null);
    const next = langFrom(r?.stdout ?? "");
    localStorage.setItem("lang", next);
    if (next === lang) return;
    lang = next;
    cx.notify();
  });
}

export const TO_ZH = {
  // Herdr 内置界面：标签栏、状态栏
  Files: "文件",
  Changes: "变更",
  commands: "命令",
  // 设置页
  Look: "外观",
  Keyboard: "快捷键",
  Machines: "机器",
  MACHINES: "机器",
  Alerts: "提醒",
  Updates: "更新",
  Privacy: "隐私",
  Terminals: "终端",
  System: "跟随系统",
  Light: "浅色",
  Dark: "深色",
  "Type to search fonts": "输入文字搜索字体",
  "This layout sets its own; yours shows in the others.": "此布局有独立的设置，你的通用设置会应用于其他布局。",
  LAYOUT: "布局",
  THEME: "主题",
  Appearance: "外观模式",
  "Light or dark, or as your system is now.": "浅色、深色，或跟随当前系统设置。",
  "For the layout you're in: each layout keeps its own light and dark theme.": "仅影响当前布局：每个布局各自记住浅色和深色主题。",
  "SIZE AND FONTS": "大小与字体",
  Zoom: "缩放",
  "Everything: terminals, panels, text.": "全局一起缩放：终端、面板和文字。",
  Reset: "重置",
  "Terminal font": "终端字体",
  "Interface font": "界面字体",
  "Automatic (Alt on US keyboards)": "自动（美式键盘作为 Alt）",
  "Types characters": "输入字符",
  "Left Option is Alt": "左 Option 作为 Alt",
  "Right Option is Alt": "右 Option 作为 Alt",
  "OPTION KEY": "Option 键",
  "Use Option for terminal shortcuts or to type accents and special characters.": "Option 键可用作终端快捷键，也可用于输入重音符号和特殊字符。",
  "Couldn't read machines.": "无法读取机器列表。",
  "It didn't work.": "未能成功。",
  "Updating…": "更新中…",
  "Renaming…": "重命名中…",
  "Removing…": "移除中…",
  Update: "更新",
  Rename: "重命名",
  Remove: "移除",
  "Copying this computer's herdr-core. Its terminals keep running.": "正在复制本机的 herdr-core。其终端会继续运行。",
  Cancel: "取消",
  "Remove machine": "移除机器",
  "New name (addresses using the old name stop working)": "新名称（使用旧名称的地址将失效）",
  Save: "保存",
  "Your computers, reached over ssh. Their terminals keep running when you disconnect.": "通过 SSH 连接的电脑。断开连接后，上面的终端会继续运行。",
  "Connected · hc version unknown": "已连接 · 未知 hc 版本",
  "Reading machines…": "读取机器中…",
  "No other machines yet.": "暂无其他机器。",
  "Add a machine": "添加机器",
  "Herdr's sound": "Herdr 音效",
  SOUNDS: "音效",
  Sounds: "音效",
  "Play sounds": "播放音效",
  "When an agent needs you, and when one finishes while you're not looking. A note shows in the corner either way.": "当 agent 需要你，或在你未查看时完成任务时播放。无论是否开启，角落都会显示通知。",
  "An agent needs you": "agent 需要你",
  Play: "播放",
  "An agent finished": "agent 已完成",
  "Want your own sounds? Ask your agent to set sounds.done and sounds.blocked in settings.json to an MP3.": "想自定义音效？让 agent 将 settings.json 中的 sounds.done 和 sounds.blocked 设为 MP3 文件路径。",
  "Checking…": "检查中…",
  "Up to date.": "已是最新版本。",
  "No successful check. Check your connection and try again.": "尚未检查成功。请检查网络连接后重试。",
  "✓ Up to date": "✓ 已是最新版本",
  "Check now": "立即检查",
  "A build from source: it updates with git, not here.": "这是从源码构建的版本，请通过 git 更新，而非此处。",
  "Restart to update": "重启并更新",
  "Updating… your terminals keep running.": "更新中…你的终端会继续运行。",
  "Try again": "重试",
  "Update herdr-core": "更新 herdr-core",
  "Not checked: a build from source looks for no updates.": "未检查：从源码构建的版本不会检查更新。",
  "What's new": "新功能",
  "Herdr looks for both every hour and tells you in the corner. herdr-core runs your terminals, so they keep running through its updates.": "Herdr 每小时检查这两项更新并在角落通知。herdr-core 负责运行终端，因此更新期间终端不会中断。",
  "USAGE DATA": "使用数据",
  "Share anonymous usage data": "分享匿名使用数据",
  "Once a day: which agents and features you use, how many spaces and tabs, your system and Herdr's version. Never what's in your terminals, never names of files, folders or projects.": "每天一次：统计使用的 agent 和功能、工作区和标签页数量、系统及 Herdr 版本。绝不包含终端内容，也不包含文件、文件夹或项目名称。",
  "What we send": "发送的数据内容",
  "This build never sends: it's built from source. You can still see what a release would send.": "此版本从源码构建，不会发送数据。你仍可查看正式版会发送的内容。",
  "Exactly what leaves your computer, word for word.": "一字不差地展示将离开本机的内容。",
  Hide: "隐藏",
  Show: "显示",
  "Nothing is collected or sent while it's off.": "关闭时不会收集或发送任何数据。",
  "Sent as is, soon.": "很快将按原样发送。",
  "Today so far": "今日截至目前",
  "A draft: it changes until the day ends (UTC), then goes out once.": "此为草稿：在当天（UTC）结束前持续更新，结束后发送一次。",
  "Send feedback": "发送反馈",
  // 文件面板
  "Cannot list this folder": "无法列出此文件夹",
  "This file is no longer available": "此文件已不存在",
  "Binary file · no text preview": "二进制文件 · 无文本预览",
  "Cannot preview this file (binary, too large, or unreadable)": "无法预览此文件（二进制、过大或不可读）",
  "Loading files…": "加载文件中…",
  "No files": "没有文件",
  "No folder": "没有文件夹",
  "File list limited to 500 entries per folder and 5,000 total": "文件列表限制每个文件夹最多 500 项，总计最多 5,000 项",
  "Loading file…": "加载文件中…",
  // 变更面板
  "Commit message": "提交信息",
  "Not a git repository": "不是 git 仓库",
  "Cannot preview this diff (too large or unreadable)": "无法预览此差异（过大或不可读）",
  "Binary file · no text diff": "二进制文件 · 无文本差异",
  "Binary file changed": "二进制文件已变更",
  "No text diff available": "无文本差异",
  "Loading changes…": "加载变更中…",
  "No uncommitted changes": "没有未提交的变更",
  "Showing the first 1,000 changed files": "仅显示前 1,000 个变更文件",
  "Select a file to view its changes": "选择一个文件查看变更",
  "Committing…": "提交中…",
  Commit: "提交",
  // 右键和新建标签页菜单
  "Split right": "向右分屏",
  "Split down": "向下分屏",
  "Use Herdr right-click menu": "使用 Herdr 右键菜单",
  "Send right-clicks to pane": "将右键交给窗格内程序",
  "Close pane": "关闭窗格",
  "New tab": "新建标签页",
  Close: "关闭",
  "New worktree": "新建 worktree",
  "Open worktree": "打开 worktree",
  "Remove worktree": "移除 worktree",
  "New space": "新建工作区",
  Keybinds: "快捷键",
  Terminal: "终端",
  "Start an agent": "启动一个 agent",
  "not installed": "未安装",
  "Could not load agents": "无法加载 agent",
  "Checking installed agents…": "检查已安装的 agent…",
  "Could not check installed agents": "无法检查已安装的 agent",
  "New worktree…": "新建 worktree…",
  "Open worktree…": "打开 worktree…",
  // 克隆版侧边栏
  Spaces: "工作区",
  "This computer": "本机",
  "New space there": "在此处新建工作区",
  "New space on this computer": "在本机新建工作区",
  // 机器需要登录时，侧边栏、通知和添加机器对话框附带命令和复制按钮。
  "needs a login: run": "需要登录，请运行",
  Copy: "复制",
  Copied: "已复制",
  // 远端机器断开连接后 agent 会停止时，机器名旁的警告。
  "Agents stop when you disconnect. Click for the fix.": "断开连接后 agent 会停止运行。点击查看修复方式。",
  // 移除 worktree 时，工作区终端 10 秒内尚未停止。
  "The space's terminals haven't stopped yet. Press Enter to check again.": "此工作区的终端尚未停止。按 Enter 重新检查。",
  // 命令面板（⌘⇧P）中的条目，定义在 mod.json 的 palette。
  "Speclink: Skills": "Speclink：技能",
  "Speclink: Panel": "Speclink：面板",
  "Loading…": "加载中…",
  Connect: "连接",
  "Connecting…": "连接中…",
  "rebuilt ·": "已重建 ·",
  "This machine was rebuilt or replaced. Click to connect to the new one.": "此机器已重建或被替换。点击连接到新机器。",
  Agents: "Agent",
  "Sort agents": "排序方式",
  "Sort agents: {0}": "排序方式：{0}",
  Priority: "按状态",
  "Grouped by space": "按工作区分组",
  Pinned: "已固定",
  "Needs you": "需要你",
  Done: "已完成",
  Working: "执行中",
  Idle: "空闲",
  Unpin: "取消固定",
  "Pin to the top": "置顶",
  "No agent status yet. Agents show up here when they report their status. Older versions and agents inside tmux may not.":
    "暂无 agent 上报状态。agent 上报后会显示在这里。旧版本或 tmux 内的 agent 可能不会上报。",
  Settings: "设置",
  "Add machine": "添加机器",
};

export const TO_EN = {
  我的技能: "My skills",
  给: "To",
  主文件夹: "Main folder",
  两站都跑: "Both stations",
  "只 review": "Review only",
  "只 verify": "Verify only",
  只归档: "Archive only",
  归档后提交: "Archive, then commit",
  开发: "Development",
  设计: "Design",
  提交与发布: "Commit and release",
  视频: "Video",
  动画: "Animation",
  内容: "Content",
  工具: "Tools",
  其他: "Other",
  个人: "Personal",
  插件: "Plugin",
  "插件 {0}": "Plugin {0}",
  "{0} 个": "{0}",
  "搜索技能：名称或说明": "Find a skill by name or description",
  找不到匹配的技能: "No matching skill",
  "{0} 当前没有 Speclink 以外的技能": "{0} has no skills other than Speclink",
  选择一个技能: "Select a skill",
  "当前标签页是 {0}，要放入输入框请选它": "The current tab runs {0}. Select it to put the command in the input box",
  "此项目没有 {0} 的 Speclink 技能。请先运行 speclink init --tools {1}": "This project has no Speclink skills for {0}. Run speclink init --tools {1} first",
  当前标签页: "Current tab",
  "perl 退出码 {0}": "perl exit code {0}",
  "↑↓ 移动　→ 补充说明　Tab 切页": "↑↓ Move　→ Note　Tab Page",
  "↑↓ 技能": "↑↓ Skill",
  "←→ 搭配": "←→ Pair",
  "Space 勾选": "Space Check",
  "⇧←→ 方式": "⇧←→ Method",
  "Tab 切页": "Tab Page",
  "、": ", ",
  "{0}：{1}": "{0}: {1}",
  // 阶段与状态
  提案中: "Proposed",
  进行中: "In progress",
  已就绪: "Ready",
  执行中: "Working",
  等待你回答: "Waiting for you",
  等待你允许: "Needs your permission",
  正在向你提问: "Asks you a question",
  等待你登录: "Needs you to sign in",
  已完成: "Done",
  空闲: "Idle",
  正在处理: "Working",
  出错了: "Error",
  正在接收指令: "Taking a command",
  // 面板
  技能未发送: "The skill was not sent",
  "无法打开 Speclink": "Could not open Speclink",
  "等待 {0} 先完成": "Waiting for {0} to finish",
  未能放入输入框: "Not put in the input box",
  附加到当前标签页输入框: "Attach to the input box of the current tab",
  其他技能: "Other skills",
  "切换到此 agent（{0}）": "Go to this agent ({0})",
  "在 Speclink 打开路线图": "Open the roadmap in Speclink",
  "等待 {0}": "Waiting for {0}",
  复制名称: "Copy name",
  "复制 slug": "Copy slug",
  "还有 {0} 刀未转出": "{0} cuts not promoted yet",
  收起: "Show less",
  "还有 {0} 个…": "{0} more…",
  "第 {0} 波可以一起开工：{1}": "Wave {0} can start together: {1}",
  "新建 {0} 个标签页，各运行一个 agent": "Open {0} new tabs, one agent each",
  开工失败: "Could not start",
  "使用 worktree": "Use a worktree",
  "直接 apply": "Apply directly",
  取消: "Cancel",
  "选择使用 worktree 或直接 apply": "Choose a worktree or a direct apply",
  整波开工: "Start the wave",
  "此项目尚未初始化 Speclink": "This project has no Speclink yet",
  "在新标签页运行 speclink init": "Run speclink init in a new tab",
  初始化: "Initialize",
  讨论: "Discussions",
  已转出: "Promoted",
  "以下变更无法读取 metadata：{0}": "Could not read metadata for these changes: {0}",
  "打开 Speclink 应用": "Open the Speclink app",
  "在 Speclink 应用中打开此项目": "Open this project in the Speclink app",
  当前工作区: "Current space",
  路线图: "Roadmap",
  技能: "Skills",
  "加载中…": "Loading…",
  // 技能弹窗
  "输入框一次只能放一个。勾选两个以上请新建标签页": "The input box takes one at a time. With two or more checks, open new tabs",
  主题: "Topic",
  "补充说明（可留空）": "Notes (optional)",
  "此技能作用于整个项目，无需搭配": "This skill works on the whole project. It needs no pairing",
  当前没有可搭配的讨论: "No discussion to pair with",
  当前没有可搭配的变更: "No change to pair with",
  搭配的讨论: "Pair with a discussion",
  搭配的变更: "Pair with a change",
  "全选 ⌘A": "All ⌘A",
  不指定: "None",
  "处理中…": "Working on it…",
  "新建 {0} 个标签页并开工 ⌘⏎": "Start {0} tabs ⌘⏎",
  "新建标签页并开工 ⌘⏎": "Start new tab ⌘⏎",
  "在当前工作区新建标签页并运行：{0}": "Open a new tab in the current space and run: {0}",
  "放入输入框 ⏎": "Put in input ⏎",
  "此项目尚未初始化 Speclink。请先在面板点击「初始化」。": "This project has no Speclink yet. Press Initialize in the panel first.",
  // 共用
  "speclink {0} 退出码 {1}": "speclink {0} exit code {1}",
  "无法粘贴到 {0}（退出码 {1}）": "Could not paste into {0} (exit code {1})",
  "hc 退出码 {0}": "hc exit code {0}",
  // 技能表
  规划: "Plan",
  实现: "Build",
  质量: "Quality",
  收尾: "Finish",
  项目: "Project",
  变更: "Change",
  把模糊的需求谈清楚: "Talk through a fuzzy requirement",
  创建变更与文档: "Create a change and its documents",
  将新需求合入变更: "Add new requirements to a change",
  按任务清单实现: "Implement from the task list",
  "在独立的 worktree 中实现": "Implement in a separate worktree",
  实现前交叉检查文档: "Cross-check the documents before you implement",
  检查代码偏离变更假设的程度: "Check how far the code moved from the change",
  "运行 review 和 verify 两站": "Run the review and verify stations",
  检查实现的质量: "Check the quality of the implementation",
  对照文档检查实现: "Check the implementation against the documents",
  将变更合入规范并归档: "Merge the change into the specs and archive it",
  归档后接着提交此变更: "Archive the change, then commit it",
  "一次只能运行一个 archive+commit，同时提交会互相影响": "Run one archive+commit at a time. Commits at the same time affect each other",
  审查中: "In review",
  已审查: "Reviewed",
  "已审查·之后有变动": "Reviewed · changed since",
  验证中: "In verification",
  已验证: "Verified",
  "已验证·之后有变动": "Verified · changed since",
  "将 worktree 合并回主分支": "Merge the worktree back into the main branch",
  仅提交此变更的文件: "Commit only the files of this change",
  扫描代码寻找改进点: "Scan the code for improvements",
  找出代码中的安全问题: "Find security problems in the code",
  追溯某项能力的来龙去脉: "Trace how a capability came to be",
  从规范生成操作手册: "Generate a manual from the specs",
  从现有代码生成初始规范: "Generate initial specs from the code",
  整理项目说明与产出规则: "Organize project context and artifact rules",
  // 送达与新建标签页原因
  "正在确认可用的 agent": "Checking which agents are available",
  "找不到带有 speclink 技能的 agent": "No agent has the speclink skills",
  开工: "Start",
  这个变更: "this change",
  这场讨论: "this discussion",
  "发送给此讨论的 agent": "Send to the agent on this discussion",
  "当前标签页的 agent {0}": "Agent on the current tab: {0}",
  "讨论中，尚未记录": "Discussing, not recorded yet",
  "{0}的 agent {1}": "Agent on {0}: {1}",
  新建标签页: "Open a new tab",
  "发送给此变更的 agent": "Send to the agent on this change",
  "发送给当前标签页的 agent": "Send to the agent in the current tab",
  "当前标签页没有 agent，可改用新建标签页": "The current tab has no agent. You can open a new tab",
  "当前标签页没有 agent": "The current tab has no agent",
  "{0} 仅支持 Claude Code 和 Antigravity": "{0} works only in Claude Code and Antigravity",
  "已有 agent 在处理": "An agent already works on it",
  "需等待 {0} 落地": "Waiting for {0} to land",
  "还有 {0} 个代码任务未完成": "{0} code tasks are not done",
  "此项目未开启 worktree 流程": "This project has no worktree flow",
  "{0} 的 apply 尚未完成": "The apply of {0} is not finished",
  请勾选要开工的变更: "Check the changes to start",
  "主文件夹一次只能运行一个 apply，可改选 apply-with-worktree": "The main folder runs one apply at a time. You can use apply-with-worktree",
  // 路线图视图
  "第 {0} 波": "Wave {0}",
  "共 {0} 个波次 · {1} 个变更 · {2} 项待转出切片": "{0} waves · {1} changes · {2} unpromoted cuts",
  返回看板: "Back to board",
  在桌面端打开: "Open in Desktop",
  "刷新 Speclink 面板": "Refresh Speclink panel",
  在路线图查看波次: "View waves in roadmap",
  查看项目路线图: "View project roadmap",
  "解锁：{0}": "Unlocks: {0}",
  "依赖：{0}": "Blocked by: {0}",
  待转出切片: "Unpromoted cuts",
  暂无路线图条目: "No roadmap items yet",
  "先发起讨论或创建变更后，波次、依赖与待转出切片会呈现在这里。": "Start a discussion or create a change to see waves, dependencies, and unpromoted cuts here.",
  "来源：{0}": "From: {0}",
  生成提案: "Propose",
  复制提案指令: "Copy propose command",
  // 详情预览与设置页
  提案: "Proposal",
  任务: "Tasks",
  设计: "Design",
  返回: "Back",
  刷新: "Refresh",
  "该变更已归档或不存在": "This change is archived or no longer exists",
  "未找到 {0} 文档": "Artifact {0} not found",
  此文档尚未创建: "This document has not been created yet",
  文档内容为空: "Document is empty",
  设置: "Settings",
  "默认启动 Agent": "Default launch agent",
  "点击面板「开工」或新建标签页时默认使用的 agent，修改后立即持久保存。": "Default agent used by Start and New Tab. Saved immediately.",
  "自动（优先项目配置）": "Auto (prefer project config)",
  "当前自动选用：{0}": "Auto selects: {0}",
  "未检测到已安装的 agent": "No installed agent detected",
  项目已配置: "Configured",
  未配置: "Not configured",
  当前项目: "Current project",
  "路径：{0}": "Path: {0}",
  "已配置工具：{0}": "Configured tools: {0}",
  "未在 .speclink.yaml 配置": "Not set in .speclink.yaml",
  "worktree 流程：{0}": "Worktree flow: {0}",
  已开启: "Enabled",
  未开启: "Disabled",
  "↑↓ 选择　⏎ 确认　Tab 切页": "↑↓ Select  ⏎ Confirm  Tab Page",
};

// Herdr 内置界面中带变量的句子。需整句匹配，名称为用户自定。{n} 为第 n 个捕获组。
const ZH_PATTERNS = [
  [/^(\d+) agents$/, "{0} 个 agent"],
  [/^· (\d+) needs you$/, "· {0} 个需要你"],
  [/^(.+) {2}commands$/, "{0}  命令"],
  [/^([\d,]+) files$/, "{0} 个文件"],
  [/^Theme: (.+)$/, "主题：{0}"],
  [/^Send back to (.+)$/, "退回 {0}"],
  [/^Default \((.+)\)$/, "默认（{0}）"],
  [/^Updated (.+) to hc (.+); its terminals kept running\.$/, "已将 {0} 更新至 hc {1}，其终端继续运行。"],
  [/^Updated (.+); its terminals kept running\.$/, "已更新 {0}，其终端继续运行。"],
  [/^Renamed (.+) to (.+)\.$/, "已将 {0} 重命名为 {1}。"],
  [/^Removed (.+); its terminals keep running there\.$/, "已移除 {0}，其终端继续在该机器运行。"],
  [/^Remove (.+)\? Its terminals keep running there\.$/, "确定移除 {0} 吗？其终端将继续在该机器运行。"],
  [/^Connected · (.+)$/, "已连接 · {0}"],
  [/^Up to date\. Checked at (.+)\.$/, "已是最新版本。检查时间 {0}。"],
  // 侧边栏收起时，机器格子的提示。
  [/^New space on (.+)$/, "在 {0} 新建工作区"],
  // 侧边栏收起时该格的提示：点击打开设置的机器页，内有复制按钮。
  [/^(.+) needs a login: run (.+)\. Click for Copy\.$/, "{0} 需要登录，请运行 {1}。点击前往设置复制命令。"],
  [/^(.+) needs a login: run$/, "{0} 需要登录，请运行"],
  [/^(.+) is ready\. Restart to switch now; it also switches when you quit\.$/, "{0} 已准备就绪。重启即可立即切换，退出 Herdr 时也会自动切换。"],
  [/^(.+) is ready\. Your terminals keep running through it\.$/, "{0} 已准备就绪。更新期间终端会继续运行。"],
  [/^Managed by (.+): update it there\.$/, "由 {0} 管理，请在对应处更新。"],
  [/^The update failed: (.+)$/, "更新失败：{0}"],
  [/^Off for now: (.+) is set where Herdr started\.$/, "当前已关闭：启动 Herdr 的环境中设置了 {0}。"],
  [/^Waiting to send: (.+)$/, "等待发送：{0}"],
  [/^Last sent: (.+)$/, "上次发送：{0}"],
  [/^Sent (.+)\.$/, "发送于 {0}。"],
  [/^Cannot preview this diff \(too large or unreadable\): (.+)$/, "无法预览此差异（过大或不可读）：{0}"],
  [/^Commit failed: (.+)$/, "提交失败：{0}"],
];

// 只查找对照表和上面的正则句子，查不到则保留原文。英文系统不翻译，内置界面本身即为英文。
export function autoTr(text) {
  if (lang !== "zh") return text;
  if (Object.hasOwn(TO_ZH, text)) return TO_ZH[text];
  for (const [pattern, zh] of ZH_PATTERNS) {
    const found = text.match(pattern);
    if (found) return zh.replace(/\{(\d)\}/g, (_match, i) => found[Number(i) + 1]);
  }
  return text;
}

const TRANSLATE = Symbol.for("speclink.translate");

// Herdr 所有 mod 共用同一份 JS 环境，因此修改一次组件原型，内置界面（设置页、菜单、标签栏、状态栏）的文字也会查对照表，
// 无需克隆这些 mod，从任何入口打开效果一致。文档未保证此行为：若 Herdr 日后更改，此段会退回英文，不会出错。
// 用户自身内容不经过这里：文件树和差异内容已由 dock-panel.mjs 的克隆版排除。ui 由调用方传入，此处无需引入 Herdr API。
export function translateHerdr({ div, InputState }) {
  const proto = Object.getPrototypeOf(div());
  const installed = TRANSLATE in proto;
  // mod 重载后需使用最新这份 lang，但不能重复包装两层。
  proto[TRANSLATE] = autoTr;
  if (installed) return;
  const text = (value) => (typeof value === "string" ? proto[TRANSLATE](value) : value);
  const { child, children, tooltip } = proto;
  proto.child = function (value) {
    return child.call(this, text(value));
  };
  proto.children = function (values) {
    return children.call(this, Array.from(values, text));
  };
  proto.tooltip = function (value) {
    return tooltip.call(this, text(value));
  };
  // 输入框的占位文本不是 child。
  const make = InputState?.new;
  if (!make) return;
  InputState.new = function (options) {
    return make.call(this, options?.placeholder ? { ...options, placeholder: text(options.placeholder) } : options);
  };
}

export function tr(text, ...args) {
  const table = lang === "zh" ? TO_ZH : TO_EN;
  return (table[text] ?? text).replace(/\{(\d)\}/g, (_match, i) => args[i]);
}
