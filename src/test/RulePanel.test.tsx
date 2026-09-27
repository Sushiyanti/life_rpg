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
    await user.clear(screen.getByLabelText('current xp'));
    await user.type(screen.getByLabelText('current xp'),'100');
    await user.click(screen.getByRole('button',{name:/save rule/i}));
    await screen.findByText(/rule saved/i);
    const createCall=transport.mock.calls.find(([command])=>command==='create_rule');
    expect(createCall?.[1]).toMatchObject({name:'XP milestone',priority:0,definition:{schemaVersion:1,trigger:'player_xp_changed',condition:{op:'number_compare',subject:'current_xp',comparison:'greater_or_equal',value:100},actions:[{kind:'award_xp',amount:10,reason:'rule_action'}]}});
    await user.click(screen.getByRole('button',{name:/send \+25 xp test event/i}));
    await waitFor(()=>expect(transport).toHaveBeenCalledWith('award_xp',{playerId:'p1',amount:25,reason:'rule_panel_test',description:null}));
    expect(await screen.findByRole('status')).toHaveTextContent(/sent a \+25 xp event/i);
  });
});
