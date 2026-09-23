import { expect, test } from '@playwright/test';
import { signIn } from './helpers/admin';

test.describe('admin runtime internet tools', () => {
  test('internet search is on by default and can be saved', async ({ page }) => {
    await signIn(page);
    await page.goto('/admin/runtime');
    await expect(page.getByRole('heading', { name: 'Runtime.' })).toBeVisible();

    const enabled = page.locator('input[name="web_enabled"]');
    await expect(enabled).toBeChecked();
    await expect(page.getByText('Search and fetch the public web')).toBeVisible();

    await enabled.uncheck();
    await page.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText('Saved.')).toBeVisible();

    await page.reload();
    await expect(page.getByRole('heading', { name: 'Runtime.' })).toBeVisible();
    await expect(page.locator('input[name="web_enabled"]')).not.toBeChecked();

    await page.locator('input[name="web_enabled"]').check();
    await page.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText('Saved.')).toBeVisible();
  });
});
