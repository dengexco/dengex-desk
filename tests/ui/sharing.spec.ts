import { test, expect } from '@playwright/test';

// These are UI lifecycle tests with mocked native IPC, not screen-capture tests.
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as any;
    w.isTauri = true;
    w.commands = [];
    w.mockRoom = { id: 'fixture-room', role: 'host', state: 'waiting', viewerName: '', expiresAt: Date.now()/1000+1800 };
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    w.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (command: string, args: any) => {
        w.commands.push(command);
        if (command === 'native_status') return { screenRecording: true, accessibility: false };
        if (command === 'pilot_config') return { serverUrl: '', platform: 'macos', canHost: true };
        if (command === 'pilot_check') return { guestHosts: true, turn: true };
        if (command === 'pilot_create') return { room: structuredClone(w.mockRoom), invite: 'fixture-invitation', serverUrl: args.serverUrl };
        if (command === 'pilot_poll') return structuredClone(w.mockRoom);
        if (command === 'pilot_accept') { w.mockRoom = {...w.mockRoom, state: 'negotiating'}; return; }
        if (command.startsWith('plugin:event|')) return 1;
      },
    };
  });
  await page.goto('/');
});

test('starts with the real sharing flow and remembers a checked server', async ({ page }) => {
  await expect(page.getByRole('heading', { name: 'Ekranınızı güvenle paylaşın.' })).toBeVisible();
  await expect(page.getByRole('button', {name:'Davet oluştur', exact:true})).toBeDisabled();
  await page.getByLabel('Sunucu adresi', {exact:true}).fill('https://support.example.com');
  await page.getByRole('button', {name:'Sunucuyu kontrol et'}).click();
  await expect(page.getByRole('status')).toContainText('TURN etkin');
  await page.reload();
  await expect(page.getByLabel('Sunucu adresi', {exact:true})).toHaveValue('https://support.example.com');
});

test('navigation preserves the room and explicit stop ends it', async ({ page }) => {
  await page.getByLabel('Sunucu adresi', {exact:true}).fill('https://support.example.com');
  await page.getByRole('button', {name:'Davet oluştur', exact:true}).click();
  await expect(page.getByText('Karşı cihazın katılması bekleniyor', {exact:true})).toBeVisible();
  const stops = await page.evaluate(() => (window as any).commands.filter((c: string)=>c==='pilot_stop').length);
  await page.getByRole('button', {name:'İzinler', exact:true}).click();
  await page.getByRole('button', {name:'Ekran paylaşımı', exact:true}).click();
  await expect(page.getByLabel('Tek kullanımlık davet')).toHaveValue('fixture-invitation');
  expect(await page.evaluate(() => (window as any).commands.filter((c: string)=>c==='pilot_stop').length)).toBe(stops);
  await page.getByRole('button', {name:'Paylaşımı bitir', exact:true}).click();
  await expect(page.getByRole('button', {name:'Davet oluştur', exact:true})).toBeVisible();
  expect(await page.evaluate(() => (window as any).commands.filter((c: string)=>c==='pilot_stop').length)).toBe(stops+1);
});

test('host must click consent after a viewer joins', async ({ page }) => {
  await page.getByLabel('Sunucu adresi', {exact:true}).fill('https://support.example.com');
  await page.getByRole('button', {name:'Davet oluştur', exact:true}).click();
  await page.evaluate(() => { (window as any).mockRoom.state='awaiting_consent'; (window as any).mockRoom.viewerName='Davetli'; });
  await expect(page.getByRole('heading', {name:'Davetli ekranınızı görmek istiyor.'})).toBeVisible();
  expect(await page.evaluate(() => (window as any).commands.includes('pilot_accept'))).toBe(false);
  await page.getByRole('button', {name:'Ekranımı paylaşmayı kabul et'}).click();
  await expect.poll(() => page.evaluate(() => (window as any).commands.includes('pilot_accept'))).toBe(true);
});
