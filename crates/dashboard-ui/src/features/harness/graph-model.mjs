/** Transport contracts shared by the editor and node:test. No execution in browser. */
export const statuses = new Set(['pending','running','completed','skipped','partial','failed','waiting','uncertain','cancelled']);
const kinds = new Set(['work','gate','branch','human']);
const own=(obj,key)=>Object.prototype.hasOwnProperty.call(obj,key);
const object=(v)=>v!==null && typeof v==='object' && !Array.isArray(v);
const fail=(message)=>{throw new Error(message);};
export function validateGraph(graph) {
  if (!object(graph) || graph.version!==1 || typeof graph.name!=='string' || !graph.name.trim() || graph.name.length>128) fail('Graph version/name invalid');
  if (!Array.isArray(graph.nodes)||graph.nodes.length===0||graph.nodes.length>512) fail('Graph needs 1..512 nodes');
  const parallel=graph.max_parallel??1;
  if (!Number.isInteger(parallel)||parallel<1||parallel>32) fail('Concurrency must be 1..32');
  const nodes=new Map();
  for(const n of graph.nodes) {
    if (!object(n)||typeof n.id!=='string'||!/^[A-Za-z0-9_-]{1,64}$/.test(n.id)||nodes.has(n.id)) fail('Duplicate/invalid node ID');
    if (!object(n.kind)||!kinds.has(n.kind.type)) fail('Unknown node kind');
    if (!['all','any'].includes(n.join??'all')) fail('Invalid join mode');
    if (!Number.isInteger(n.max_attempts??1)||(n.max_attempts??1)<1||(n.max_attempts??1)>3) fail('Invalid retry count');
    if(n.kind.type==='work'&&(typeof n.kind.agent!=='string'||!n.kind.agent||typeof n.kind.prompt!=='string'||!n.kind.prompt))fail('Work node needs agent and prompt');
    if(n.kind.type==='gate'&&(typeof n.kind.verifier!=='string'||!n.kind.verifier))fail('Gate needs trusted verifier ID');
    if(n.kind.type==='human'&&(typeof n.kind.question!=='string'||!n.kind.question))fail('Human node needs question');
    if(n.kind.type==='branch'&&(typeof n.kind.source!=='string'||typeof n.kind.pointer!=='string'||(n.kind.pointer!==''&&!n.kind.pointer.startsWith('/'))||!own(n.kind,'equals')))fail('Branch needs source, JSON Pointer and equals');
    nodes.set(n.id,n);
  }
  for(const n of nodes.values()) {
    const deps=n.depends_on??[];if(!Array.isArray(deps))fail('Dependencies must be an array');const seen=new Set();
    for(const d of deps) {
      if(!object(d)||!nodes.has(d.node)||d.node===n.id||seen.has(d.node))fail('Missing/self/duplicate dependency');seen.add(d.node);
      if(!['completed','true','false'].includes(d.on??'completed'))fail('Unknown edge condition');
      if((d.on??'completed')!=='completed'&&nodes.get(d.node).kind.type!=='branch')fail('Boolean edge must originate at branch');
    }
    if(n.kind.type==='branch'&&!seen.has(n.kind.source))fail('Branch source must be a direct dependency');
  }
  const done=new Set();const layers=[];
  while(done.size<nodes.size) {
    const next=[...nodes.values()].filter(n=>!done.has(n.id)&&(n.depends_on??[]).every(d=>done.has(d.node))).map(n=>n.id);
    if(!next.length)fail('Cycles require a bounded-iteration extension; v1 is a DAG');layers.push(next);next.forEach(id=>done.add(id));
  }
  return {graph,layers};
}
export function toFlow(graph,checkpoint=null) {
  const {layers}=validateGraph(graph);const positions=new Map();layers.forEach((layer,x)=>layer.forEach((id,y)=>positions.set(id,{x:x*285,y:y*120})));
  const cp=checkpoint?.nodes??{};
  const nodes=graph.nodes.map(n=>{
    const status=own(cp,n.id)?cp[n.id].status:'pending';if(!statuses.has(status))fail('Unknown checkpoint state');
    return {id:n.id,position:positions.get(n.id),data:{label:`${n.id} · ${n.kind.type} · ${status}`,status,kind:n.kind.type},type:'default'};
  });
  const edges=graph.nodes.flatMap(n=>(n.depends_on??[]).map(d=>({id:`${d.node}->${n.id}`,source:d.node,target:n.id,label:d.on??'completed',animated:own(cp,n.id)&&cp[n.id].status==='running'})));
  return {nodes,edges};
}
export function aggregate(checkpoint) {
  const values=Object.values(checkpoint?.nodes??{}).map(n=>n.status);
  if(!values.length||values.some(v=>!statuses.has(v)))return 'invalid';
  for(const state of ['uncertain','failed','partial','cancelled','waiting'])if(values.includes(state))return state;
  if(values.every(v=>['completed','skipped'].includes(v)))return 'completed';return 'running';
}
export function parseGraph(text) {
  if(typeof text!=='string'||new TextEncoder().encode(text).length>2*1024*1024)fail('Graph JSON exceeds 2 MiB');
  const graph=JSON.parse(text);validateGraph(graph);return graph;
}
export function canResolve(checkpoint,nodeId,revision) {
  return !!checkpoint && Number.isSafeInteger(revision) && checkpoint.revision===revision && own(checkpoint.nodes??{},nodeId) && checkpoint.nodes[nodeId].status==='waiting';
}
