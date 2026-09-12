import { useMemo, useState } from 'react';
import ReactFlow, { Background, Controls, MiniMap } from 'reactflow';
import 'reactflow/dist/style.css';
import { aggregate, parseGraph, toFlow, type Graph, type Checkpoint } from './graph-model.mjs';

interface Props {
  graph:Graph;
  checkpoint?:Checkpoint|null;
  /** Only an authenticated host/BFF may implement these callbacks. */
  onStart?:(graph:Graph)=>Promise<void>;
  onResume?:(runId:string,revision:number)=>Promise<void>;
  onResolve?:(runId:string,revision:number,nodeId:string,approved:boolean)=>Promise<void>;
}
export function HarnessGraphWorkbench({graph,checkpoint=null,onStart,onResume,onResolve}:Props) {
  const [text,setText]=useState(()=>JSON.stringify(graph,null,2));
  const [current,setCurrent]=useState(graph);
  const [selected,setSelected]=useState<string|null>(null);
  const [error,setError]=useState('');const[busy,setBusy]=useState(false);
  const flow=useMemo(()=>toFlow(current,checkpoint),[current,checkpoint]);
  const state=aggregate(checkpoint);
  const chosen=current.nodes.find(n=>n.id===selected);
  async function action(fn:()=>Promise<void>){setBusy(true);setError('');try{await fn();}catch(e){setError(e instanceof Error?e.message:'Request failed');}finally{setBusy(false);}}
  function validate(){try{if(checkpoint)throw new Error('Active definition is immutable. Start a new run to edit.');setCurrent(parseGraph(text));setError('');}catch(e){setError(e instanceof Error?e.message:'Invalid graph');}}
  return <section aria-label="Harness graph" style={{display:'grid',gap:12}}>
    <header><h2>AnyCode · Harness Graph</h2><p>{checkpoint?`Run ${checkpoint.run_id} · revision ${checkpoint.revision} · ${state}`:'Not executed. Preview only until a host is connected.'}</p></header>
    <div style={{display:'flex',gap:8}}>
      <button disabled={busy||!!checkpoint} onClick={validate}>Validate JSON</button>
      <button disabled={busy||!onStart||!!checkpoint} onClick={()=>onStart&&action(()=>onStart(current))}>Start validated graph</button>
      <button disabled={busy||!onResume||!checkpoint||['uncertain','failed','partial','cancelled','completed'].includes(state)} onClick={()=>onResume&&checkpoint&&action(()=>onResume(checkpoint.run_id,checkpoint.revision))}>Resume explicitly</button>
    </div>
    {error&&<p role="alert">{error}</p>}
    <div style={{height:460,border:'1px solid',borderRadius:8}}>
      <ReactFlow nodes={flow.nodes} edges={flow.edges} fitView nodesConnectable={false} nodesDraggable={false} onNodeClick={(_,node)=>setSelected(node.id)}>
        <Background/><MiniMap/><Controls/>
      </ReactFlow>
    </div>
    {chosen&&<aside><h3>{chosen.id}</h3><pre style={{whiteSpace:'pre-wrap'}}>{JSON.stringify(checkpoint?.nodes[chosen.id]??chosen,null,2)}</pre>
      {chosen.kind.type==='human'&&checkpoint?.nodes[chosen.id]?.status==='waiting'&&<div>
        <button disabled={busy||!onResolve} onClick={()=>onResolve&&checkpoint&&action(()=>onResolve(checkpoint.run_id,checkpoint.revision,chosen.id,true))}>Approve this node</button>
        <button disabled={busy||!onResolve} onClick={()=>onResolve&&checkpoint&&action(()=>onResolve(checkpoint.run_id,checkpoint.revision,chosen.id,false))}>Reject</button>
      </div>}
    </aside>}
    <label>Versioned graph definition<textarea value={text} disabled={!!checkpoint} onChange={e=>setText(e.target.value)} spellCheck={false} style={{display:'block',width:'100%',minHeight:260,fontFamily:'monospace'}}/></label>
  </section>;
}
