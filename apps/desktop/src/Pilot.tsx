import React,{useEffect,useRef,useState} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {Monitor,ShieldCheck,Copy,Link,Square,Check,LoaderCircle} from 'lucide-react';
type Room={id:string;role:'host'|'viewer';state:string;viewerName:string;expiresAt:number;offer?:RTCSessionDescriptionInit;ice?:RTCConfiguration;leaseSeconds?:number;security?:{version:number;inviteExpiresAt:number;events:{at:number;kind:string}[]}};
type Config={serverUrl:string;canHost:boolean;platform:string};
const labels:Record<string,string>={waiting:'Karşı cihazın katılması bekleniyor',awaiting_consent:'Ekranı paylaşan kişinin onayı bekleniyor',negotiating:'Şifreli görüntü bağlantısı kuruluyor',active:'Oturum yetkisi açık',ended:'Oturum sona erdi'};
const securityLabels:Record<string,string>={invitation_created:'Tek kullanımlık davet oluşturuldu',viewer_joined:'Davet kullanıldı; yerel onay bekleniyor',host_accepted:'Ekranı paylaşan kişi izin verdi',session_active:'Süreli ekran izleme yetkisi verildi',session_ended:'Oturum sonlandırıldı',heartbeat_lost:'Bağlantı kesildi; oturum kapatıldı',invitation_expired:'Davet süresi doldu'};
export function Pilot(){
 const [serverStatus,setServerStatus]=useState('');
 const [config,setConfig]=useState<Config>();const [url,setUrl]=useState('');const [invite,setInvite]=useState('');const [name,setName]=useState('Bu bilgisayar');
 const [room,setRoom]=useState<Room>();const [ownInvite,setOwnInvite]=useState('');const [error,setError]=useState('');const [busy,setBusy]=useState(false);
 const [videoState,setVideoState]=useState('Görüntü bekleniyor');const [received,setReceived]=useState(false);const [copied,setCopied]=useState(false);const [needsPlay,setNeedsPlay]=useState(false);
 const video=useRef<HTMLVideoElement>(null);const peer=useRef<RTCPeerConnection|undefined>(undefined);const negotiating=useRef(false);const generation=useRef(0);const active=useRef(false);const lastLease=useRef(0);const media=useRef<MediaStream|undefined>(undefined);
 const native=isTauri();
 function closeVideo(){generation.current++;peer.current?.close();peer.current=undefined;negotiating.current=false;media.current?.getTracks().forEach(t=>t.stop());media.current=undefined;if(video.current)video.current.srcObject=null;setReceived(false);setNeedsPlay(false)}
 async function stop(){active.current=false;lastLease.current=0;closeVideo();setVideoState('Görüntü bekleniyor');setBusy(true);try{await invoke('pilot_stop')}catch(e){setError(String(e))}finally{setRoom(undefined);setOwnInvite('');setBusy(false)}}
 useEffect(()=>{
  if(!native)return;
  void invoke<Config>('pilot_config').then(c=>{setConfig(c);setUrl(localStorage.getItem('dengex.serverUrl')||c.serverUrl);setName(c.platform==='macos'?'Mac cihazı':'Windows cihazı')}).catch(e=>setError(String(e)));
  const unlisten=listen<string>('pilot-error',e=>setError(e.payload));
  return()=>{active.current=false;closeVideo();void invoke('pilot_stop');void unlisten.then(f=>f())};
 },[]);
 async function viewerOffer(offer:RTCSessionDescriptionInit,ice?:RTCConfiguration){
  if(negotiating.current||peer.current||!active.current)return;
  negotiating.current=true;const current=++generation.current;
  try{
   if(typeof RTCPeerConnection==='undefined')throw new Error('Bu WebView WebRTC desteklemiyor. Güncel Windows WebView2 gerekir.');
   const pc=new RTCPeerConnection(ice??{iceServers:[{urls:'stun:stun.cloudflare.com:3478'}]});peer.current=pc;
   pc.ontrack=e=>{if(current!==generation.current)return;media.current=e.streams[0]??new MediaStream([e.track]);if(video.current){video.current.srcObject=media.current;void video.current.play().catch(()=>{setNeedsPlay(true);setVideoState('Görüntüyü aç düğmesine basın.')})}};
   pc.onconnectionstatechange=()=>{if(current!==generation.current)return;if(pc.connectionState==='failed'){setError('Görüntü bağlantısı kurulamadı. Sunucunun TURN ayarlarını ve ağın UDP erişimini kontrol edin. HTTPS tüneli ekran trafiğini taşımaz.');void stop()}else if(pc.connectionState==='connected'){setVideoState('Görüntü kanalı bağlandı; ilk kare bekleniyor')}};
   await pc.setRemoteDescription(offer);await pc.setLocalDescription(await pc.createAnswer());
   if(pc.iceGatheringState!=='complete')await new Promise<void>((resolve,reject)=>{const timeout=setTimeout(()=>{pc.removeEventListener('icegatheringstatechange',change);reject(new Error('ICE adayları zamanında toplanamadı.'))},20000);function change(){if(pc.iceGatheringState==='complete'){clearTimeout(timeout);pc.removeEventListener('icegatheringstatechange',change);resolve()}}pc.addEventListener('icegatheringstatechange',change);change()});
   if(!active.current||current!==generation.current){pc.close();return}
   const r=await invoke<Room>('pilot_answer',{description:pc.localDescription?.toJSON()});lastLease.current=performance.now()+(r.leaseSeconds??0)*1000;setRoom(r);
  }catch(e){setError(String(e));void stop()}finally{negotiating.current=false}
 }
 useEffect(()=>{
  if(!room||room.state==='ended')return;
  let cancelled=false;let polling=false;
  const poll=async()=>{
   if(polling||!active.current)return;polling=true;
   try{
    const next=await invoke<Room>('pilot_poll');if(cancelled)return;
    if(next.state==='ended'){active.current=false;closeVideo();setRoom(next);return}
    if(next.role==='viewer'&&next.state==='active')lastLease.current=performance.now()+(next.leaseSeconds??0)*1000;
    setRoom(next);if(next.role==='viewer'&&next.offer)void viewerOffer(next.offer,next.ice);
   }catch(e){if(!cancelled)setError(String(e))}finally{polling=false}
  };
  const timer=setInterval(()=>void poll(),1500);
  const lease=setInterval(()=>{if(lastLease.current&&performance.now()>lastLease.current&&active.current){setError('Oturum yetkisi yenilenemedi; görüntü kapatıldı.');void stop()}},500);
  void poll();return()=>{cancelled=true;clearInterval(timer);clearInterval(lease)};
 },[room?.id,room?.state==='ended']);
 async function checkServer(){setError('');setServerStatus('');setBusy(true);try{const s=await invoke<{guestHosts:boolean;turn:boolean}>('pilot_check',{serverUrl:url});setServerStatus(`Sunucu hazır. ${s.guestHosts?'Davet oluşturma açık.':'Davet oluşturma yöneticiyle sınırlı.'} ${s.turn?'TURN etkin.':'TURN yok; doğrudan bağlantı kullanılacak.'}`);localStorage.setItem('dengex.serverUrl',url.trim())}catch(e){setError(String(e))}finally{setBusy(false)}}
 async function create(){setError('');setBusy(true);setCopied(false);try{const v=await invoke<{room:Room;invite:string;serverUrl:string}>('pilot_create',{serverUrl:url});active.current=true;lastLease.current=0;setRoom(v.room);setOwnInvite(v.invite);setUrl(v.serverUrl);localStorage.setItem('dengex.serverUrl',v.serverUrl)}catch(e){setError(String(e))}finally{setBusy(false)}}
 async function join(){setError('');setBusy(true);try{const v=await invoke<Room>('pilot_join',{serverUrl:url,invite,name});setInvite('');active.current=true;lastLease.current=0;setRoom(v);localStorage.setItem('dengex.serverUrl',url.trim())}catch(e){setError(String(e))}finally{setBusy(false)}}
 async function accept(){setError('');setBusy(true);try{await invoke('pilot_accept')}catch(e){setError(String(e))}finally{setBusy(false)}}
 async function copy(){try{await navigator.clipboard.writeText(ownInvite);setCopied(true)}catch{setError('Kodu seçip kopyalayabilirsiniz.')}}
 return <>
  <div className="eyebrow">AÇIK KAYNAK · ONAYLI EKRAN PAYLAŞIMI · BETA</div><h1>Ekranınızı güvenle paylaşın.</h1><p className="lead">Bağlanılacak bilgisayarda davet oluşturun. İzleyecek bilgisayarda bu daveti girin. Paylaşım, ekranı açan kişinin kabulüyle başlar.</p>
  {error&&<div className="error" role="alert">{error}</div>}
  {!native&&<div className="info-notice">Bu ekran kurulu masaüstü uygulamasında çalışır.</div>}
  {!room&&<section className="small-card server-setup"><div className="section-title"><ShieldCheck size={20}/><h3>Bağlantı sunucusu</h3></div><p className="small-description">Her iki bilgisayarda aynı HTTPS adresini kullanın. Kendi dengeX sunucunuzun adresini girin; ortak bir sunucu pakete dahil değildir.</p><label className="pilot-field">Sunucu adresi<input value={url} onChange={e=>{setUrl(e.target.value);setServerStatus('')}} placeholder="https://destek.ornek.com" autoCapitalize="none" spellCheck={false}/></label><button className="secondary" disabled={!native||busy||!url.trim()} onClick={checkServer}>Sunucuyu kontrol et</button>{serverStatus&&<p role="status">{serverStatus}</p>}</section>}
  {!room?<div className="two-columns pilot-entry">
   <section className="small-card"><div className="section-title"><Monitor size={20}/><h3>Bu bilgisayarın ekranını paylaş</h3></div><p className="small-description">Bu bilgisayara bağlanılacaksa burada davet oluşturun. 5 dakika geçerli daveti karşı tarafa iletin; gelen isteği ayrıca kabul edin.</p><button className="primary" disabled={!config?.canHost||busy||!url.trim()} onClick={create}><Link size={16}/>Davet oluştur</button>{!config?.canHost&&<p className="field-hint">Ekran paylaşımı kurulu Windows veya macOS uygulamasında kullanılabilir.</p>}{config?.platform==='windows'&&<p className="field-hint">Windows: ilk monitör, en fazla 1280×720. Kilitli ekran ve uzak klavye/fare kontrolü desteklenmez.</p>}</section>
   <section className="small-card"><div className="section-title"><Link size={20}/><h3>Davet ile ekranı izle</h3></div><label className="pilot-field">Cihaz adı<input value={name} onChange={e=>setName(e.target.value)} maxLength={64}/></label><label className="pilot-field">Davet kodu<input aria-label="Davet kodu" value={invite} onChange={e=>setInvite(e.target.value)} autoCapitalize="none" spellCheck={false} autoComplete="off" type="password"/></label><button className="primary" disabled={!native||busy||!invite.trim()||!url||!name.trim()} onClick={join}>Bağlantı isteği gönder</button></section>
  </div>:<section className="small-card pilot-session">
   <div className="section-title"><ShieldCheck size={20}/><h3>{labels[room.state]??room.state}</h3><span className="closed">Yalnızca izle</span></div>
   {room.role==='host'&&room.state==='waiting'&&<><p className="small-description">Karşı bilgisayarda “Davet ile ekranı izle” bölümüne bu sunucu adresini ve daveti girin. Davet, ekranı sizin kabulünüz olmadan açmaz.</p><label className="pilot-field">Sunucu adresi<input readOnly value={url}/></label><label className="pilot-field">Tek kullanımlık davet<input readOnly value={ownInvite} onFocus={e=>e.target.select()}/></label><button className="secondary" onClick={copy}>{copied?<Check size={15}/>:<Copy size={15}/>}Daveti kopyala</button></>}
   {room.role==='host'&&room.state==='awaiting_consent'&&<div className="consent"><h2>{room.viewerName} ekranınızı görmek istiyor.</h2><p>Bu ad davetli tarafından yazıldı; personel hesabı/MFA doğrulanmadı. Yalnızca daveti verdiğiniz kişiyle deneme yapın.</p><p>İzin: ana ekranınızı görüntüleme. Klavye, fare, dosya ve pano erişimi verilmez.</p><div className="test-actions"><button className="primary" disabled={busy} onClick={accept}>Ekranımı paylaşmayı kabul et</button><button className="secondary" onClick={stop}>Reddet</button></div></div>}
   {room.role==='host'&&['negotiating','active'].includes(room.state)&&<div className="consent"><h2>Ekran paylaşımına izin verdiniz.</h2><p>{room.state==='active'?'Paylaşım yetkisi doğrulandı; görüntü akışı hazırlanıyor.':'Görüntü bağlantısı kuruluyor.'} Paylaşımı bu penceredeki Paylaşımı bitir düğmesiyle veya uygulamayı kapatarak durdurabilirsiniz. Klavye ve fare kontrolü verilmez.</p></div>}
   {room.role==='viewer'&&room.state!=='ended'&&<><div className="viewer"><video ref={video} autoPlay muted playsInline controls onPlaying={()=>{setReceived(true);setNeedsPlay(false);setVideoState('Canlı görüntü')}}/>{!received&&<div className="viewer-placeholder">{needsPlay?<button className="primary" onClick={()=>void video.current?.play().catch(()=>setError('Görüntü oynatılamadı. Oturumu yeniden başlatın.'))}>Görüntüyü aç</button>:<LoaderCircle size={25} className="spin"/>}<span>{room.state==='awaiting_consent'?'Karşı bilgisayardaki kullanıcının onayı bekleniyor':videoState}</span></div>}</div><p className="small-description">{videoState} · Görüntü kaydedilmez. Pilot yalnızca izleme destekler.</p></>}
   {room.security?.version===1&&<details className="security-panel"><summary><ShieldCheck size={16}/> Oturum güvenliği</summary><p>Tek kullanımlık davet · Yerel onay · En fazla 30 dakika · Bağlantı denemesi sınırı</p>{room.state==='waiting'&&<p>Davetin son kullanımı: {new Date(room.security.inviteExpiresAt*1000).toLocaleTimeString('tr-TR')}</p>}<ol>{room.security.events.map((event,i)=><li key={i}><time>{new Date(event.at*1000).toLocaleTimeString('tr-TR')}</time> {securityLabels[event.kind]??'Oturum durumu değişti'}</li>)}</ol><small>Yalnızca bu oturumun olayları gösterilir. Kayıtlar sunucu belleğinde tutulur; ekran içeriği kaydedilmez.</small></details>}
   <button className="secondary stop-pilot" disabled={busy} onClick={stop}><Square size={14}/>{room.state==='ended'?'Yeni bağlantı':'Paylaşımı bitir'}</button>
  </section>}
  <div className="info-notice"><ShieldCheck size={18}/><p>HTTPS sunucusu eşleştirme trafiğini taşır. Görüntü WebRTC üzerinden şifrelenir. Sunucuda TURN/UDP yapılandırıldığında doğrudan bağlantı kurulamayan ağlarda aktarım sunucusu kullanılır. UDP trafiğini engelleyen ağlar henüz desteklenmez. Uygulama veya sunucu kapanırsa oturum sona erer.</p></div>
 </>;
}
