import { expect, type Page } from '@playwright/test';

export const ADMIN_KEY = process.env.SVEDA_ADMIN_API_KEY ?? 'sveda-e2e-admin-key';

export async function signIn(page: Page) {
  await page.addInitScript(() => {
    window.localStorage.setItem('locale', 'en');
  });
  await page.goto('/admin');
  await expect(page.locator('#key')).toBeVisible();
  await page.locator('#key').fill(ADMIN_KEY);
  await page.getByRole('button', { name: 'Open settings' }).click();
  await expect(page.getByRole('heading', { name: 'Dashboard.' })).toBeVisible();
}

export async function openAdmin(page: Page, path: string, heading: string) {
  await page.goto(path);
  await expect(page.getByRole('heading', { name: heading })).toBeVisible();
}
