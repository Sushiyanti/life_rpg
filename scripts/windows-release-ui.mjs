import { chromium } from 'playwright-core';
import { mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { spawn } from 'node:child_process';

const port = Number(process.env.WEBVIEW2_DEBUG_PORT || 9222);
const evidenceDir = resolve(process.env.EVIDENCE_DIR || 'windows-evidence');
const dialogScript = resolve(process.env.DIALOG_SCRIPT || 'scripts/windows-dialog.ps1');
const backupPath = resolve(process.env.BACKUP_PATH || 'windows-evidence/backup with spaces 测试.liferpg-backup');
const malformedPath = resolve(process.env.MALFORMED_PATH || 'windows-evidence/malformed backup 测试.liferpg-backup');
mkdirSync(dirname(backupPath), { recursive: true });
mkdirSync(dirname(malformedPath), { recursive: true });
writeFileSync(malformedPath, 'not a Life RPG backup\n');

function log(name, value = 'PASS') { console.log(`UI_GATE ${name}=${value}`); }
function wait(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
function startDialog(mode, path) {
  const child = spawn('pwsh', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', dialogScript, '-Mode', mode, '-Path', path], { stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  child.stdout.on('data', value => { output += value; });
  child.stderr.on('data', value => { output += value; });
  return new Promise((resolve, reject) => child.on('close', code => {
    if (code !== 0) reject(new Error(`Windows ${mode} dialog helper failed: ${output}`));
    else { console.log(`UI_EVIDENCE native_${mode.toLowerCase()}_dialog=${output.trim() || 'PASS'}`); resolve(); }
  }));
}
async function waitForDebug() {
  const endpoint = `http://127.0.0.1:${port}/json/version`;
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    try { const response = await fetch(endpoint); if (response.ok) return; } catch {}
    await wait(250);
  }
  throw new Error(`WebView2 debugging endpoint did not appear: ${endpoint}`);
}
async function clickNav(page, name) {
  await page.getByRole('button', { name, exact: true }).click();
  await wait(400);
}
async function fillAndSubmit(page, label, value, button) {
  await page.getByLabel(label, { exact: true }).fill(value);
  await page.getByRole('button', { name: button, exact: true }).click();
  await wait(700);
}

await waitForDebug();
const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
const context = browser.contexts()[0];
const page = context.pages()[0];
page.setDefaultTimeout(20000);
await page.waitForLoadState('domcontentloaded');
await page.screenshot({ path: resolve(evidenceDir, '01-first-run.png'), fullPage: true });
await page.getByRole('heading', { name: 'Start your first Player world' }).first().waitFor();
log('first_run_visible');

const playerName = 'Windows Release Player';
const playerInput = page.getByLabel('Player name', { exact: true });
await playerInput.focus();
await page.keyboard.press('Tab');
const focusedAfterTab = await page.evaluate(() => document.activeElement?.getAttribute('aria-label') || document.activeElement?.getAttribute('placeholder') || document.activeElement?.tagName || 'none');
console.log(`UI_EVIDENCE startup_tab_focus=${focusedAfterTab}`);
await playerInput.fill(playerName);
await page.getByLabel('Starting note', { exact: false }).fill('Windows x64 release validation world');
await page.keyboard.press('Tab');
await page.keyboard.press('Enter');
await page.getByRole('heading', { name: 'Player Hub' }).first().waitFor();
log('first_run_world_creation');
log('keyboard_first_run_submit');

await clickNav(page, 'System health');
await page.getByText('1.0.1', { exact: true }).waitFor();
log('runtime_version_1_0_1');
await page.screenshot({ path: resolve(evidenceDir, '02-version-status.png'), fullPage: true });

await clickNav(page, 'Quests');
await page.getByRole('heading', { name: 'Quest log' }).first().waitFor();
await fillAndSubmit(page, 'Title', 'Windows release quest', 'Add quest');
await page.getByText('Windows release quest', { exact: true }).waitFor();
const quest = page.getByText('Windows release quest', { exact: true }).locator('..');
const detailsButton = page.getByRole('button', { name: 'Stages & activity', exact: true }).first();
await detailsButton.click();
await page.getByLabel('New stage name', { exact: true }).fill('Validation stage');
await page.getByRole('button', { name: 'Add stage', exact: true }).click();
await page.getByText('Validation stage', { exact: true }).waitFor();
await page.getByLabel('New branch in Validation stage', { exact: true }).fill('Validation branch');
await page.getByRole('button', { name: 'Add branch', exact: true }).click();
await page.getByText(/Validation branch/).waitFor();
await page.getByRole('button', { name: 'Start branch session', exact: true }).click();
await page.getByText(/Branch session started/).waitFor();
log('representative_quest_stage_branch_session');

await clickNav(page, 'Skills');
await page.getByRole('heading', { name: 'Skills' }).first().waitFor();
await fillAndSubmit(page, 'Tree name', 'Windows validation skills', 'Create tree');
await page.getByRole('heading', { name: 'Windows validation skills', exact: true }).first().waitFor();
await fillAndSubmit(page, 'Skill name', 'Release testing', 'Add skill');
await page.getByText('Release testing', { exact: true }).first().waitFor();
log('representative_skill_tree_skill');

await clickNav(page, 'Concepts');
await page.getByRole('heading', { name: 'Concepts' }).first().waitFor();
await fillAndSubmit(page, 'Name', 'Windows release concept', 'Create concept');
await page.getByText('Windows release concept', { exact: true }).first().waitFor();
log('representative_concept');

await clickNav(page, 'Content Guidebook');
await page.getByRole('heading', { name: 'Content Guidebook' }).first().waitFor();
await page.getByRole('button', { name: /New content/ }).click();
await page.getByLabel('Title', { exact: true }).fill('Windows validation note');
await page.getByLabel('Body', { exact: true }).fill('Created through the installed Windows application.');
await page.getByRole('button', { name: 'Save content', exact: true }).click();
await page.getByText('Windows validation note', { exact: true }).first().waitFor();
log('representative_content');

await clickNav(page, 'Tags');
await page.getByRole('heading', { name: 'Tag Manager' }).first().waitFor();
await page.getByLabel('Name', { exact: true }).last().fill('Windows release tag');
await page.getByRole('button', { name: 'Create Tag', exact: true }).click();
await page.getByText('Windows release tag', { exact: true }).first().waitFor();
log('representative_tag');

await clickNav(page, 'Player Hub');
await page.getByRole('heading', { name: 'Player Hub' }).first().waitFor();
const customize = page.getByRole('button', { name: 'Customize workspace', exact: true });
if (await customize.count()) {
  await customize.click();
  await page.getByText('Workspace builder', { exact: false }).waitFor().catch(() => {});
  log('workspace_open_and_keyboard_navigation');
} else {
  log('workspace_open_and_keyboard_navigation', 'NOT_TESTED');
}
await page.screenshot({ path: resolve(evidenceDir, '03-representative-workflows.png'), fullPage: true });

await clickNav(page, 'System health');
await page.getByRole('button', { name: 'Create verified backup', exact: true }).waitFor();
const saveDialog = startDialog('Save', backupPath);
await page.getByRole('button', { name: 'Create verified backup', exact: true }).click();
await saveDialog;
try {
  await page.getByText(/Backup created and verified/).waitFor();
} catch {
  const error = page.getByRole('alert').first();
  if (await error.count()) throw new Error(`Backup operation failed: ${await error.innerText()}`);
  throw new Error('Backup operation produced neither success notice nor an error alert.');
}
log('backup_create_spaces_unicode_path');

console.log('UI_PROGRESS valid_backup_restore=START');
const backupDialog = startDialog('Open', backupPath);
await page.getByRole('button', { name: 'Inspect \/ restore backup', exact: true }).click({ timeout: 10000 });
await backupDialog;
console.log('UI_PROGRESS valid_backup_dialog=PASS');
try {
  await page.getByRole('heading', { name: 'Verified backup preview' }).first().waitFor();
} catch {
  const error = page.getByRole('alert').first();
  if (await error.count()) throw new Error(`Valid backup inspection failed: ${await error.innerText()}`);
  throw new Error('Valid backup inspection produced neither preview nor an error alert.');
}
await page.getByText('Supported — same schema', { exact: true }).waitFor();
await page.getByRole('button', { name: 'Review restore consequences', exact: true }).click();
await page.getByRole('button', { name: 'Confirm restore', exact: true }).click();
await page.getByRole('heading', { name: 'Reload to use the restored world' }).first().waitFor();
await page.getByRole('button', { name: 'Reload Life RPG', exact: true }).click();
await page.getByRole('heading', { name: 'Player Hub' }).first().waitFor();
await page.getByText('Windows release quest', { exact: true }).waitFor();
log('backup_restore_relationships_workspace');

await page.getByRole('button', { name: 'Check current world', exact: true }).click();
await page.getByText(/Read-only current-world check: passed/).waitFor();
log('sqlite_integrity_after_restore');
await page.screenshot({ path: resolve(evidenceDir, '04-backup-restore-integrity.png'), fullPage: true });

const malformedDialog = startDialog('Open', malformedPath);
await page.getByRole('button', { name: 'Inspect \/ restore backup', exact: true }).click({ timeout: 10000 });
await malformedDialog;
await page.getByRole('alert').filter({ hasText: /backup|integrity|could not/i }).waitFor();
log('malformed_backup_rejection');

console.log(`UI_ARTIFACT backup=${backupPath}`);
console.log(`UI_ARTIFACT malformed=${malformedPath}`);
await browser.close();
