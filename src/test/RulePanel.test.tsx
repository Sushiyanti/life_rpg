import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { CoreClient, type InvokeTransport } from '../domain/ipc';
import { RulePanel } from '../features/world/RulePanel';

describe('RulePanel',()=>{
  it('authors a declarative XP rule and sends a test event through CoreClient',async()=>{
    const user=userEvent.setup();
    const transport=vi.fn(async(command:string,args?:Record<string,unknown>):Promise<unknown>=>{
      if(command==='list_rules'||command==='list_rule_executions')return [];
      if(command==='create_rule')return {id:'rule-1',name:args?.name,description:args?.description??null,enabled:true,priority:args?.priority,definition:args?.definition,metadataJson:'{}',createdAt:'2026-09-27T00:00:00Z',updatedAt:'2026-09-27T00:00:00Z'};
      if(command==='award_xp')return {player:{id:'p1'},transaction:{}};
      return {};
    });
    const client=new CoreClient(transport as unknown as InvokeTransport);
    render(<RulePanel client={client} playerId="p1"/>);
    await screen.findByText(/no rules yet/i);
    await user.type(screen.getByLabelText(/rule name/i),'XP milestone');
    await user.clear(screen.getByLabelText('Threshold'));
    await user.type(screen.getByLabelText('Threshold'),'100');
    await user.click(screen.getByRole('button',{name:/save rule/i}));
    await screen.findByText(/rule saved/i);
    const createCall=transport.mock.calls.find(([command])=>command==='create_rule');
    expect(createCall?.[1]).toMatchObject({name:'XP milestone',priority:0,definition:{schemaVersion:1,trigger:'player_xp_changed',condition:{op:'number_compare',subject:'current_xp',comparison:'greater_or_equal',value:100},actions:[{kind:'award_xp',amount:10,reason:'rule_action'}]}});
    await user.click(screen.getByRole('button',{name:/send \+25 xp test event/i}));
    await waitFor(()=>expect(transport).toHaveBeenCalledWith('award_xp',{playerId:'p1',amount:25,reason:'rule_panel_test',description:null}));
    expect(await screen.findByRole('status')).toHaveTextContent(/sent a \+25 xp event/i);
  });
  it('authors a typed Skill XP trigger and an independent Skill XP action',async()=>{
    const user=userEvent.setup();
    const transport=vi.fn(async(command:string,args?:Record<string,unknown>):Promise<unknown>=>{
      if(command==='list_rules'||command==='list_rule_executions')return [];
      if(command==='create_rule')return {id:'rule-skill',name:args?.name,definition:args?.definition};
      return {};
    });
    render(<RulePanel client={new CoreClient(transport as unknown as InvokeTransport)} playerId="p1"/>);
    await user.type(await screen.findByLabelText(/rule name/i),'Practice threshold');
    await user.selectOptions(screen.getByLabelText('Trigger'),'skill_xp_changed');
    await user.selectOptions(screen.getByLabelText('Numeric field'),'current_skill_xp');
    await user.clear(screen.getByLabelText('Threshold')); await user.type(screen.getByLabelText('Threshold'),'50');
    await user.selectOptions(screen.getByLabelText('Action'),'award_skill_xp');
    await user.type(screen.getByLabelText('Target Skill ID'),'skill-7');
    await user.clear(screen.getByLabelText('Skill XP delta')); await user.type(screen.getByLabelText('Skill XP delta'),'15');
    await user.click(screen.getByRole('button',{name:/save rule/i}));
    await screen.findByText(/rule saved/i);
    expect(transport.mock.calls.find(([command])=>command==='create_rule')?.[1]).toMatchObject({definition:{trigger:'skill_xp_changed',condition:{op:'number_compare',subject:'current_skill_xp',value:50},actions:[{kind:'award_skill_xp',skillId:'skill-7',delta:15,reason:'rule_action'}]}});
  });
  it('authors a Session text condition and an explicit Effect deactivation action',async()=>{
    const user=userEvent.setup();
    const transport=vi.fn(async(command:string,args?:Record<string,unknown>):Promise<unknown>=>{
      if(command==='list_rules'||command==='list_rule_executions')return [];
      if(command==='create_rule')return {id:'rule-session',name:args?.name,definition:args?.definition};
      return {};
    });
    render(<RulePanel client={new CoreClient(transport as unknown as InvokeTransport)}/>);
    await user.type(await screen.findByLabelText(/rule name/i),'End session cleanup');
    await user.selectOptions(screen.getByLabelText('Trigger'),'session_finished');
    await user.selectOptions(screen.getByLabelText('Text field'),'session_status');
    await user.type(screen.getByLabelText('Expected text'),'completed');
    await user.selectOptions(screen.getByLabelText('Action'),'deactivate_effect');
    await user.type(screen.getByLabelText('Effect ID'),'effect-9');
    await user.click(screen.getByRole('button',{name:/save rule/i}));
    await screen.findByText(/rule saved/i);
    expect(transport.mock.calls.find(([command])=>command==='create_rule')?.[1]).toMatchObject({definition:{trigger:'session_finished',condition:{op:'text_compare',subject:'session_status',comparison:'equal',value:'completed'},actions:[{kind:'deactivate_effect',effectId:'effect-9'}]}});
  });
});
