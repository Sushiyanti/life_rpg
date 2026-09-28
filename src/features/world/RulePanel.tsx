import { useCallback, useEffect, useState, type FormEvent } from 'react';
import { coreClient, type CoreClient } from '../../domain/ipc';
import type {
  Comparison, EventKind, NumericSubject, Rule, RuleAction, RuleCondition,
  RuleExecution, TextSubject,
} from '../../domain/world';
import './RulePanel.css';

type ConditionMode = 'number' | 'text';
type ActionKind = RuleAction['kind'];
const NUMERIC_SUBJECTS: Partial<Record<EventKind, NumericSubject[]>> = {
  player_xp_changed: ['current_xp', 'previous_xp', 'requested_amount', 'applied_amount', 'player_level'],
  quest_completed: ['quest_progress', 'quest_xp_reward'],
  stat_changed: ['stat_value'],
  concept_progress_changed: ['previous_progress', 'current_progress', 'concept_progress_level'],
  skill_xp_changed: ['previous_skill_xp', 'current_skill_xp', 'requested_skill_xp_delta', 'applied_skill_xp_delta'],
};
const TEXT_SUBJECTS: Partial<Record<EventKind, TextSubject[]>> = {
  quest_completed: ['quest_type'],
  stat_changed: ['stat_code'],
  concept_progress_changed: ['concept_type', 'concept_track_code'],
  skill_xp_changed: ['skill_id', 'event_source'],
  skill_unlocked: ['skill_id', 'event_source'],
  session_started: ['session_id', 'quest_id', 'quest_stage_id', 'quest_branch_id', 'skill_id', 'concept_id'],
  session_finished: ['session_id', 'quest_id', 'quest_stage_id', 'quest_branch_id', 'skill_id', 'concept_id', 'session_status', 'session_result'],
  effect_created: ['concept_id', 'effect_type_code', 'effect_source'],
  effect_deactivated: ['effect_type_code', 'effect_source'],
};
const pretty = (value: string) => value.replaceAll('_', ' ');
const ACTIONS: { value: ActionKind; label: string }[] = [
  { value: 'award_xp', label: 'Award / remove Player XP' },
  { value: 'award_skill_xp', label: 'Award / remove Skill XP' },
  { value: 'unlock_skill', label: 'Unlock a Skill (requires Rule-authorized policy)' },
  { value: 'complete_quest', label: 'Complete a Quest' },
  { value: 'modify_player_stat', label: 'Adjust a Player stat' },
  { value: 'set_player_stat', label: 'Set a Player stat' },
  { value: 'set_concept_progress', label: 'Set Concept progress' },
  { value: 'apply_effect', label: 'Apply an immediate Effect' },
  { value: 'deactivate_effect', label: 'Deactivate an active Effect' },
];

function actionSummary(json: string): string {
  try {
    const rows: unknown = JSON.parse(json);
    if (Array.isArray(rows)) {
      const kinds = rows.flatMap(row => row && typeof row === 'object' && 'kind' in row && typeof row.kind === 'string' ? [pretty(row.kind)] : []);
      if (kinds.length) return `Rule actions: ${kinds.join(', ')}`;
    }
  } catch { /* Keep the audit view useful even if an old row has malformed JSON. */ }
  return 'Rule action recorded';
}

export function RulePanel({ playerId, client = coreClient }: { playerId?: string; client?: CoreClient }) {
  const [rules, setRules] = useState<Rule[]>([]);
  const [history, setHistory] = useState<RuleExecution[]>([]);
  const [name, setName] = useState('');
  const [trigger, setTrigger] = useState<EventKind>('player_xp_changed');
  const [mode, setMode] = useState<ConditionMode>('number');
  const [subject, setSubject] = useState<NumericSubject | TextSubject>('current_xp');
  const [comparison, setComparison] = useState<Comparison>('greater_or_equal');
  const [threshold, setThreshold] = useState('100');
  const [textValue, setTextValue] = useState('');
  const [action, setAction] = useState<ActionKind>('award_xp');
  const [amount, setAmount] = useState('10');
  const [statCode, setStatCode] = useState('focus');
  const [questId, setQuestId] = useState('');
  const [skillId, setSkillId] = useState('');
  const [effectId, setEffectId] = useState('');
  const [effectTypeCode, setEffectTypeCode] = useState('buff');
  const [effectName, setEffectName] = useState('');
  const [effectConceptId, setEffectConceptId] = useState('');
  const [effectIntensity, setEffectIntensity] = useState('1');
  const [effectExpiry, setEffectExpiry] = useState('');
  const [priority, setPriority] = useState('0');
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');

  const refresh = useCallback(async () => {
    try {
      const [r, h] = await Promise.all([client.listRules(), client.listRuleExecutions(30)]);
      setRules(r); setHistory(h);
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Could not load rules'); }
  }, [client]);
  useEffect(() => { void refresh(); }, [refresh]);

  const numericSubjects = NUMERIC_SUBJECTS[trigger] ?? [];
  const textSubjects = TEXT_SUBJECTS[trigger] ?? [];
  const availableSubjects = mode === 'number' ? numericSubjects : textSubjects;
  function changeTrigger(next: EventKind) {
    setTrigger(next);
    const nextNumeric = NUMERIC_SUBJECTS[next] ?? [];
    const nextText = TEXT_SUBJECTS[next] ?? [];
    const nextMode: ConditionMode = nextNumeric.length ? 'number' : 'text';
    setMode(nextMode);
    setSubject((nextMode === 'number' ? nextNumeric[0] : nextText[0]) ?? 'session_id');
  }

  async function create(event: FormEvent) {
    event.preventDefault();
    const numberValue = Number(threshold);
    const amountValue = Number(amount);
    if (!name.trim() || (mode === 'number' && !Number.isFinite(numberValue)) || !Number.isFinite(amountValue)) return;
    const condition: RuleCondition = mode === 'number'
      ? { op: 'number_compare', subject: subject as NumericSubject, comparison, value: numberValue }
      : { op: 'text_compare', subject: subject as TextSubject, comparison: comparison === 'not_equal' ? 'not_equal' : 'equal', value: textValue.trim() };
    let ruleAction: RuleAction;
    if (action === 'award_xp') ruleAction = { kind: 'award_xp', amount: Math.trunc(amountValue), reason: 'rule_action' };
    else if (action === 'award_skill_xp') ruleAction = { kind: 'award_skill_xp', skillId: skillId.trim(), delta: Math.trunc(amountValue), reason: 'rule_action' };
    else if (action === 'unlock_skill') ruleAction = { kind: 'unlock_skill', skillId: skillId.trim() };
    else if (action === 'set_player_stat') ruleAction = { kind: 'set_player_stat', statCode: statCode.trim(), value: amountValue };
    else if (action === 'modify_player_stat') ruleAction = { kind: 'modify_player_stat', statCode: statCode.trim(), delta: amountValue };
    else if (action === 'set_concept_progress') ruleAction = { kind: 'set_concept_progress', conceptId: questId.trim(), trackCode: statCode.trim(), value: amountValue, level: null };
    else if (action === 'complete_quest') ruleAction = { kind: 'complete_quest', questId: questId.trim() };
    else if (action === 'deactivate_effect') ruleAction = { kind: 'deactivate_effect', effectId: effectId.trim() };
    else ruleAction = {
      kind: 'apply_effect', typeCode: effectTypeCode.trim(), name: effectName.trim(), description: null,
      targetConceptId: effectConceptId.trim() || null, intensity: Math.trunc(Number(effectIntensity)),
      expiresInSeconds: effectExpiry.trim() ? Math.trunc(Number(effectExpiry)) : null,
    };

    if (mode === 'text' && !textValue.trim()) { setMessage('Enter a value for this text condition.'); return; }
    if ((action === 'award_skill_xp' || action === 'unlock_skill') && !skillId.trim()) { setMessage('Enter the target Skill ID.'); return; }
    if (action === 'complete_quest' && !questId.trim()) { setMessage('Enter a Quest ID for the completion action.'); return; }
    if (action === 'set_concept_progress' && !questId.trim()) { setMessage('Enter the target Concept ID.'); return; }
    if (action === 'deactivate_effect' && !effectId.trim()) { setMessage('Enter the target Effect ID.'); return; }
    if (action === 'apply_effect' && (!effectTypeCode.trim() || !effectName.trim() || Number(effectIntensity) < 1 || (effectExpiry && Number(effectExpiry) < 1))) { setMessage('Effect actions need a type, name, positive intensity, and optional positive expiry seconds.'); return; }
    if (action === 'award_xp' && Math.trunc(amountValue) === 0 || action === 'award_skill_xp' && Math.trunc(amountValue) === 0) { setMessage('XP changes must be non-zero.'); return; }

    const definition = { schemaVersion: 1 as const, trigger, condition, actions: [ruleAction] };
    setBusy(true); setMessage('');
    try {
      await client.createRule(name.trim(), Math.trunc(Number(priority) || 0), definition);
      setName(''); setMessage('Rule saved. Matching events run in one bounded, audited transaction.'); await refresh();
    } catch (error) { setMessage(error instanceof Error ? error.message : 'Could not save rule'); }
    finally { setBusy(false); }
  }
  async function toggle(rule: Rule) {
    setBusy(true); try { await client.setRuleEnabled(rule.id, !rule.enabled); await refresh(); }
    catch (error) { setMessage(error instanceof Error ? error.message : 'Could not update rule'); }
    finally { setBusy(false); }
  }
  async function testXp() {
    if (!playerId) return;
    setBusy(true); try { await client.awardXp(playerId, 25, 'rule_panel_test'); setMessage('Sent a +25 XP event through the normal application path.'); await refresh(); }
    catch (error) { setMessage(error instanceof Error ? error.message : 'Test event failed'); }
    finally { setBusy(false); }
  }

  return <section className="rule-panel" aria-label="Declarative rule engine">
    <div className="rule-panel__header"><div><p className="rule-panel__eyebrow">BOUNDED · PLAYER-AUTHORED AUTOMATION</p><h2>Rules</h2><p>Choose a real event, a typed condition, and one explicit action. Rules contain data, never executable code. Skill levels remain Player-controlled.</p></div>{playerId && <button className="rule-panel__test" onClick={testXp} disabled={busy}>Send +25 XP test event</button>}</div>
    <form className="rule-form" onSubmit={create}>
      <label>Rule name<input value={name} onChange={e => setName(e.target.value)} maxLength={160} required placeholder="Skill milestone" /></label>
      <label>Trigger<select value={trigger} onChange={e => changeTrigger(e.target.value as EventKind)}>
        <option value="player_xp_changed">Player XP changed</option><option value="quest_completed">Quest completed</option><option value="stat_changed">Player stat changed</option><option value="concept_progress_changed">Concept progress changed</option>
        <option value="skill_xp_changed">Skill XP changed</option><option value="skill_unlocked">Skill unlocked</option><option value="session_started">Session started</option><option value="session_finished">Session finished</option><option value="effect_created">Effect created</option><option value="effect_deactivated">Effect deactivated</option>
      </select></label>
      {numericSubjects.length > 0 && <label>Condition field<select value={mode} onChange={e => { const next=e.target.value as ConditionMode; setMode(next); setSubject((next==='number'?numericSubjects[0]:textSubjects[0]) ?? 'session_id'); }}><option value="number">Numeric field</option>{textSubjects.length > 0 && <option value="text">Text field</option>}</select></label>}
      {availableSubjects.length > 0 && <label>{mode === 'number' ? 'Numeric field' : 'Text field'}<select value={subject} onChange={e => setSubject(e.target.value as NumericSubject | TextSubject)}>{availableSubjects.map(value => <option key={value} value={value}>{pretty(value)}</option>)}</select></label>}
      {mode === 'number' ? <><label>Comparison<select value={comparison} onChange={e => setComparison(e.target.value as Comparison)}><option value="greater_or_equal">at least</option><option value="greater">greater than</option><option value="equal">equals</option><option value="less_or_equal">at most</option><option value="less">less than</option><option value="not_equal">not equal to</option></select></label><label>Threshold<input type="number" value={threshold} onChange={e => setThreshold(e.target.value)} step="any" required /></label></>
        : <><label>Text comparison<select value={comparison === 'not_equal' ? 'not_equal' : 'equal'} onChange={e => setComparison(e.target.value as Comparison)}><option value="equal">equals</option><option value="not_equal">does not equal</option></select></label><label>Expected text<input value={textValue} onChange={e => setTextValue(e.target.value)} maxLength={512} required placeholder="e.g. completed" /></label></>}
      <label>Action<select value={action} onChange={e => setAction(e.target.value as ActionKind)}>{ACTIONS.map(item => <option key={item.value} value={item.value}>{item.label}</option>)}</select></label>
      {(action === 'modify_player_stat' || action === 'set_player_stat') && <label>Stat code<input value={statCode} onChange={e => setStatCode(e.target.value)} required /></label>}
      {(action === 'award_xp' || action === 'award_skill_xp' || action === 'set_player_stat' || action === 'modify_player_stat' || action === 'set_concept_progress') && <label>{action === 'award_xp' ? 'Player XP delta' : action === 'award_skill_xp' ? 'Skill XP delta' : action === 'set_player_stat' ? 'Stat value' : action === 'modify_player_stat' ? 'Stat delta' : 'Progress value'}<input type="number" value={amount} onChange={e => setAmount(e.target.value)} step={action.includes('xp') ? '1' : 'any'} required /></label>}
      {(action === 'award_skill_xp' || action === 'unlock_skill') && <label>Target Skill ID<input value={skillId} onChange={e => setSkillId(e.target.value)} maxLength={160} required /></label>}
      {(action === 'complete_quest' || action === 'set_concept_progress') && <label>{action === 'complete_quest' ? 'Quest ID' : 'Concept ID'}<input value={questId} onChange={e => setQuestId(e.target.value)} maxLength={160} required /></label>}
      {action === 'deactivate_effect' && <label>Effect ID<input value={effectId} onChange={e => setEffectId(e.target.value)} maxLength={160} required /></label>}
      {action === 'apply_effect' && <><label>Effect type code<input value={effectTypeCode} onChange={e => setEffectTypeCode(e.target.value)} maxLength={160} required /></label><label>Effect name<input value={effectName} onChange={e => setEffectName(e.target.value)} maxLength={160} required /></label><label>Target Concept ID <span className="muted">optional; blank targets Player</span><input value={effectConceptId} onChange={e => setEffectConceptId(e.target.value)} maxLength={160} /></label><label>Intensity<input type="number" min="1" step="1" value={effectIntensity} onChange={e => setEffectIntensity(e.target.value)} required /></label><label>Expiry after seconds <span className="muted">optional, max 1 year</span><input type="number" min="1" max="31536000" step="1" value={effectExpiry} onChange={e => setEffectExpiry(e.target.value)} /></label></>}
      <label>Priority<input type="number" value={priority} onChange={e => setPriority(e.target.value)} /></label>
      <button type="submit" disabled={busy || !name.trim() || availableSubjects.length === 0}>Save rule</button>
    </form>
    {message && <p role="status" className="rule-panel__message">{message}</p>}
    <div className="rule-panel__columns"><div><h3>Saved rules</h3>{rules.length === 0 ? <p className="rule-panel__empty">No rules yet. Rules are evaluated by descending priority, with stable ID ordering.</p> : <ul className="rule-list">{rules.map(rule => <li key={rule.id}><div><strong>{rule.name}</strong><span>{pretty(rule.definition.trigger)} · priority {rule.priority} · {rule.definition.actions.map(a => pretty(a.kind)).join(', ')}</span></div><button onClick={() => void toggle(rule)} disabled={busy} aria-label={`${rule.enabled ? 'Disable' : 'Enable'} ${rule.name}`}>{rule.enabled ? 'Enabled' : 'Disabled'}</button></li>)}</ul>}</div>
      <div><h3>Recent executions</h3>{history.length === 0 ? <p className="rule-panel__empty">Execution outcomes will appear here when matching domain events occur.</p> : <ul className="rule-history">{history.slice(0, 12).map(row => <li key={row.id}><strong>{pretty(row.status)}</strong><span>{pretty(row.eventKind)} · depth {row.depth} · {row.executedAt}</span><small>{actionSummary(row.actionsJson)}</small>{row.error && <small>{row.error}</small>}</li>)}</ul>}</div></div>
  </section>;
}
