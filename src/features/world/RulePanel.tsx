import { useCallback, useEffect, useState, type FormEvent } from 'react';
import { coreClient, type CoreClient } from '../../domain/ipc';
import type { Comparison, EventKind, Rule, RuleAction, RuleCondition, RuleExecution } from '../../domain/world';
import './RulePanel.css';

type ActionKind = 'award_xp' | 'set_player_stat' | 'modify_player_stat' | 'complete_quest';
const subjectFor = (trigger: EventKind) => trigger === 'player_xp_changed' ? 'current_xp' as const : trigger === 'quest_completed' ? 'quest_progress' as const : 'stat_value' as const;
const pretty = (value:string) => value.replaceAll('_',' ');

export function RulePanel({ playerId, client=coreClient }: { playerId?: string; client?:CoreClient }) {
  const [rules,setRules]=useState<Rule[]>([]); const [history,setHistory]=useState<RuleExecution[]>([]);
  const [name,setName]=useState(''); const [trigger,setTrigger]=useState<EventKind>('player_xp_changed');
  const [comparison,setComparison]=useState<Comparison>('greater_or_equal'); const [threshold,setThreshold]=useState('100');
  const [action,setAction]=useState<ActionKind>('award_xp'); const [amount,setAmount]=useState('10');
  const [statCode,setStatCode]=useState('focus'); const [questId,setQuestId]=useState(''); const [priority,setPriority]=useState('0');
  const [busy,setBusy]=useState(false); const [message,setMessage]=useState('');
  const refresh=useCallback(async()=>{try{const [r,h]=await Promise.all([client.listRules(),client.listRuleExecutions(30)]);setRules(r);setHistory(h);}catch(error){setMessage(error instanceof Error?error.message:'Could not load rules');}},[client]);
  useEffect(()=>{void refresh();},[refresh]);
  async function create(event:FormEvent){event.preventDefault();const thresholdValue=Number(threshold);const amountValue=Number(amount);if(!name.trim()||!Number.isFinite(thresholdValue)||!Number.isFinite(amountValue))return;
    let condition:RuleCondition={op:'number_compare',subject:subjectFor(trigger),comparison,value:thresholdValue};
    let ruleAction:RuleAction;
    if(action==='award_xp')ruleAction={kind:'award_xp',amount:Math.trunc(amountValue),reason:'rule_action'};
    else if(action==='set_player_stat')ruleAction={kind:'set_player_stat',statCode:statCode.trim(),value:amountValue};
    else if(action==='modify_player_stat')ruleAction={kind:'modify_player_stat',statCode:statCode.trim(),delta:amountValue};
    else ruleAction={kind:'complete_quest',questId:questId.trim()};
    if(trigger==='quest_completed'&&action==='complete_quest'&&!questId.trim()){setMessage('Enter a Quest ID for the completion action.');return;}
    const definition={schemaVersion:1 as const,trigger,condition,actions:[ruleAction]};
    setBusy(true);setMessage('');try{await client.createRule(name.trim(),Math.trunc(Number(priority)||0),definition);setName('');setMessage('Rule saved. Matching events run in one bounded, audited transaction.');await refresh();}catch(error){setMessage(error instanceof Error?error.message:'Could not save rule');}finally{setBusy(false);}
  }
  async function toggle(rule:Rule){setBusy(true);try{await client.setRuleEnabled(rule.id,!rule.enabled);await refresh();}catch(error){setMessage(error instanceof Error?error.message:'Could not update rule');}finally{setBusy(false);}}
  async function testXp(){if(!playerId)return;setBusy(true);try{await client.awardXp(playerId,25,'rule_panel_test');setMessage('Sent a +25 XP event through the normal application path.');await refresh();}catch(error){setMessage(error instanceof Error?error.message:'Test event failed');}finally{setBusy(false);}}
  return <section className="rule-panel" aria-label="Declarative rule engine">
    <div className="rule-panel__header"><div><p className="rule-panel__eyebrow">Phase 3 · declarative automation</p><h2>Rules</h2><p>Create event-driven rules from the supported vocabulary. Rules contain data, not executable code.</p></div>{playerId&&<button className="rule-panel__test" onClick={testXp} disabled={busy}>Send +25 XP test event</button>}</div>
    <form className="rule-form" onSubmit={create}>
      <label>Rule name<input value={name} onChange={e=>setName(e.target.value)} maxLength={160} required placeholder="XP milestone"/></label>
      <label>Trigger<select value={trigger} onChange={e=>setTrigger(e.target.value as EventKind)}><option value="player_xp_changed">Player XP changed</option><option value="quest_completed">Quest completed</option><option value="stat_changed">Player stat changed</option></select></label>
      <label>Condition<select value={comparison} onChange={e=>setComparison(e.target.value as Comparison)}><option value="greater_or_equal">at least</option><option value="greater">greater than</option><option value="equal">equals</option><option value="less_or_equal">at most</option><option value="less">less than</option><option value="not_equal">not equal to</option></select></label>
      <label>{pretty(subjectFor(trigger))}<input type="number" value={threshold} onChange={e=>setThreshold(e.target.value)} step="any" required/></label>
      <label>Action<select value={action} onChange={e=>setAction(e.target.value as ActionKind)}><option value="award_xp">Award / remove XP</option><option value="modify_player_stat">Adjust a stat</option><option value="set_player_stat">Set a stat</option><option value="complete_quest">Complete a Quest</option></select></label>
      {(action==='modify_player_stat'||action==='set_player_stat')&&<label>Stat code<input value={statCode} onChange={e=>setStatCode(e.target.value)} required/></label>}
      {action==='complete_quest'?<label>Quest ID<input value={questId} onChange={e=>setQuestId(e.target.value)} required/></label>:<label>{action==='award_xp'?'XP amount':action==='set_player_stat'?'Stat value':'Stat delta'}<input type="number" value={amount} onChange={e=>setAmount(e.target.value)} step={action==='award_xp'?'1':'any'} required/></label>}
      <label>Priority<input type="number" value={priority} onChange={e=>setPriority(e.target.value)}/></label>
      <button type="submit" disabled={busy||!name.trim()}>Save rule</button>
    </form>
    {message&&<p role="status" className="rule-panel__message">{message}</p>}
    <div className="rule-panel__columns"><div><h3>Saved rules</h3>{rules.length===0?<p className="rule-panel__empty">No rules yet. Rules are evaluated by descending priority, with stable ID ordering.</p>:<ul className="rule-list">{rules.map(rule=><li key={rule.id}><div><strong>{rule.name}</strong><span>{rule.definition.trigger.replaceAll('_',' ')} · priority {rule.priority} · {rule.definition.actions.map(a=>a.kind.replaceAll('_',' ')).join(', ')}</span></div><button onClick={()=>void toggle(rule)} disabled={busy} aria-label={`${rule.enabled?'Disable':'Enable'} ${rule.name}`}>{rule.enabled?'Enabled':'Disabled'}</button></li>)}</ul>}</div>
      <div><h3>Recent executions</h3>{history.length===0?<p className="rule-panel__empty">Execution outcomes will appear here when matching domain events occur.</p>:<ul className="rule-history">{history.slice(0,12).map(row=><li key={row.id}><strong>{row.status.replaceAll('_',' ')}</strong><span>{row.eventKind.replaceAll('_',' ')} · depth {row.depth} · {row.executedAt}</span>{row.error&&<small>{row.error}</small>}</li>)}</ul>}</div></div>
  </section>;
}
