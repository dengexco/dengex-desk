import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { Check, ChevronRight, CircleHelp, Command, FlaskConical, Monitor, Moon, RefreshCw, ShieldCheck, Sun, X, Zap } from 'lucide-react';
import type { NativeStatus, ProbeReport } from '@dengex/protocol';
import { tr as t } from './i18n';
import './styles.css';
import {Pilot} from './Pilot';
const native=isTauri();
type Tab='permissions'|'lab'|'pilot';
type Probe='transport'|'synthetic'|'capture'|'input';
function App(){
 const [tab,setTab]=useState<Tab>('pilot');
 const [dark,setDark]=useState(false);
 const [status,setStatus]=useState<NativeStatus>();
 const [error,setError]=useState('');
 const [permissionBusy,setPermissionBusy]=useState(false);
 const [permissionNotice,setPermissionNotice]=useState('');
 const [busy,setBusy]=useState<Probe>();
 const [reports,setReports]=useState<Partial<Record<Probe,ProbeReport>>>({});
 async function refresh(){
  if(!native)return;
  try{setStatus(await invoke<NativeStatus>('native_status'))}catch(e){setError(String(e))}
 }
 useEffect(()=>{
  void refresh();
  const onFocus=()=>{void refresh()};
  window.addEventListener('focus',onFocus);
  return ()=>window.removeEventListener('focus',onFocus);
 },[]);
 useEffect(()=>{document.documentElement.dataset.theme=dark?'dark':'light'},[dark]);
 async function requestPermission(permission:'screen'|'input'){
  setError('');setPermissionBusy(true);
  setPermissionNotice(`Sistem Ayarları'nda dengeX Remote Lab için ${permission==='screen'?'Ekran ve Sistem Sesi Kaydı':'Erişilebilirlik'} iznini açın. macOS isterse uygulamayı kapatıp yeniden açın.`);
  try{await invoke('request_permission',{permission})}catch(e){setError(String(e))}
  finally{setPermissionBusy(false);void refresh()}
 }
 async function run(kind:Probe){
  setError('');setBusy(kind);
  setReports(p=>{const next={...p};delete next[kind];return next});
  try{const r=await invoke<ProbeReport>('run_probe',{kind});setReports(p=>({...p,[kind]:r}))}
  catch(e){setError(String(e))}
  finally{setBusy(undefined);void refresh()}
 }
 function reason(report:ProbeReport){
  const messages:Record<string,string>={
   screen_recording_permission_required:'Ekran Kaydı izni kapalı. “Ekran iznini aç” düğmesiyle Sistem Ayarları’na gidip dengeX Remote Lab için izni açın; ardından testi tekrar çalıştırın.',
   accessibility_permission_required:'Klavye ve fare testi için Erişilebilirlik izni gerekli. Sistem Ayarları’nda dengeX Remote Lab için izni açın.',
   cancelled_before_start:'Test başlamadan pencere kapatıldı.',
   no_decoded_frames:'Ekrandan çözülebilen görüntü alınamadı. İzinleri kontrol edip testi tekrar çalıştırın.',
   probe_process_failed:'Test süreci beklenmedik biçimde kapandı.',
  };
  return messages[report.reason??'']??`Test tamamlanamadı: ${report.reason??'Native bileşen sonuç üretemedi.'}`;
 }
 const permission=(value?:boolean)=>value===undefined?t.unknown:value?t.allowed:t.required;
 const tests:[Probe,string,string][]=[['transport',t.transport,t.transportHint],['synthetic',t.synthetic,t.syntheticHint],['capture',t.capture,t.captureHint],['input',t.inputTest,t.inputHint]];
 return <div className="shell">
  <aside className="sidebar">
   <a className="brand" href="#" aria-label="dengeX Remote ana sayfa" onClick={()=>setTab('pilot')}>denge<span>X</span><small>REMOTE</small></a>
   <div className="workspace"><span className="workspace-icon"><Command size={18}/></span><div>Bu bilgisayar<small>Kişisel çalışma alanı</small></div><span className="dot"/></div>
   <div className="nav-label">ÇALIŞMA ALANI</div>
   <nav aria-label="Ana gezinme">{([['pilot',Monitor,'Ekran paylaşımı'],['permissions',ShieldCheck,t.permissions],['lab',FlaskConical,t.lab]] as const).map(([id,Icon,label])=><button key={id} className={tab===id?'nav active':'nav'} onClick={()=>setTab(id)} aria-current={tab===id?'page':undefined}><Icon size={19}/>{label}</button>)}</nav>
   <div className="sidebar-bottom"><div className="private-note"><ShieldCheck size={20}/><p>Görünür ve izinli destek<small>Erişim her zaman sizin kontrolünüzde.</small></p></div><div className="build"><span className="build-dot"/>0.2.0 · Beta<button className="icon-button" onClick={()=>setDark(!dark)} aria-label={t.theme}>{dark?<Sun size={16}/>:<Moon size={16}/>}</button></div></div>
  </aside>
  <div className="main-wrap">
   <header><div className="breadcrumb">Çalışma alanı <ChevronRight size={13}/><strong>{tab==='permissions'?t.permissions:tab==='pilot'?'Ekran paylaşımı':t.lab}</strong></div><span className="dev-badge"><span/>{t.development}</span></header>
   <main>
   {error&&<div className="error" role="alert"><X size={18}/>{error}<button className="icon-button" onClick={()=>setError('')} aria-label="Hata mesajını kapat"><X size={16}/></button></div>}
   <div hidden={tab!=='pilot'}><Pilot/></div>
   {tab==='pilot'?null:tab==='permissions'?<>
    <div className="eyebrow">SİSTEM İZİNLERİ</div><h1>Kontrol sizde.</h1><p className="lead">Sistem izni, bir teknisyene bağlantı izni vermez.</p>
    {status?.unsupported?<div className="info-notice">Windows ekran paylaşımı açık bir kullanıcı masaüstünde çalışır. Buradaki macOS izin ayarları Windows için gerekli değildir.</div>:<section className="small-card permission-detail">
     {([{id:'screen',Icon:Monitor,label:t.screen,value:status?.screenRecording,help:'macOS → Sistem Ayarları → Gizlilik ve Güvenlik → Ekran ve Sistem Sesi Kaydı'},
        {id:'input',Icon:Command,label:t.input,value:status?.accessibility,help:'macOS → Sistem Ayarları → Gizlilik ve Güvenlik → Erişilebilirlik'}] as const).map(({id,Icon,label,value,help})=>
      <div className="permission-item" key={id}><Icon size={24}/><div><h3>{label}</h3><p>{help}</p><button className="secondary permission-action" onClick={()=>requestPermission(id)} disabled={!native||permissionBusy||!!busy}>{value?'Sistem ayarlarını aç':id==='screen'?'Ekran iznini aç':'Giriş iznini aç'}</button></div><span className={value?'permission-ok':'muted'}>{permission(value)}</span></div>)}
     <button className="secondary" onClick={refresh} disabled={!native}><RefreshCw size={16}/>{t.refresh}</button>
    </section>}
    {permissionNotice&&<p className="permission-notice" role="status">{permissionNotice}</p>}
    <p className="small-description">İzin değiştikten sonra macOS yeniden başlatma isterse uygulamayı kapatıp açın. Sistem ayarlarından geri döndüğünüzde izin durumu tekrar kontrol edilir.</p>
   </>:<>
    <div className="eyebrow">GELİŞTİRİCİ ARAÇLARI · YEREL TESTLER</div><h1>{t.testTitle}</h1><p className="lead">{t.testHint}</p>
    {!native&&<div className="info-notice"><CircleHelp size={18}/><p>{t.nativeOnly}</p></div>}
    {permissionNotice&&<p className="permission-notice" role="status">{permissionNotice}</p>}
    <div className="tests">{tests.map(([id,title,hint],i)=>{
     const report=reports[id];
     const needsPermission=id==='capture'?status?.screenRecording===false:id==='input'?status?.accessibility===false:false;
     return <section key={id} className="test-card">
      <div className="test-heading"><span className="test-number">0{i+1}</span><h3>{title}</h3>{report&&<span className={report.result==='passed'?'result pass':report.result==='blocked'?'result blocked':'result fail'}>{report.result==='passed'?t.passed:report.result==='blocked'?'İzin gerekli':t.failed}</span>}</div>
      <p>{hint}</p>
      <div className="test-actions"><button className="secondary" onClick={()=>run(id)} disabled={!native||!!busy||permissionBusy||(id!=='transport'&&status?.unsupported===true)}>{busy===id?<RefreshCw size={15} className="spin"/>:<Zap size={15}/>} {busy===id?t.running:id==='capture'?'Ekranımı 15 sn test et':t.start}</button>
      {needsPermission&&<button className="secondary" disabled={!native||permissionBusy||!!busy} onClick={()=>requestPermission(id==='capture'?'screen':'input')}><ShieldCheck size={15}/>{id==='capture'?'Ekran iznini aç':'Giriş iznini aç'}</button>}</div>
      {busy===id&&<p className="test-progress" role="status">{id==='capture'?'Görüntü, açılan native pencerede gösterilir. Durdur düğmesi testi bitirir.':'Native bileşen çalışıyor; sonuç burada gösterilecek.'}</p>}
      {report&&report.result!=='passed'&&<p className="test-reason" role="alert">{reason(report)}</p>}
      {report&&<div className="metrics">{report.handshakeAndEchoMs!==undefined&&<span>Bağlantı + yanıt <b>{report.handshakeAndEchoMs} ms</b></span>}{report.native&&<><span>Çözülen kare <b>{report.native.decodedFrames}</b></span><span>Boyut <b>{report.native.width} × {report.native.height}</b></span></>}{report.unicodeTextVerified&&<span><Check size={13}/> Türkçe metin doğrulandı</span>}{report.mouseClickVerified&&<span><Check size={13}/> Yerel fare tıklaması doğrulandı</span>}</div>}
     </section>;
    })}</div><p className="small-description">“Ekranımı 15 sn test et” düğmesi yalnızca bu bilgisayarda ekran yakalamayı başlatır. Test penceresi ve Durdur düğmesi görünür kalır. Ekran görüntüsü dosyaya kaydedilmez.</p>
   </>}
   <footer><span><ShieldCheck size={14}/> Yerel onay · Ayrı izinler · Görünür oturum</span><span>dengeX <span className="footer-dot">·</span> 2026</span></footer>
   </main>
  </div>
 </div>
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><App/></React.StrictMode>);
