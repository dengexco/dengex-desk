import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { ArrowUpRight, Check, ChevronRight, CircleHelp, Command, FlaskConical, Headphones, KeyRound, Laptop, LockKeyhole, Monitor, Moon, RefreshCw, ShieldCheck, Sun, X, Zap } from 'lucide-react';
import type { DeviceIdentity, NativeStatus, ProbeReport } from '@dengex/protocol';
import { tr as t } from './i18n';
import './styles.css';
import {Pilot} from './Pilot';
const native=isTauri();
type Tab='support'|'permissions'|'lab'|'pilot';
type Probe='transport'|'synthetic'|'capture'|'input';
function App(){
 const [tab,setTab]=useState<Tab>('support');
 const [dark,setDark]=useState(false);
 const [status,setStatus]=useState<NativeStatus>();
 const [error,setError]=useState('');
 const [identity,setIdentity]=useState<DeviceIdentity>();
 const [unregisteredWindows,setUnregisteredWindows]=useState(false);
 const [identityError,setIdentityError]=useState('');
 const [permissionBusy,setPermissionBusy]=useState(false);
 const [permissionNotice,setPermissionNotice]=useState('');
 const [busy,setBusy]=useState<Probe>();
 const [reports,setReports]=useState<Partial<Record<Probe,ProbeReport>>>({});
 async function refresh(){
  if(!native)return;
  try{setStatus(await invoke<NativeStatus>('native_status'))}catch(e){setError(String(e))}
 }
 async function loadIdentity(){
  if(!native)return;
  setIdentityError('');
  try{const value=await invoke<DeviceIdentity & {unregisteredWindows?:boolean}>('device_identity');if(value.unregisteredWindows){setUnregisteredWindows(true)}else{setIdentity(value)}}catch(e){setIdentityError(String(e))}
 }
 useEffect(()=>{
  void refresh();void loadIdentity();
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
   <a className="brand" href="#" aria-label="dengeX Remote ana sayfa" onClick={()=>setTab('support')}>denge<span>X</span><small>REMOTE</small></a>
   <div className="workspace"><span className="workspace-icon"><Command size={18}/></span><div>Bu bilgisayar<small>Kişisel çalışma alanı</small></div><span className="dot"/></div>
   <div className="nav-label">ÇALIŞMA ALANI</div>
   <nav aria-label="Ana gezinme">{([['support',Headphones,t.support],['permissions',ShieldCheck,t.permissions],['pilot',Monitor,'Cihaza bağlan'],['lab',FlaskConical,t.lab]] as const).map(([id,Icon,label])=><button key={id} className={tab===id?'nav active':'nav'} onClick={()=>setTab(id)} aria-current={tab===id?'page':undefined}><Icon size={19}/>{label}{id==='lab'&&<span className="mini">0</span>}</button>)}<button className="nav" disabled title="Hesap ve yetkilendirme entegrasyonu bekleniyor"><Laptop size={19}/>{t.technician}<LockKeyhole size={13} className="trailing"/></button></nav>
   <div className="sidebar-bottom"><div className="private-note"><ShieldCheck size={20}/><p>Görünür ve izinli destek<small>Erişim her zaman sizin kontrolünüzde.</small></p></div><div className="build"><span className="build-dot"/>0.1.0 · Native lab<button className="icon-button" onClick={()=>setDark(!dark)} aria-label={t.theme}>{dark?<Sun size={16}/>:<Moon size={16}/>}</button></div></div>
  </aside>
  <div className="main-wrap">
   <header><div className="breadcrumb">Çalışma alanı <ChevronRight size={13}/><strong>{tab==='support'?t.support:tab==='permissions'?t.permissions:tab==='pilot'?'Cihaza bağlan':t.lab}</strong></div><span className="dev-badge"><span/>{t.development}</span></header>
   <main>
   {error&&<div className="error" role="alert"><X size={18}/>{error}<button className="icon-button" onClick={()=>setError('')} aria-label="Hata mesajını kapat"><X size={16}/></button></div>}
   {tab==='pilot'?<Pilot/>:tab==='support'?<>
    <div className="eyebrow"><span className="line"/> DENGEX REMOTE</div><h1>{t.product}</h1><p className="lead">{t.subtitle}</p>
    <section className="support-card"><div className="card-top"><span className="card-icon"><Monitor size={23}/></span><div><h2>Birlikte çözelim.</h2><p>Bu bilgisayarın kimliği ve bağlantı durumu.</p></div><span className="offline"><span/>{t.notConnected}</span></div>
     <div className="identity-grid"><div><label>{t.identity}</label><div className={identity?"device-id":"placeholder-id"}>{identity?.supportId??(unregisteredWindows?"Windows pilot cihazı":identityError?"Kimlik okunamadı":"Oluşturuluyor…")}</div><span className="field-hint">{identity?`${identity.deviceName}${identity.storage==='none'?'':' · Bu cihazda kayıtlı'}`:unregisteredWindows?"Bağlantı için Cihaza bağlan bölümünden davet oluşturun":native?"Cihazın anahtar kasası kontrol ediliyor":"Native uygulamada oluşturulur"}</span></div><div><label>{t.code}</label><div className="unavailable-code">Henüz kullanılamıyor</div><span className="field-hint"><LockKeyhole size={12}/>{t.pendingCode}</span></div></div>
     <div className="support-footer"><div><LockKeyhole size={16}/><span>Her bağlantı için açık onayınız gerekir.</span></div><button className="primary" onClick={()=>setTab('pilot')}>İki cihazla dene <ArrowUpRight size={16}/></button></div>
    </section>
    {identityError&&<div className="error" role="alert"><p>{identityError}</p><button className="secondary" onClick={loadIdentity}>Tekrar dene</button></div>}
    <div className="info-notice"><CircleHelp size={18}/><p>{unregisteredWindows?"Bu Windows ekranını paylaşmak için Cihaza bağlan → Davet oluştur yolunu kullanın. Kalıcı cihaz kimliği kaydı bu pilotta henüz yoktur.":t.supportUnavailable}</p></div>
    <div className="two-columns"><section className="small-card"><div className="section-title"><ShieldCheck size={20}/><h3>Paylaşım izinleri</h3><button onClick={()=>setTab('permissions')} className="text-button">Yönet <ChevronRight size={14}/></button></div><div className="permission-row"><span><Monitor size={16}/>{t.screen}</span><span className={status?.screenRecording?'permission-ok':'muted'}>{permission(status?.screenRecording)}</span></div><div className="permission-row"><span><Command size={16}/>{t.input}</span><span className={status?.accessibility?'permission-ok':'muted'}>{permission(status?.accessibility)}</span></div></section>
    <section className="small-card"><div className="section-title"><KeyRound size={20}/><h3>{t.permanent}</h3><span className="closed">{t.disabled}</span></div><p className="small-description">Bu cihaza yalnızca sizin onayınızla erişim sağlanır. Katılımsız erişim henüz kullanılabilir değil.</p><div className="subtle-note"><LockKeyhole size={13}/> Kurulum, kalıcı erişimi etkinleştirmez.</div></section></div>
    <button className="lab-banner" onClick={()=>setTab('lab')}><span className="lab-symbol"><FlaskConical size={22}/></span><span><strong>Native motoru yakından inceleyin</strong><small>İzinler, video işleme ve WebRTC için gerçek yerel testler.</small></span><ArrowUpRight size={20}/></button>
   </>:tab==='permissions'?<>
    <div className="eyebrow">SİSTEM İZİNLERİ</div><h1>Kontrol sizde.</h1><p className="lead">Sistem izni, bir teknisyene bağlantı izni vermez.</p>
    <section className="small-card permission-detail">
     {([{id:'screen',Icon:Monitor,label:t.screen,value:status?.screenRecording,help:'macOS → Sistem Ayarları → Gizlilik ve Güvenlik → Ekran ve Sistem Sesi Kaydı'},
        {id:'input',Icon:Command,label:t.input,value:status?.accessibility,help:'macOS → Sistem Ayarları → Gizlilik ve Güvenlik → Erişilebilirlik'}] as const).map(({id,Icon,label,value,help})=>
      <div className="permission-item" key={id}><Icon size={24}/><div><h3>{label}</h3><p>{help}</p><button className="secondary permission-action" onClick={()=>requestPermission(id)} disabled={!native||permissionBusy||!!busy}>{value?'Sistem ayarlarını aç':id==='screen'?'Ekran iznini aç':'Giriş iznini aç'}</button></div><span className={value?'permission-ok':'muted'}>{permission(value)}</span></div>)}
     <button className="secondary" onClick={refresh} disabled={!native}><RefreshCw size={16}/>{t.refresh}</button>
    </section>
    {permissionNotice&&<p className="permission-notice" role="status">{permissionNotice}</p>}
    <p className="small-description">İzin değiştikten sonra macOS yeniden başlatma isterse uygulamayı kapatıp açın. Sistem ayarlarından geri döndüğünüzde izin durumu tekrar kontrol edilir.</p>
   </>:<>
    <div className="eyebrow">AŞAMA 0 · YEREL DENEYLER</div><h1>{t.testTitle}</h1><p className="lead">{t.testHint}</p>
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
