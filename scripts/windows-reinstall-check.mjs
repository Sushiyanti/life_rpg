import { chromium } from 'playwright-core';
const port = Number(process.env.WEBVIEW2_DEBUG_PORT || 9223);
const deadline = Date.now() + 30000;
while (Date.now() < deadline) {
  try { const response = await fetch(`http://127.0.0.1:${port}/json/version`); if (response.ok) break; } catch {}
  await new Promise(resolve => setTimeout(resolve, 250));
}
const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
const page = browser.contexts()[0].pages()[0];
page.setDefaultTimeout(20000);
await page.getByRole('heading', { name: 'Player Hub' }).first().waitFor();
await page.getByText('Windows release quest', { exact: true }).waitFor();
await page.getByRole('button', { name: 'System health', exact: true }).click();
await page.getByText('1.0.1', { exact: true }).waitFor();
console.log('UI_GATE reinstall_launch_persistence=PASS');
await browser.close();
