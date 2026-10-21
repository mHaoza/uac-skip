#!/usr/bin/env node
/**
 * 一键更新版本并触发发版
 *
 *   pnpm release patch      # 0.1.0 -> 0.1.1
 *   pnpm release minor      # 0.1.0 -> 0.2.0
 *   pnpm release major      # 0.1.0 -> 1.0.0
 *   pnpm release 0.4.2      # 直接指定版本
 *   pnpm release patch --dry-run    # 只预览，不改文件不提交
 *   pnpm release patch --no-push    # 只提交，不推送
 *
 * 做的事情：把版本号同步写入 package.json / tauri.conf.json / Cargo.toml / Cargo.lock，
 * 然后提交成 `release: vX.Y.Z`。推送后 GitHub Actions 会自动打包并创建 Release。
 */

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

/** 需要同步版本号的文件，正则只替换第一个匹配，避免动到依赖的版本 */
const TARGETS = [
  {
    file: "package.json",
    pattern: /("version"\s*:\s*")[^"]+(")/,
  },
  {
    file: "src-tauri/tauri.conf.json",
    pattern: /("version"\s*:\s*")[^"]+(")/,
  },
  {
    file: "src-tauri/Cargo.toml",
    pattern: /(^version\s*=\s*")[^"]+(")/m,
  },
  {
    file: "src-tauri/Cargo.lock",
    pattern: /(\[\[package\]\]\r?\nname = "uac-skip"\r?\nversion = ")[^"]+(")/,
  },
];

const c = {
  dim: (s) => `\x1b[2m${s}\x1b[0m`,
  red: (s) => `\x1b[31m${s}\x1b[0m`,
  green: (s) => `\x1b[32m${s}\x1b[0m`,
  yellow: (s) => `\x1b[33m${s}\x1b[0m`,
  cyan: (s) => `\x1b[36m${s}\x1b[0m`,
  bold: (s) => `\x1b[1m${s}\x1b[0m`,
};

function fail(message, hint) {
  console.error(`\n${c.red("✗")} ${message}`);
  if (hint) console.error(`  ${c.dim(hint)}`);
  process.exit(1);
}

function git(args, options = {}) {
  const result = execFileSync("git", args, {
    cwd: ROOT,
    encoding: "utf8",
    ...options,
  });
  // stdio: "inherit" 时返回 null，不能直接 trim
  return typeof result === "string" ? result.trim() : "";
}

function gitQuiet(args) {
  try {
    return git(args, { stdio: ["ignore", "pipe", "ignore"] });
  } catch {
    return "";
  }
}

/* ---------------- 版本号处理 ---------------- */

const SEMVER = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/;

function parseVersion(text) {
  const match = SEMVER.exec(text);
  if (!match) return null;
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    pre: match[4] ?? null,
  };
}

function formatVersion({ major, minor, patch, pre }) {
  return `${major}.${minor}.${patch}${pre ? `-${pre}` : ""}`;
}

/** 返回 a > b ? 1 : a < b ? -1 : 0（忽略预发布标识，够用即可） */
function compareVersion(a, b) {
  for (const key of ["major", "minor", "patch"]) {
    if (a[key] !== b[key]) return a[key] > b[key] ? 1 : -1;
  }
  return 0;
}

function nextVersion(current, target) {
  if (target === "patch") {
    return formatVersion({ ...current, patch: current.patch + 1, pre: null });
  }
  if (target === "minor") {
    return formatVersion({ major: current.major, minor: current.minor + 1, patch: 0, pre: null });
  }
  if (target === "major") {
    return formatVersion({ major: current.major + 1, minor: 0, patch: 0, pre: null });
  }
  return target;
}

/* ---------------- 主流程 ---------------- */

const argv = process.argv.slice(2);
const flags = new Set(argv.filter((item) => item.startsWith("--")));
const positional = argv.filter((item) => !item.startsWith("--"));

const dryRun = flags.has("--dry-run");
const skipPush = flags.has("--no-push") || dryRun;
const target = (positional[0] ?? "patch").replace(/^v/, "");

console.log(c.bold("\nUACSkip 发版\n"));

/* 1. 环境检查 */
if (!gitQuiet(["rev-parse", "--is-inside-work-tree"])) {
  fail("当前目录不是 git 仓库", "请先 git init 或切换到仓库目录");
}

const branch = gitQuiet(["branch", "--show-current"]);
if (!branch) {
  fail("当前处于 detached HEAD 状态", "请先切换到分支，例如 git checkout main");
}

const dirty = gitQuiet(["status", "--porcelain"]);
if (dirty) {
  console.error(`${c.yellow("!")} 工作区有未提交的改动：\n`);
  console.error(
    dirty
      .split("\n")
      .slice(0, 12)
      .map((line) => `    ${line}`)
      .join("\n"),
  );
  fail("请先提交或暂存这些改动再发版", "发版会单独提交版本号，混入其他改动会让发布说明失真");
}

/* 2. 计算新版本 */
const pkgPath = join(ROOT, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
const currentText = pkg.version;
const current = parseVersion(currentText);
if (!current) fail(`package.json 里的版本号无法解析：${currentText}`);

const nextText = nextVersion(current, target);
const next = parseVersion(nextText);
if (!next) {
  fail(`版本号格式无效：${target}`, "支持 patch / minor / major，或形如 0.2.0、1.0.0-beta.1 的具体版本");
}

if (compareVersion(next, current) <= 0) {
  fail(`新版本 ${nextText} 必须大于当前版本 ${currentText}`, "版本号只能递增");
}

const tag = `v${nextText}`;
const existingTag = gitQuiet(["rev-parse", "-q", "--verify", `refs/tags/${tag}`]);
if (existingTag) {
  fail(`标签 ${tag} 已存在`, "换一个更高的版本号");
}

console.log(`  当前版本  ${c.dim(currentText)}`);
console.log(`  新的版本  ${c.cyan(nextText)}  ${c.dim(`(标签 ${tag})`)}`);
console.log(`  目标分支  ${c.dim(branch)}`);
console.log(`  提交信息  ${c.dim(`release: ${tag}`)}`);

/* 3. 写入版本号 */
const changed = [];
for (const item of TARGETS) {
  const fullPath = join(ROOT, item.file);
  let text;
  try {
    text = readFileSync(fullPath, "utf8");
  } catch {
    console.log(`  ${c.yellow("跳过")} ${item.file} ${c.dim("(文件不存在)")}`);
    continue;
  }
  if (!item.pattern.test(text)) {
    console.log(`  ${c.yellow("跳过")} ${item.file} ${c.dim("(未找到版本号字段)")}`);
    continue;
  }
  const updated = text.replace(item.pattern, `$1${nextText}$2`);
  if (updated !== text) {
    if (!dryRun) writeFileSync(fullPath, updated, "utf8");
    changed.push(item.file);
  }
}

if (changed.length === 0) {
  fail("没有文件需要更新，可能版本号已经同步过了");
}

console.log(`\n${c.green("✓")} 已更新 ${changed.length} 个文件`);
for (const file of changed) console.log(`    ${c.dim(file)}`);

if (dryRun) {
  console.log(`\n${c.yellow("dry-run")} 仅预览，未修改文件、未提交。\n`);
  process.exit(0);
}

/* 4. 提交 */
git(["add", "--", ...changed], { stdio: "inherit" });
git(["commit", "-m", `release: ${tag}`], { stdio: "inherit" });
console.log(`${c.green("✓")} 已提交 ${c.dim(`release: ${tag}`)}`);

/* 5. 推送 */
if (skipPush) {
  console.log(`\n${c.yellow("已跳过推送")} 需要发布时执行：${c.cyan(`git push origin ${branch}`)}\n`);
  process.exit(0);
}

try {
  git(["push", "origin", branch], { stdio: "inherit" });
} catch {
  fail(
    "推送失败，提交已保存在本地",
    `网络或权限问题排除后手动执行：git push origin ${branch}`,
  );
}

const remote = gitQuiet(["remote", "get-url", "origin"])
  .replace(/^git@github\.com:/, "https://github.com/")
  .replace(/\.git$/, "")
  .replace(/^https:\/\/[^@]+@/, "https://");

console.log(`\n${c.green("✓")} 已推送，GitHub Actions 将开始打包\n`);
console.log(`  ${c.dim("提交信息")}  release: ${tag}`);
console.log(`  ${c.dim("发布说明")}  上一个 Release → ${tag} 之间的提交`);
console.log(`  ${c.dim("产物")}      UACSkip-${nextText}-x64-portable.exe`);
if (remote.startsWith("http")) {
  console.log(`  ${c.dim("进度")}      ${remote}/actions\n`);
}
