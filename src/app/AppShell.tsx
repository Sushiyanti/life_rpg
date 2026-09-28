import type { ReactNode } from 'react';
import type { Player, Workspace } from '../domain/world';
import './AppShell.css';

export type AppRoute = 'dashboard'|'player'|'quests'|'skills'|'skillTrees'|'concepts'|'effects'|'journal'|'tags'|'timeline'|'explorer'|'history'|'rules'|'status';
const navigation: {section:string;items:{id:AppRoute;label:string;glyph:string}[]}[] = [
  {section:'World',items:[{id:'player',label:'Player Hub',glyph:'◉'},{id:'dashboard',label:'Overview',glyph:'◈'},{id:'quests',label:'Quests',glyph:'◇'},{id:'skills',label:'Skills',glyph:'⌁'},{id:'skillTrees',label:'Skill trees',glyph:'⌘'},{id:'concepts',label:'Concepts',glyph:'◎'},{id:'effects',label:'Effects',glyph:'✦'},{id:'journal',label:'Content Guidebook',glyph:'▤'},{id:'tags',label:'Tags',glyph:'#'}]},
  {section:'Tools',items:[{id:'timeline',label:'Timeline',glyph:'◷'},{id:'explorer',label:'World explorer',glyph:'⌕'},{id:'history',label:'History & recovery',glyph:'◴'},{id:'rules',label:'Rules & automation',glyph:'⟳'}]},
];
const titles:Record<AppRoute,string>={dashboard:'Your world',player:'Player Hub',quests:'Quest log',skills:'Skills',skillTrees:'Skill trees',concepts:'Concepts',effects:'Effects',journal:'Content Guidebook',tags:'Tag Manager',timeline:'Timeline',explorer:'World explorer',history:'History & recovery',rules:'Rules & automation',status:'System health'};

export function AppShell({children,route,onNavigate,player,players,onPlayerChange,workspaces,workspace,onWorkspaceChange,onCreate,onQuickCapture,ready}:{children:ReactNode;route:AppRoute;onNavigate:(route:AppRoute)=>void;player:Player|null;players:{id:string;name:string}[];onPlayerChange:(id:string)=>void;workspaces:Workspace[];workspace:Workspace|null;onWorkspaceChange:(id:string)=>void;onCreate:()=>void;onQuickCapture:()=>void;ready:boolean}) {
  return <div className="app-shell">
    <aside className="app-rail" aria-label="Main navigation">
      <button className="app-brand" onClick={()=>onNavigate('dashboard')} aria-label="Life RPG home"><span className="app-brand__mark">L</span><span>Life RPG</span></button>
      <div className="app-rail__scroll">{navigation.map(group=><section className="app-nav-group" key={group.section}><p>{group.section}</p>{group.items.map(item=><button key={item.id} className={`app-nav-item ${route===item.id?'is-active':''}`} onClick={()=>onNavigate(item.id)} aria-current={route===item.id?'page':undefined}><span aria-hidden="true">{item.glyph}</span>{item.label}</button>)}</section>)}</div>
      <div className="app-rail__bottom"><button className={`app-nav-item ${route==='status'?'is-active':''}`} onClick={()=>onNavigate('status')}><span className={`health-dot ${ready?'is-ready':''}`} aria-hidden="true"/>System health</button><small>Offline-first · local world</small></div>
    </aside>
    <div className="app-main-column">
      <header className="app-topbar"><div className="app-topbar__title"><span>WORLD / {titles[route].toUpperCase()}</span><h1>{titles[route]}</h1></div>
        <div className="app-topbar__actions">{players.length>0&&<label className="world-switch"><span>Active world</span><select aria-label="Active player world" value={player?.id??''} onChange={e=>onPlayerChange(e.target.value)}>{players.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</select></label>}{(route==='dashboard'||route==='player')&&player&&workspaces.length>0&&<label className="world-switch"><span>Workspace</span><select aria-label="Active workspace" value={workspace?.id??''} onChange={e=>onWorkspaceChange(e.target.value)}>{workspaces.map(w=><option key={w.id} value={w.id}>{w.name}</option>)}</select></label>}{player&&<div className="top-player"><span className="avatar">{player.name.slice(0,1).toUpperCase()}</span><div><strong>{player.name}</strong><small>Level {player.level}{player.levelName?` · ${player.levelName}`:''}</small></div></div>}{player&&<button className="button button--quiet button--compact" onClick={onQuickCapture} aria-label="Quick capture">✎ Capture</button>}<button className="button button--primary button--compact" onClick={onCreate} disabled={!player} aria-label="Create new item">＋ New</button></div>
      </header>
      <main className="app-shell__content" key={route}>{children}</main>
      <footer className="app-footer"><span>Life RPG</span><span>Progress is player-authored · history stays recoverable</span></footer>
    </div>
  </div>;
}
