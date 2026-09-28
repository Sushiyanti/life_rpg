import { useEffect, useMemo, useState, type FormEvent } from 'react';
import type { CoreClient } from '../../domain/ipc';
import type { Concept, Effect, EffectHistoryEntry, EffectType, Player, QuestSession } from '../../domain/world';
import { effectLifecycleAt } from './effectLifecycle';
import './EffectsScreen.css';

type Props = {
  client: CoreClient; player: Player; effects: Effect[]; concepts: Concept[]; sessions: QuestSession[];
  isVisible: (kind: string, id: string) => boolean;
  onVisibility: (kind: string, id: string) => Promise<void>;
  showHidden: boolean; onShowHidden: () => void; onRefresh: () => Promise<void>;
};
type Draft = { typeCode: string; name: string; description: string; targetConceptId: string; intensity: string; startedAt: string; expiryMode: 'never'|'at'; expiresAt: string };
const newDraft = (): Draft => ({ typeCode: 'buff', name: '', description: '', targetConceptId: '', intensity: '1', startedAt: '', expiryMode: 'never', expiresAt: '' });
const fmt = (value: string) => new Date(value).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
const toLocalInput = (value: string) => { const d = new Date(value); return new Date(d.getTime()-d.getTimezoneOffset()*60000).toISOString().slice(0,16); };
const fromLocalInput = (value: string) => value ? new Date(value).toISOString() : undefined;
const pretty = (value: string) => value.replaceAll('_',' ');

export function EffectsScreen({client,player,effects,concepts,sessions,isVisible,onVisibility,showHidden,onShowHidden,onRefresh}:Props) {
  const [types,setTypes]=useState<EffectType[]>([]); const [draft,setDraft]=useState<Draft>(newDraft());
  const [editing,setEditing]=useState<string|null>(null); const [historyId,setHistoryId]=useState<string|null>(null);
  const [history,setHistory]=useState<Record<string,EffectHistoryEntry[]>>({}); const [busy,setBusy]=useState(false);
  const [message,setMessage]=useState(''); const [error,setError]=useState(''); const [sessionContext,setSessionContext]=useState('');
  const [deactivationSession,setDeactivationSession]=useState('');
  const hidden=effects.filter(effect=>!isVisible('effect',effect.id)).length;
  const shown=effects.filter(effect=>showHidden||isVisible('effect',effect.id));
  const currentSessionOptions=sessions.filter(session=>session.status==='in_progress');
  const ordered=useMemo(()=>shown.slice().sort((a,b)=>Date.parse(b.startedAt)-Date.parse(a.startedAt)),[shown]);
  useEffect(()=>{let live=true;void client.listEffectTypes().then(rows=>{if(live){setTypes(rows);if(rows[0])setDraft(value=>({...value,typeCode:rows[0]!.code}));}}).catch(reason=>{if(live)setError(reason instanceof Error?reason.message:'Effect types could not be loaded.')});return()=>{live=false}},[client]);

  function startEdit(effect:Effect){setEditing(effect.id);setDraft({typeCode:effect.typeCode,name:effect.name,description:effect.description??'',targetConceptId:effect.targetConceptId??'',intensity:String(effect.intensity),startedAt:toLocalInput(effect.startedAt),expiryMode:effect.expiresAt?'at':'never',expiresAt:effect.expiresAt?toLocalInput(effect.expiresAt):''});setError('');setMessage('');}
  function cancelEdit(){setEditing(null);setDraft(newDraft());setSessionContext('');}
  async function submit(event:FormEvent){event.preventDefault();if(!draft.name.trim()||!draft.typeCode)return;setBusy(true);setError('');setMessage('');try{
    const value={typeCode:draft.typeCode,name:draft.name.trim(),description:draft.description.trim()||undefined,targetConceptId:draft.targetConceptId||null,intensity:Number(draft.intensity),startedAt:fromLocalInput(draft.startedAt),expiresAt:draft.expiryMode==='at'?fromLocalInput(draft.expiresAt):null};
    if(editing){await client.updateEffect(player.id,editing,value);setMessage('Effect details saved; the before/after state was recorded in history.');}
    else {await client.createEffect(player.id,value,sessionContext||undefined);setMessage(sessionContext?'Effect created and explicitly linked to this Session.':'Effect created. No expiry is recorded unless you chose one.');}
    await onRefresh();cancelEdit();
  }catch(reason){setError(reason instanceof Error?reason.message:'Effect could not be saved. Your form remains available.');}finally{setBusy(false)}}

  async function toggleHistory(effectId:string){if(historyId===effectId){setHistoryId(null);return;}setHistoryId(effectId);if(history[effectId])return;setBusy(true);setError('');try{const rows=await client.listEffectHistory(player.id,effectId);setHistory(value=>({...value,[effectId]:rows}));}catch(reason){setError(reason instanceof Error?reason.message:'Effect history could not be loaded.');}finally{setBusy(false)}}
  async function deactivate(effect:Effect){setBusy(true);setError('');setMessage('');try{await client.deactivateEffect(player.id,effect.id,deactivationSession||undefined);await onRefresh();setMessage(deactivationSession?'Effect manually deactivated; its Session context and history were recorded.':'Effect manually deactivated; time-based expiry remains distinct.');}catch(reason){setError(reason instanceof Error?reason.message:'Effect could not be manually deactivated.');}finally{setBusy(false)}}

  return <div className="workspace-page effect-manager">
    <header className="page-heading"><div><p className="eyebrow">PLAYER-MANAGED WORLD RECORDS</p><h2>Effects</h2><p>Choose a type, target, intensity and start. “Never” records no expiry; only your explicit action manually deactivates an Effect.</p></div></header>
    {message&&<p className="effect-feedback" role="status">{message}</p>}{error&&<p className="effect-feedback effect-feedback--error" role="alert">{error}</p>}
    <section className="surface-card effect-create-card"><div className="surface-card__heading"><div><p className="eyebrow">{editing?'EDIT RECORDED MEANING':'CREATE WORLD RECORD'}</p><h3>{editing?'Edit Effect':'New Effect'}</h3></div>{editing&&<button className="text-link" type="button" onClick={cancelEdit}>Cancel</button>}</div>
      <form className="effect-form" onSubmit={event=>void submit(event)}>
        <label>Type<select required value={draft.typeCode} onChange={event=>setDraft({...draft,typeCode:event.target.value})}>{types.map(type=><option key={type.code} value={type.code}>{type.label} · {type.code}</option>)}</select>{types.find(type=>type.code===draft.typeCode)?.description&&<small>{types.find(type=>type.code===draft.typeCode)?.description}</small>}</label>
        <label>Name<input required maxLength={512} value={draft.name} onChange={event=>setDraft({...draft,name:event.target.value})} placeholder="e.g. Focus before a presentation"/></label>
        <label>Description <span className="muted">optional</span><textarea maxLength={4000} rows={2} value={draft.description} onChange={event=>setDraft({...draft,description:event.target.value})} placeholder="What this Effect means to you"/></label>
        <label>Target<select value={draft.targetConceptId} onChange={event=>setDraft({...draft,targetConceptId:event.target.value})}><option value="">Player</option>{concepts.map(concept=><option key={concept.id} value={concept.id}>Concept · {concept.name}</option>)}</select><small>Effects target the Player or one of your existing Concepts.</small></label>
        <label>Intensity<input type="number" step="1" value={draft.intensity} onChange={event=>setDraft({...draft,intensity:event.target.value})}/></label>
        <label>Starts at <span className="muted">blank means now</span><input type="datetime-local" value={draft.startedAt} onChange={event=>setDraft({...draft,startedAt:event.target.value})}/></label>
        <label>Expiry<select value={draft.expiryMode} onChange={event=>setDraft({...draft,expiryMode:event.target.value as Draft['expiryMode']})}><option value="never">Never — no expiry recorded</option><option value="at">At a recorded time</option></select></label>
        {draft.expiryMode==='at'&&<label>Expires at<input required type="datetime-local" value={draft.expiresAt} onChange={event=>setDraft({...draft,expiresAt:event.target.value})}/></label>}
        {!editing&&<label>Session context <span className="muted">optional, explicit</span><select value={sessionContext} onChange={event=>setSessionContext(event.target.value)}><option value="">No Session association</option>{currentSessionOptions.map(session=><option key={session.id} value={session.id}>Session · {session.id.slice(0,12)}</option>)}</select><small>A Session link is made only if you select one.</small></label>}
        {editing&&effects.some(effect=>effect.id===editing&&effectLifecycleAt(effect)==='expired')&&<p className="muted" role="note">This Effect is expired. Descriptive edits keep it expired. Changing its recorded expiry to a future time is a deliberate lifecycle edit, recorded in history; its derived state will become active again without a separate automatic-reactivation event.</p>}
        {editing&&<label>Expiry changes are recorded separately from meaning changes.</label>}
        <div className="effect-form__actions"><button className="button button--primary" disabled={busy||types.length===0||!draft.name.trim()}>{busy?'Saving…':editing?'Save Effect':'Create Effect'}</button>{types.length===0&&<small>No active Effect types are registered.</small>}</div>
      </form>
    </section>
    <section className="effect-session-choice"><label>Optional Session for manual deactivation<select value={deactivationSession} onChange={event=>setDeactivationSession(event.target.value)}><option value="">No Session context</option>{currentSessionOptions.map(session=><option key={session.id} value={session.id}>Session · {session.id.slice(0,12)}</option>)}</select></label><span>Choosing a Session explicitly records the relationship; no active Effects are auto-attached.</span></section>
    {hidden>0&&<div className="hidden-awareness"><span>◌</span><p><strong>{hidden} hidden Effect{hidden===1?'':'s'}</strong> still exist in this world. Visibility changes do not delete records.</p><button className="text-link" onClick={onShowHidden}>{showHidden?'Hide hidden':'Show hidden'}</button></div>}
    {ordered.length===0?<div className="empty-card"><strong>No Effects in this view</strong><p>{effects.length===0?'Create one above. You may intentionally leave expiry set to Never.':'Hidden Effects remain stored; show them to inspect their lifecycle.'}</p></div>:<div className="effect-list">{ordered.map(effect=>{
      const lifecycle=effectLifecycleAt(effect); const visible=isVisible('effect',effect.id); const rows=history[effect.id]??[];
      return <article className="effect-card effect-manager__card" key={effect.id}><span className="effect-card__sigil" aria-hidden="true">✦</span><div className="effect-manager__main"><span className={`type-pill ${lifecycle==='active'?'type-pill--completed':''} effect-state effect-state--${lifecycle}`}>{pretty(lifecycle)}</span><h3>{effect.name}</h3><p>{effect.description||`${effect.typeCode} · intensity ${effect.intensity}`}</p><small>Target: {effect.targetKind==='concept'?`Concept · ${concepts.find(item=>item.id===effect.targetConceptId)?.name??'linked Concept'}`:'Player'} · intensity {effect.intensity}</small><small>Started {fmt(effect.startedAt)} · {effect.expiresAt?`Recorded expiry ${fmt(effect.expiresAt)}`:'No expiry recorded · indefinite/manual'}</small>{effect.deactivatedAt&&<small>{effect.deactivationSource==='rule'?'Deactivated by a Rule':'Manually deactivated'} {fmt(effect.deactivatedAt)}</small>}
        {historyId===effect.id&&<div className="effect-history"><strong>Effect history · {rows.length} recorded event{rows.length===1?'':'s'}</strong>{rows.length===0?<p className="muted">No lifecycle changes recorded yet.</p>:rows.map(row=><details className="effect-history__event" key={row.id}><summary><span>{pretty(row.eventKind)}</span><time>{fmt(row.recordedAt)}</time>{row.sessionId&&<small>Session {row.sessionId.slice(0,12)}</small>}</summary>{row.previousStateJson&&<div><b>Before</b><pre>{JSON.stringify(JSON.parse(row.previousStateJson),null,2)}</pre></div>}<div><b>Recorded state</b><pre>{JSON.stringify(JSON.parse(row.currentStateJson),null,2)}</pre></div></details>)}</div>}
      </div><div className="effect-card__actions"><button className="text-link" onClick={()=>void onVisibility('effect',effect.id)}>{visible?'Hide here':'Show here'}</button><button className="text-link" onClick={()=>void toggleHistory(effect.id)}>{historyId===effect.id?'Close history':'View history'}</button>{!effect.deactivatedAt&&<button className="text-link" onClick={()=>startEdit(effect)}>Edit</button>}{!effect.deactivatedAt&&lifecycle==='active'&&<button className="button button--small button--quiet" disabled={busy} onClick={()=>void deactivate(effect)}>Manually deactivate</button>}</div></article>;
      })}</div>}
  </div>;
}
