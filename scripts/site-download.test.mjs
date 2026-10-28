import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const { pickAssets } = createRequire(import.meta.url)("../site/download.js");

const base = "https://github.com/KookaStudio/KookaPictura/releases/download/v0.2.0/";
const names = [
  "kooka-pictura_0.2.0_amd64.snap",
  "KookaPictura-0.2.0-1.x86_64.rpm",
  "KookaPictura-0.2.0-macOS.dmg",
  "KookaPictura-0.2.0-windows-x64-portable.zip",
  "KookaPictura-0.2.0-windows-x64-setup.exe",
  "KookaPictura-0.2.0-x86_64.AppImage",
  "KookaPictura-0.2.0-x86_64.flatpak",
  "KookaPictura_0.2.0_amd64.deb",
];
const assets = names.map((name) => ({ name, browser_download_url: base + name }));

test("classifies the v0.2.0 assets", () => {
  const p = pickAssets(assets);
  assert.deepEqual(p.linux.map((l) => l.label), ["rpm", "deb", "AppImage", "snap", "flatpak"]);
  assert.equal(p.macos.url, base + "KookaPictura-0.2.0-macOS.dmg");
  assert.equal(p.windows.url, base + "KookaPictura-0.2.0-windows-x64-setup.exe");
});

test("omits a format missing from the release", () => {
  const p = pickAssets(assets.filter((a) => !a.name.endsWith(".snap")));
  assert.ok(!p.linux.some((l) => l.label === "snap"));
});

test("returns nothing for an empty asset list", () => {
  const p = pickAssets([]);
  assert.deepEqual(p, { linux: [], macos: null, windows: null });
});
