import type { ReactNode } from 'react';
import type { Concept, ConceptProgressTrack, Effect, NarrativeEntry, Player, PlayerStat, Quest, QuestSession, Skill, Transaction, WorkspacePanel, WorkspacePanelType, WorldOverview } from '../../domain/world';
import type { AppRoute } from '../../app/AppShell';
import { PANEL_REGISTRY } from './panelRegistry';

export type PanelRow={id:string;kind:string;title:string;subtitle:string;value?:string;status?:string;typeCode?:string;at?:string};
type Data={player:Player;overview:WorldOverview;concepts:Concept[];stats:PlayerStat[];sessions:QuestSession[];tracks:Record<string,ConceptProgressTrack[]>};
type Props={panel:WorkspacePanel;data:Data;allowedIds?:ReadonlySet<string>;isVisible:(kind:string,id:string)=>boolean;onVisibility:(kind:string,id:string)=>Promise<void>;onNavigate:(route:AppRoute)=>void;onOpenEntity:(kind:string,id:string)=>void};
const pretty=(s:string)=>s.replaceAll('_',' ');const minutes=(m:number)=>m<60?`${m}m`:`${Math.floor(m/60)}h ${m%60}m`;
const registry:Record<WorkspacePanelType,(d:Data)=>PanelRow[]>={
 player:d=>[{id:d.player.id,kind:'player',title:d.player.name,subtitle:`Player level ${d.player.level} · ${d.player.levelName??d.player.progressionLabel??'manually set'}`,value:`${d.player.currentXp} XP`,status:'active'},...d.stats.map(s=>({id:`stat:${s.statCode}`,kind:'player',title:pretty(s.statCode),subtitle:'Player-authored stat',value:String(s.currentValue),status:'active'}))],
 quests:d=>d.overview.quests.map((q:Quest)=>({id:q.id,kind:'quest',title:q.title,subtitle:`${pretty(q.status)} · ${q.progress}%`,value:`${q.progress}%`,status:q.status,typeCode:q.typeCode,at:q.startedAt??undefined})),
 skills:d=>d.overview.skills.map((s:Skill)=>({id:s.id,kind:'skill',title:s.name,subtitle:`Level ${s.level} · ${minutes(s.investedMinutes)} recorded`,value:s.levelName??String(s.level),status:s.status,typeCode:s.typeCode})),
 concepts:d=>d.concepts.map(c=>({id:c.id,kind:'concept',title:c.name,subtitle:pretty(c.typeCode),status:c.isActive?'active':'archived',typeCode:c.typeCode,at:c.updatedAt})),
 progress:d=>d.concepts.flatMap(c=>(d.tracks[c.id]??[]).filter(t=>t.isActive).map(t=>({id:`${c.id}:${t.trackCode}`,kind:'concept',title:`${c.name} · ${pretty(t.trackCode)}`,subtitle:`${t.level?`Level ${t.level} · `:''}${t.control==='manual'?'Player-authored':'rule-controlled'}`,value:String(t.currentValue),status:c.isActive?'active':'archived',typeCode:t.trackCode,at:t.updatedAt}))),
 effects:d=>d.overview.effects.map((x:Effect)=>({id:x.id,kind:'effect',title:x.name,subtitle:`${pretty(x.typeCode)} · intensity ${x.intensity}`,status:x.deactivatedAt?'inactive':'active',typeCode:x.typeCode,at:x.startedAt})),
 activity:d=>d.sessions.map((s:QuestSession)=>({id:s.id,kind:'quest_session',title:s.questId?d.overview.quests.find(q=>q.id===s.questId)?.title??'Quest session':s.skillId?d.overview.skills.find(k=>k.id===s.skillId)?.name??'Skill session':s.conceptId?d.concepts.find(c=>c.id===s.conceptId)?.name??'Concept session':'Recorded session',subtitle:`${pretty(s.status)} · ${new Date(s.startedAt).toLocaleString()}`,status:s.status,at:s.startedAt})),
 transactions:d=>d.overview.recentTransactions.map((t:Transaction)=>({id:String(t.id),kind:'transaction',title:`${pretty(t.resource)} · ${t.amount>0?'+':''}${t.amount}`,subtitle:`${t.reason??t.description??'Recorded transaction'} · ${new Date(t.occurredAt).toLocaleString()}`,status:'recorded',typeCode:t.typeCode,at:t.occurredAt})),
 journal:d=>d.overview.narratives.map((n:NarrativeEntry)=>({id:n.id,kind:'narrative_entry',title:n.title,subtitle:`${pretty(n.kind)} · ${new Date(n.createdAt).toLocaleString()}`,status:n.kind,typeCode:n.kind,at:n.createdAt})),
};
const compare=(a:PanelRow,b:PanelRow,sort:string)=>{if(sort==='name_asc')return a.title.localeCompare(b.title);if(sort==='status_asc')return (a.status??'').localeCompare(b.status??'');if(sort==='progress_desc'||sort==='level_desc')return Number(b.value??0)-Number(a.value??0);return (b.at??'').localeCompare(a.at??'')};
const routeFor=(type:WorkspacePanelType):AppRoute=>({player:'player',quests:'quests',skills:'skills',concepts:'concepts',progress:'concepts',effects:'effects',activity:'quests',transactions:'explorer',journal:'journal'} as const)[type];
const icon:Record<WorkspacePanelType,string>={player:'◉',quests:'◇',skills:'⌁',concepts:'◎',progress:'◈',effects:'✦',activity:'◷',transactions:'↗',journal:'▤'};

export function WorkspacePanelView({panel,data,allowedIds,isVisible,onVisibility,onNavigate,onOpenEntity}:Props){
 const definition=PANEL_REGISTRY[panel.panelType];
 let rows=registry[panel.panelType](data);
 if(panel.filterStatus&&!['active','archived'].includes(panel.filterStatus))rows=rows.filter(r=>r.status===panel.filterStatus);
 if(panel.filterTypeCode)rows=rows.filter(r=>r.typeCode===panel.filterTypeCode);
 if(allowedIds)rows=rows.filter(r=>allowedIds.has(r.id)|| (r.kind==='concept'&&allowedIds.has(r.id.split(':')[0]!)));
 rows=rows.filter(r=>!['quest','skill','concept','effect','quest_session','narrative_entry'].includes(r.kind)||isVisible(r.kind,r.id.split(':')[0]!));
 const backendOrder=allowedIds?new Map(Array.from(allowedIds).map((id,index)=>[id,index])):null;
 rows.sort((a,b)=>panel.panelType!=='progress'&&panel.sortBy!=='status_asc'&&backendOrder?((backendOrder.get(a.id)??backendOrder.get(a.id.split(':')[0]!)??Number.MAX_SAFE_INTEGER)-(backendOrder.get(b.id)??backendOrder.get(b.id.split(':')[0]!)??Number.MAX_SAFE_INTEGER)):compare(a,b,panel.sortBy));
 if(panel.filterRecentDays){const cutoff=Date.now()-panel.filterRecentDays*86400000;rows=rows.filter(r=>r.at&&Date.parse(r.at)>=cutoff)}
 rows=rows.slice(0,panel.itemLimit);
 const act=(row:PanelRow):ReactNode=>['quest','skill','concept','effect','quest_session','narrative_entry'].includes(row.kind)?<button className="text-link" onClick={()=>void onVisibility(row.kind,row.id.split(':')[0]!)}>{isVisible(row.kind,row.id.split(':')[0]!)?'Hide':'Show'}</button>:null;
 const open=(row:PanelRow)=>onOpenEntity(row.kind,row.kind==='concept'?row.id.split(':')[0]!:row.id);
 if(!panel.isVisible)return null;
 return <div className={`panel-data panel-data--${panel.variant} panel-data--${panel.density}`} data-panel-source={panel.panelType}>
  {rows.length===0?<div className="widget-empty"><p>No {definition.label.toLowerCase()} match this view.</p><button className="text-link" onClick={()=>onNavigate(routeFor(panel.panelType))}>Open {definition.label.toLowerCase()} →</button></div>:rows.map((row,i)=><article className={`panel-row panel-row--${panel.variant}`} key={row.id}>
   {panel.variant==='metrics'?<button className="panel-row__metric-open" onClick={()=>open(row)} aria-label={`Open ${row.title}`}><span className="panel-row__metric-icon">{icon[panel.panelType]}</span><strong>{row.value??row.title}</strong><span>{row.title}</span></button>:<>
    <span className="panel-row__mark" aria-hidden="true">{panel.variant==='timeline'?<i/>:icon[panel.panelType]}</span><button className="panel-row__open" onClick={()=>open(row)}><strong>{row.title}</strong><small>{row.subtitle}</small></button>{row.value&&panel.variant!=='compact'&&<span className="panel-row__value">{row.value}</span>}{act(row)}
   </>}
   {panel.variant==='detailed'&&<p className="panel-row__detail">{row.status?`Status: ${pretty(row.status)}.`:''}{row.at?` Recorded ${new Date(row.at).toLocaleString()}.`:''}</p>}
   {panel.variant==='tree'&&i>0&&<span className="panel-row__tree-branch" aria-hidden="true">↳</span>}
  </article>)}
 </div>;
}
