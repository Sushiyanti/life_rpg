import { useCallback, useEffect, useState } from 'react';
import { AppShell, type AppRoute } from './app/AppShell';
import { coreClient } from './domain/ipc';
import type { Concept, Player, PlayerStat, QuestSession, SearchHit, WorldOverview } from './domain/world';
import { RulePanel } from './features/world/RulePanel';
import { StatusScreen } from './features/status/StatusScreen';
import { WorldWorkspace } from './features/world/WorldWorkspace';

const ACTIVE_WORLD_KEY = 'life-rpg.active-world.v1';
const ACTIVE_ROUTE_KEY = 'life-rpg.active-route.v1';
const knownRoutes:AppRoute[]=['dashboard','player','quests','skills','skillTrees','concepts','effects','journal','explorer','history','rules','status'];
const emptyQuery = {text:null,kind:'player',playerId:null,conceptId:null,typeCode:null,status:null,active:null,from:null,through:null,context:null,includeHidden:true,includeArchived:true,includeTrashed:true,sort:'name' as const,limit:250,offset:0};

export function App() {
  const [route,setRoute]=useState<AppRoute>(()=>{try{const saved=localStorage.getItem(ACTIVE_ROUTE_KEY) as AppRoute|null;return saved&&knownRoutes.includes(saved)?saved:'dashboard'}catch{return 'dashboard'}});
  const [players,setPlayers]=useState<{id:string;name:string}[]>([]);
  const [player,setPlayer]=useState<Player|null>(null);
  const [overview,setOverview]=useState<WorldOverview|null>(null);
  const [concepts,setConcepts]=useState<Concept[]>([]);
  const [stats,setStats]=useState<PlayerStat[]>([]);
  const [sessions,setSessions]=useState<QuestSession[]>([]);
  const [healthReady,setHealthReady]=useState(false);
  const [loading,setLoading]=useState(true);
  const [error,setError]=useState('');
  const [notice,setNotice]=useState('');
  const [quickCreate,setQuickCreate]=useState(false);

  const refreshWorld=useCallback(async(id=player?.id)=>{
    if(!id)return;
    try{
      const [world,conceptRows,statRows,sessionRows]=await Promise.all([
        coreClient.getWorldOverview(id),coreClient.listConcepts(id),coreClient.listPlayerStats(id),coreClient.listQuestSessions(id),
      ]);
      setPlayer(world.player);setOverview(world);setConcepts(conceptRows);setStats(statRows);setSessions(sessionRows);setError('');
    }catch(e){setError(e instanceof Error?e.message:'The local world could not be loaded.');}
  },[player?.id]);

  useEffect(()=>{
    let live=true;
    async function boot(){
      setLoading(true);
      try{
        const [playerHits]=await Promise.all([coreClient.searchWorld(emptyQuery),coreClient.ping()]);
        if(!live)return;
        const choices=playerHits.filter((h:SearchHit)=>h.kind==='player').map(h=>({id:h.id,name:h.name}));
        setPlayers(choices);setHealthReady(true);
        const saved=localStorage.getItem(ACTIVE_WORLD_KEY);
        const selected=choices.find(p=>p.id===saved)??choices[0];
        if(selected){const loaded=await coreClient.getPlayer(selected.id);if(live&&loaded){localStorage.setItem(ACTIVE_WORLD_KEY,selected.id);setPlayer(loaded);const [world,conceptRows,statRows,sessionRows]=await Promise.all([coreClient.getWorldOverview(selected.id),coreClient.listConcepts(selected.id),coreClient.listPlayerStats(selected.id),coreClient.listQuestSessions(selected.id)]);if(live){setPlayer(world.player);setOverview(world);setConcepts(conceptRows);setStats(statRows);setSessions(sessionRows);}}}
      }catch(e){if(live){setHealthReady(false);setError(e instanceof Error?e.message:'The local world could not be reached.');}}
      finally{if(live)setLoading(false);}
    }
    void boot();return()=>{live=false;};
  },[]);

  const selectPlayer=useCallback(async(id:string)=>{
    if(!id)return;localStorage.setItem(ACTIVE_WORLD_KEY,id);setLoading(true);setError('');
    try{const [world,conceptRows,statRows,sessionRows]=await Promise.all([coreClient.getWorldOverview(id),coreClient.listConcepts(id),coreClient.listPlayerStats(id),coreClient.listQuestSessions(id)]);setPlayer(world.player);setOverview(world);setConcepts(conceptRows);setStats(statRows);setSessions(sessionRows);}
    catch(e){setError(e instanceof Error?e.message:'Could not switch world.');}
    finally{setLoading(false);}
  },[]);

  const createPlayer=useCallback(async(name:string,description?:string)=>{
    const created=await coreClient.createPlayer(name,description);localStorage.setItem(ACTIVE_WORLD_KEY,created.id);setPlayers(rows=>[...rows,{id:created.id,name:created.name}]);setPlayer(created);await refreshWorld(created.id);setNotice(`World created for ${created.name}.`);
  },[refreshWorld]);

  const afterMutation=useCallback(async(message?:string)=>{await refreshWorld();if(message)setNotice(message);},[refreshWorld]);
  const quickNavigate=(next:AppRoute)=>{setQuickCreate(false);setRoute(next);};
  useEffect(()=>{try{localStorage.setItem(ACTIVE_ROUTE_KEY,route)}catch{}},[route]);

  return <AppShell route={route} onNavigate={setRoute} player={player} players={players} onPlayerChange={id=>void selectPlayer(id)} onCreate={()=>setQuickCreate(true)} ready={healthReady}>
    {notice&&<div className="notice-bar" role="status"><span>{notice}</span><button className="text-link" onClick={()=>setNotice('')} aria-label="Dismiss notification">Dismiss</button></div>}
    {error&&<div className="error-banner" role="alert"><strong>World unavailable</strong><span>{error}</span><button className="button button--small" onClick={()=>void refreshWorld()}>Retry</button></div>}
    {loading&&<div className="loading-state" role="status"><span className="loading-dot"/>Loading your world…</div>}
    {!loading&&(!player||!overview)?<WorldWorkspace route={route} client={coreClient} player={null} overview={null} concepts={[]} stats={[]} sessions={[]} onRefresh={afterMutation} onCreatePlayer={createPlayer} onNavigate={setRoute}/>:!loading&&player&&overview&&<WorldWorkspace route={route} client={coreClient} player={player} overview={overview} concepts={concepts} stats={stats} sessions={sessions} onRefresh={afterMutation} onCreatePlayer={createPlayer} onNavigate={setRoute}/>}
    {route==='rules'&&player&&<RulePanel playerId={player.id} client={coreClient}/>}
    {route==='status'&&<StatusScreen/>}
    {quickCreate&&<div className="modal-backdrop" role="presentation" onClick={()=>setQuickCreate(false)}><section className="quick-create" role="dialog" aria-modal="true" aria-labelledby="quick-create-title" onClick={e=>e.stopPropagation()}><button className="quick-create__close" aria-label="Close" onClick={()=>setQuickCreate(false)}>×</button><p className="eyebrow">Start with a record</p><h2 id="quick-create-title">Create in this world</h2><p className="muted">Choose what you want to add. Your entry stays local and editable.</p><div className="quick-create__grid">{([{label:'Quest',route:'quests'},{label:'Skill tree',route:'skills'},{label:'Concept',route:'concepts'},{label:'Journal entry',route:'journal'}] as const).map(x=><button className="quick-create__choice" key={x.route} onClick={()=>quickNavigate(x.route)}><span>＋</span>{x.label}<small>Open creation form</small></button>)}</div></section></div>}
  </AppShell>;
}
