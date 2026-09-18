import React,{useEffect,useRef,useState} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {Monitor,ShieldCheck,Copy,Link,Square,Check,LoaderCircle} from 'lucide-react';
type Room={id:string;role:'host'|'viewer';state:string;viewerName:string;expiresAt:number;offer?:RTCSessionDescriptionInit;leaseSeconds?:number};
type Config={serverUrl:string;canHost:boolean;platform:string};
const labels:Record<string,string>={waiting:'Windows cihazının katılması bekleniyor',awaiting_consent:'Ekranı paylaşan kişinin onayı bekleniyor',negotiating:'Şifreli görüntü bağlantısı kuruluyor',active:'Oturum yetkisi açık',ended:'Deneme sona erdi'};
export function Pilot(){
 const [config,setConfig]=useState<Config>();const [url,setUrl]=useState('');const [invite,setInvite]=useState('');const [name,setName]=useState('Windows cihazı');
 const [room,setRoom]=useState<Room>();const [ownInvite,setOwnInvite]=useState('');const [error,setError]=useState('');const [busy,setBusy]=useState(false);
 const [videoState,setVideoState]=useState('Görüntü bekleniyor');const [received,setReceived]=useState(false);const [copied,setCopied]=useState(false);
 const video=useRef<HTMLVideoElement>(null);const peer=useRef<RTCPeerConnection|undefined>(undefined);const negotiating=useRef(false);const generation=useRef(0);const active=useRef(false);const lastLease=useRef(0);const media=useRef<MediaStream|undefined>(undefined);
 const native=isTauri();
 function closeVideo(){generation.current++;peer.current?.close();peer.current=undefined;negotiating.current=false;media.current?.getTracks().forEach(t=>t.stop());media.current=undefined;if(video.current)video.current.srcObject=null;setReceived(false)}
 async function stop(){active.current=false;closeVideo();setBusy(true);try{await invoke('pilot_stop')}catch(e){setError(String(e))}finally{setRoom(undefined);setOwnInvite('');setBusy(false)}}
 useEffect(()=>{
  if(!native)return;
  void invoke<Config>('pilot_config').then(c=>{setConfig(c);setUrl(c.serverUrl)}).catch(e=>setError(String(e)));
  const unlisten=listen<string>('pilot-error',e=>setError(e.payload));
  return()=>{active.current=false;closeVideo();void invoke('pilot_stop');void unlisten.then(f=>f())};
 },[]);
 async function viewerOffer(offer:RTCSessionDescriptionInit){
  if(negotiating.current||peer.current||!active.current)return;
  negotiating.current=true;const current=++generation.current;
  try{
   if(typeof RTCPeerConnection==='undefined')throw new Error('Bu WebView WebRTC desteklemiyor. Güncel Windows WebView2 gerekir.');
   const pc=new RTCPeerConnection({iceServers:[{urls:'stun:stun.cloudflare.com:3478'}]});peer.current=pc;
   pc.ontrack=e=>{if(current!==generation.current)return;media.current=e.streams[0]??new MediaStream([e.track]);if(video.current){video.current.srcObject=media.current;void video.current.play().catch(()=>setVideoState('Görüntü için oynatma düğmesine basın.'))}};
   pc.onconnectionstatechange=()=>{if(current!==generation.current)return;if(pc.connectionState==='failed'){setError('Doğrudan görüntü bağlantısı kurulamadı. Bu ağ için TURN sunucusu gerekiyor; tünel bağlantısı ekran trafiğini aktarmıyor.');void stop()}else if(pc.connectionState==='connected'){setVideoState('Görüntü kanalı bağlandı; ilk kare bekleniyor')}};
   await pc.setRemoteDescription(offer);await pc.setLocalDescription(await pc.createAnswer());
   if(pc.iceGatheringState!=='complete')await new Promise<void>((resolve,reject)=>{const timeout=setTimeout(()=>{pc.removeEventListener('icegatheringstatechange',change);reject(new Error('ICE adayları zamanında toplanamadı.'))},20000);function change(){if(pc.iceGatheringState==='complete'){clearTimeout(timeout);pc.removeEventListener('icegatheringstatechange',change);resolve()}}pc.addEventListener('icegatheringstatechange',change);change()});
   if(!active.current||current!==generation.current){pc.close();return}
   const r=await invoke<Room>('pilot_answer',{description:pc.localDescription?.toJSON()});lastLease.current=performance.now()+(r.leaseSeconds??0)*1000;setRoom(r);
  }catch(e){setError(String(e));closeVideo()}finally{negotiating.current=false}
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
    setRoom(next);if(next.role==='viewer'&&next.offer)void viewerOffer(next.offer);
   }catch(e){if(!cancelled)setError(String(e))}finally{polling=false}
  };
  const timer=setInterval(()=>void poll(),1500);
  const lease=setInterval(()=>{if(lastLease.current&&performance.now()>lastLease.current&&active.current){setError('Oturum yetkisi yenilenemedi; görüntü kapatıldı.');void stop()}},500);
  void poll();return()=>{cancelled=true;clearInterval(timer);clearInterval(lease)};
 },[room?.id,room?.state==='ended']);
 async function create(){setError('');setBusy(true);try{const v=await invoke<{room:Room;invite:string;serverUrl:string}>('pilot_create');active.current=true;lastLease.current=0;setRoom(v.room);setOwnInvite(v.invite);setUrl(v.serverUrl)}catch(e){setError(String(e))}finally{setBusy(false)}}
 async function join(){setError('');setBusy(true);try{const v=await invoke<Room>('pilot_join',{serverUrl:url,invite,name});setInvite('');active.current=true;lastLease.current=0;setRoom(v)}catch(e){setError(String(e))}finally{setBusy(false)}}
 async function accept(){setError('');setBusy(true);try{await invoke('pilot_accept')}catch(e){setError(String(e))}finally{setBusy(false)}}
 async function copy(){try{await navigator.clipboard.writeText(ownInvite);setCopied(true)}catch{setError('Kodu seçip kopyalayabilirsiniz.')}}
 return <>
  <div className="eyebrow">İKİ CİHAZ · DAVETLİ DENEME</div><h1>Birlikte bağlanalım.</h1><p className="lead">Windows cihazında bu Mac’in ekranını açın. Her oturumda Mac’te onay gerekir.</p>
  {error&&<div className="error" role="alert">{error}</div>}
  {!native&&<div className="info-notice">Bu ekran kurulu masaüstü uygulamasında çalışır.</div>}
  {!room?<div className="two-columns pilot-entry">
   <section className="small-card"><div className="section-title"><Monitor size={20}/><h3>Bu Mac’in ekranını paylaş</h3></div><p className="small-description">30 dakika geçerli, tek kullanımlık bir davet oluşturun. Davetli katıldığında paylaşımı ayrıca onaylayın.</p><button className="primary" disabled={!config?.canHost||busy} onClick={create}><Link size={16}/>Davet oluştur</button>{!config?.canHost&&<p className="field-hint">Paylaşım bu pilotta sunucu yapılandırması olan Mac’ten başlatılır.</p>}</section>
   <section className="small-card"><div className="section-title"><Link size={20}/><h3>Davet ile ekranı izle</h3></div><label className="pilot-field">Sunucu adresi<input value={url} onChange={e=>setUrl(e.target.value)} placeholder="https://…trycloudflare.com" autoCapitalize="none" spellCheck={false}/></label><label className="pilot-field">Cihaz adı<input value={name} onChange={e=>setName(e.target.value)} maxLength={64}/></label><label className="pilot-field">Davet kodu<input value={invite} onChange={e=>setInvite(e.target.value)} autoCapitalize="none" spellCheck={false} autoComplete="off" type="password"/></label><button className="primary" disabled={!native||busy||!invite.trim()||!url||!name.trim()} onClick={join}>Bağlantı isteği gönder</button></section>
  </div>:<section className="small-card pilot-session">
   <div className="section-title"><ShieldCheck size={20}/><h3>{labels[room.state]??room.state}</h3><span className="closed">Yalnızca izle</span></div>
   {room.role==='host'&&room.state==='waiting'&&<><p className="small-description">Windows uygulamasına bu sunucu adresini ve daveti girin. Davet, ekranı sizin kabulünüz olmadan açmaz.</p><label className="pilot-field">Sunucu adresi<input readOnly value={url}/></label><label className="pilot-field">Tek kullanımlık davet<input readOnly value={ownInvite} onFocus={e=>e.target.select()}/></label><button className="secondary" onClick={copy}>{copied?<Check size={15}/>:<Copy size={15}/>}Daveti kopyala</button></>}
   {room.role==='host'&&room.state==='awaiting_consent'&&<div className="consent"><h2>{room.viewerName} ekranınızı görmek istiyor.</h2><p>Bu ad davetli tarafından yazıldı; personel hesabı/MFA doğrulanmadı. Yalnızca daveti verdiğiniz kişiyle deneme yapın.</p><p>İzin: ana ekranınızı görüntüleme. Klavye, fare, dosya ve pano erişimi verilmez.</p><div className="test-actions"><button className="primary" disabled={busy} onClick={accept}>Ekranımı paylaşmayı kabul et</button><button className="secondary" onClick={stop}>Reddet</button></div></div>}
   {room.role==='host'&&['negotiating','active'].includes(room.state)&&<p className="permission-notice">Görüntü bağlantısı ve oturum yetkisi doğrulanınca native paylaşım penceresi açılır. O penceredeki Durdur veya buradaki Denemeyi bitir düğmesi paylaşımı sonlandırır.</p>}
   {room.role==='viewer'&&room.state!=='ended'&&<><div className="viewer"><video ref={video} autoPlay muted playsInline controls onPlaying={()=>{setReceived(true);setVideoState('Canlı görüntü')}}/>{!received&&<div className="viewer-placeholder"><LoaderCircle size={25} className="spin"/><span>{room.state==='awaiting_consent'?'Mac kullanıcısının onayı bekleniyor':videoState}</span></div>}</div><p className="small-description">{videoState} · Görüntü kaydedilmez. İlk pilot yalnızca izleme destekler.</p></>}
   <button className="secondary stop-pilot" disabled={busy} onClick={stop}><Square size={14}/>{room.state==='ended'?'Yeni denemeye dön':'Denemeyi bitir'}</button>
  </section>}
  <div className="info-notice"><ShieldCheck size={18}/><p>Cloudflare Tunnel eşleştirme trafiğini taşır. Görüntü WebRTC üzerinden şifrelenir. Bu yapılandırmada TURN yok; doğrudan bağlantının engellendiği ağlarda görüntü açılamaz. Uygulama veya sunucu kapanırsa deneme sona erer.</p></div>
 </>;
}
