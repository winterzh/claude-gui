#!/usr/bin/env node
/**
 * Pre-build script: reads config-packaging/config.json and injects values
 * into tauri.conf.json, Cargo.toml, package.json, and copies icon/splash.
 *
 * If config file doesn't exist or enabled !== true, exits silently (no-op).
 */
import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, '..');
const CONFIG_PATH = path.join(ROOT, 'config-packaging', 'config.json');

// --- Check config ---
if (!fs.existsSync(CONFIG_PATH)) {
  console.log('[packaging] No config-packaging/config.json found, skipping.');
  process.exit(0);
}

let config;
try {
  config = JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8'));
} catch (e) {
  console.error('[packaging] Failed to parse config.json:', e.message);
  process.exit(1);
}

if (config.enabled !== true) {
  console.log('[packaging] config.json exists but enabled !== true, skipping.');
  process.exit(0);
}

console.log('[packaging] Applying packaging config...');

// --- Patch tauri.conf.json ---
const tauriConfPath = path.join(ROOT, 'src-tauri', 'tauri.conf.json');
const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));

if (config.appName) {
  tauriConf.productName = config.appName.replace(/\s+/g, '-');
  if (tauriConf.app?.windows?.[0]) {
    tauriConf.app.windows[0].title = config.appName;
  }
}
if (config.version) {
  tauriConf.version = config.version;
}
if (config.identifier) {
  tauriConf.identifier = config.identifier;
}

fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n');
console.log('[packaging] Patched tauri.conf.json');

// --- Patch Cargo.toml ---
const cargoPath = path.join(ROOT, 'src-tauri', 'Cargo.toml');
let cargo = fs.readFileSync(cargoPath, 'utf8');

if (config.appSlug) {
  cargo = cargo.replace(/^name = ".*"$/m, `name = "${config.appSlug}"`);
  const libName = config.appSlug.replace(/-/g, '_') + '_lib';
  cargo = cargo.replace(/^name = ".*_lib"$/m, `name = "${libName}"`);
}
if (config.version) {
  cargo = cargo.replace(/^version = ".*"$/m, `version = "${config.version}"`);
}
if (config.appName) {
  cargo = cargo.replace(/^description = ".*"$/m, `description = "A one-click ${config.appName}"`);
}
if (config.company?.authors) {
  const authors = JSON.stringify(config.company.authors);
  cargo = cargo.replace(/^authors = \[.*\]$/m, `authors = ${authors}`);
}

fs.writeFileSync(cargoPath, cargo);
console.log('[packaging] Patched Cargo.toml');

// --- Patch package.json ---
const pkgPath = path.join(ROOT, 'package.json');
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));

if (config.appSlug) {
  pkg.name = config.appSlug;
}
if (config.version) {
  pkg.version = config.version;
}

fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
console.log('[packaging] Patched package.json');

// --- Generate icons ---
if (config.icon) {
  const iconPath = path.resolve(ROOT, config.icon);
  if (fs.existsSync(iconPath)) {
    console.log('[packaging] Generating icons from', config.icon);
    try {
      execSync(`npx tauri icon "${iconPath}"`, { cwd: ROOT, stdio: 'inherit' });
      console.log('[packaging] Icons generated');
    } catch (e) {
      console.warn('[packaging] Icon generation failed:', e.message);
    }
  } else {
    console.warn('[packaging] Icon file not found:', iconPath);
  }
}

// --- Copy splash ---
if (config.splash) {
  const splashSrc = path.resolve(ROOT, config.splash);
  const splashDst = path.join(ROOT, 'public', 'splash.png');
  if (fs.existsSync(splashSrc)) {
    fs.mkdirSync(path.dirname(splashDst), { recursive: true });
    fs.copyFileSync(splashSrc, splashDst);
    console.log('[packaging] Copied splash image to public/splash.png');
  } else {
    console.warn('[packaging] Splash file not found:', splashSrc);
  }
}

console.log('[packaging] Done.');
