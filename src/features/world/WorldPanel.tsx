import {useState} from 'react';
import {coreClient} from '../../domain/ipc';
import type {Player, WorldOverview} from '../../domain/world';
import './WorldPanel.css';

/** Minimal developer-facing proof that the persistent Phase 2 world works end-to-end. */
export function WorldPanel(){
 const [name,setName]=useState(''); const [player,setPlayer]=useState<Player|null>(null); const [world,setWorld]=useState<WorldOverview|null>(null); const [message,setMessage]=useState(''); const [busy,setBusy]=useState(false);
 async function create(){if(!name.trim())return;setBusy(true);try{const p=await coreClient.createPlayer(name.trim());setPlayer(p);setWorld(await coreClient.getWorldOverview(p.id));setMessage(`Created ${p.name} in the local world.`)}catch(e){setMessage(e instanceof Error?e.message:'Failed to create player')}finally{setBusy(false)}}
 async function award(){if(!player)return;setBusy(true);try{const result=await coreClient.awardXp(player.id,25,'verification');setPlayer(result.player);setWorld(await coreClient.getWorldOverview(player.id));setMessage('Awarded +25 XP and wrote its transaction atomically.')}catch(e){setMessage(e instanceof Error?e.message:'Failed to award XP')}finally{setBusy(false)}}
 return <section className="world-panel" aria-label="Persistent world verification"><div><p className="world-panel__eyebrow">Phase 2 · persistent core</p><h2>World verification</h2><p className="world-panel__copy">Create one local player and exercise the Player + XP ledger seam. Advanced RPG presentation intentionally belongs to later phases.</p></div>{!player?<div className="world-panel__actions"><label>Player name<input value={name} onChange={e=>setName(e.target.value)} placeholder="Ada" disabled={busy}/></label><button onClick={create} disabled={busy||!name.trim()}>Create player</button></div>:<div className="world-panel__summary"><strong>{player.name}</strong><span>Level {player.level} · {player.currentXp} XP</span><button onClick={award} disabled={busy}>Award +25 XP</button>{world&&<small>{world.quests.length} quests · {world.skills.length} skills · {world.recentTransactions.length} ledger entries</small>}</div>}{message&&<p role="status" className="world-panel__message">{message}</p>}</section>;
}
