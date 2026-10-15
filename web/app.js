'use strict';
// Static interface backed by the same API as the CLI.
const projects=[];
const projectBriefs={};
let liveTasks=[];
const $=s=>document.querySelector(s);
const esc=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
let currentWorkTab='projects';
function renderProjects(){
 $('#projects').innerHTML=projects.length?projects.map(p=>{const working=liveTasks.filter(t=>t.project_id===p.id&&t.status==='working').length;return `<article class="project-tile"><a class="project-repo" href="${safeLink(p.repo)?esc(p.repo):'#'}" target="_blank" rel="noopener noreferrer">${esc(p.name)}<span aria-hidden="true">↗</span></a><span class="category-tag category-${projectBriefs[p.id].category.toLowerCase()}">${projectBriefs[p.id].category}</span><p class="project-brief-description">${esc(projectBriefs[p.id].description)}</p><button class="project-work-link" data-project="${p.id}">${working} tasks in progress</button></article>`;}).join(''):'<p class="empty">No projects yet.</p>';
}
function renderActivity(){
 const status=currentWorkTab==='done'?'done':'working';
 $('#activity').innerHTML=liveTasks.filter(t=>t.status===status).map(t=>`<button class="work-task" data-live-task="${esc(t.id)}"><span>${esc(projects.find(p=>p.id===t.project_id)?.name||'')}</span><strong>${esc(t.title)}</strong><span>${status==='done'?'✓':'↗'}</span></button>`).join('')||'<p class="empty">No tasks yet.</p>';
}
function safeLink(link){if(!link)return false;try{return ['https:','http:'].includes(new URL(link).protocol)}catch{return false}}
function toast(text){$('#toast').textContent=text;$('#toast').classList.add('visible');clearTimeout(toast.timer);toast.timer=setTimeout(()=>$('#toast').classList.remove('visible'),2600);}
function openProject(id){const project=projects.find(p=>p.id===id);$('#project-detail').innerHTML=`<h2 id="project-title">${esc(project.name)}</h2><p>${esc(project.description)}</p><a href="${esc(project.repo)}" target="_blank" rel="noopener noreferrer">Repository ↗</a>`+liveTasks.filter(t=>t.project_id===id).map(t=>`<button class="work-task" data-live-task="${esc(t.id)}"><strong>${esc(t.title)}</strong><span>${esc(t.status)}</span></button>`).join('');$('#project-dialog').showModal();}

document.addEventListener('click',event=>{const project=event.target.closest('[data-project]');if(project)openProject(project.dataset.project);const close=event.target.closest('.close-dialog');if(close)close.closest('dialog').close();});
function selectTab(name){
 currentWorkTab=name;
 for(const tab of ['projects','working','done']){const active=tab===name;$('#'+tab+'-tab').classList.toggle('selected',active);$('#'+tab+'-tab').setAttribute('aria-selected',String(active));}
 $('#projects-panel').hidden=name!=='projects';$('#activity-panel').hidden=name==='projects';$('#activity-panel').setAttribute('aria-labelledby',name+'-tab');renderActivity();
}
for(const tab of ['projects','working','done'])$('#'+tab+'-tab').addEventListener('click',()=>selectTab(tab));
$('.tabs').addEventListener('keydown',event=>{const tabs=['projects','working','done'];if(['ArrowLeft','ArrowRight','Home','End'].includes(event.key)){event.preventDefault();const i=tabs.indexOf(currentWorkTab);const next=tabs[event.key==='Home'?0:event.key==='End'?2:(i+(event.key==='ArrowLeft'?2:1))%3];selectTab(next);$('#'+next+'-tab').focus();}});
for(const dialog of document.querySelectorAll('dialog'))dialog.addEventListener('click',event=>{if(event.target===dialog){const r=dialog.getBoundingClientRect();if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom)dialog.close();}});
let currentPrompt='';
let contributionIdentity='github';
function promptLines(amount){
 const project=projects.find(p=>p.id===$('#contribution-project').value);
 return [
  project?['Contribute ',`${amount}%`,' of my weekly agent budget to ',project.name,' via SpareSwarm.']:['Contribute ',`${amount}%`,' of my weekly agent budget to ','anything',' on SpareSwarm.'],
  contributionIdentity==='github'?['Share it using ','my own GitHub identity','.']:['Share it ','anonymously','.']
 ];
}
function updateBudget(){
 const amount=Number($('#budget-slider').value);
 $('#budget-output').innerHTML=`${amount}<span>%</span>`;
 $('#budget-slider').style.setProperty('--share',`${amount}%`);
 $('#budget-slider').setAttribute('aria-valuetext',`${amount} percent of my total weekly agent budget`);
 const prompt=$('#agent-prompt');
 if(amount===0){currentPrompt='';prompt.innerHTML='<span class="command-muted"># Choose a share above to generate an instruction.</span>';}
 else{const lines=promptLines(amount);currentPrompt=lines.map(parts=>parts.join('')).join('\n');prompt.innerHTML=lines.map((parts,lineIndex)=>parts.map((part,i)=>lineIndex===1&&i===2?'':lineIndex===1&&i===1?`<span class="identity-phrase"><button type="button" class="prompt-identity" aria-label="Change sharing identity" title="Click to change sharing identity">${esc(part)}</button>${esc(parts[i+1]||'')}</span>`:(i===1||i===3)?`<button type="button" class="prompt-identity prompt-value" data-edit="${i===1?'budget':'project'}" aria-label="Change ${i===1?'budget':'project'}">${esc(part)}</button>`:esc(part)).join('')).join('\n');}
 prompt.classList.toggle('is-empty',amount===0);
 $('#copy-prompt').disabled=amount===0;
}
$('#budget-slider').addEventListener('input',updateBudget);

const valueEditor=document.createElement('div');valueEditor.className='prompt-value-editor';valueEditor.hidden=true;document.body.append(valueEditor);
$('#agent-prompt').addEventListener('click',event=>{
 const button=event.target.closest('.prompt-identity');if(!button)return;
 const type=button.dataset.edit;
 if(!type){contributionIdentity=contributionIdentity==='github'?'anonymous':'github';updateBudget();$('#agent-prompt .prompt-identity:not([data-edit])').focus();return;}
 valueEditor.innerHTML=type==='budget'?'<label class="inline-budget-label">Weekly budget <output>'+$('#budget-slider').value+'%</output><input aria-label="Weekly agent budget" type="range" min="0" max="100" step="5" value="'+$('#budget-slider').value+'"></label>':['<button data-value="">anything</button>',...projects.map(p=>`<button data-value="${p.id}">${esc(p.name)}</button>`)].join('');
 const rect=button.getBoundingClientRect();valueEditor.style.left=Math.min(rect.left,innerWidth-240)+'px';valueEditor.style.top=Math.min(rect.bottom+8,innerHeight-250)+'px';valueEditor.hidden=false;
 const input=valueEditor.querySelector('input');if(input){input.focus();input.addEventListener('input',()=>{if(input.value==='')return;$('#budget-slider').value=Math.max(0,Math.min(100,Number(input.value)));valueEditor.querySelector('output').textContent=input.value+'%';updateBudget();});}else valueEditor.querySelector('button').focus();
});
valueEditor.addEventListener('click',event=>{const option=event.target.closest('[data-value]');if(!option)return;$('#contribution-project').value=option.dataset.value;pickerToggle.querySelector('span').textContent=option.textContent;updateBudget();valueEditor.hidden=true;});
document.addEventListener('click',event=>{if(!event.target.closest('.prompt-value-editor,.prompt-value'))valueEditor.hidden=true;});
document.addEventListener('keydown',event=>{if(event.key==='Escape'||event.key==='Enter'&&event.target.closest('.prompt-value-editor input'))valueEditor.hidden=true;});

$('#contribution-project').addEventListener('change',updateBudget);
const pickerToggle=$('#project-picker-toggle'),pickerMenu=$('#project-picker-menu');
function closeProjectPicker(){pickerMenu.hidden=true;pickerToggle.setAttribute('aria-expanded','false');}
pickerToggle.addEventListener('click',()=>{pickerMenu.hidden=!pickerMenu.hidden;pickerToggle.setAttribute('aria-expanded',String(!pickerMenu.hidden));if(!pickerMenu.hidden)pickerMenu.querySelector('button').focus();});
pickerMenu.addEventListener('click',event=>{const option=event.target.closest('[data-pick-project]');if(!option)return;$('#contribution-project').value=option.dataset.pickProject;pickerToggle.querySelector('span').textContent=option.textContent;closeProjectPicker();updateBudget();pickerToggle.focus();});
document.addEventListener('click',event=>{if(!event.target.closest('.project-picker'))closeProjectPicker();});
pickerMenu.addEventListener('keydown',event=>{const options=[...pickerMenu.querySelectorAll('button')],index=options.indexOf(document.activeElement);if(event.key==='Escape'){closeProjectPicker();pickerToggle.focus();}if(['ArrowDown','ArrowUp','Home','End'].includes(event.key)){event.preventDefault();options[event.key==='Home'?0:event.key==='End'?options.length-1:(index+(event.key==='ArrowDown'?1:options.length-1))%options.length].focus();}});

updateBudget();
$('#copy-prompt').addEventListener('click',async()=>{try{await navigator.clipboard.writeText(currentPrompt);const button=$('#copy-prompt');const icon=button.innerHTML;button.innerHTML='✓';button.setAttribute('aria-label','Instruction copied');clearTimeout(button.copyTimer);button.copyTimer=setTimeout(()=>{button.innerHTML=icon;button.setAttribute('aria-label','Copy instruction');},1600);}catch{const range=document.createRange();range.selectNodeContents($('#agent-prompt'));const selection=getSelection();selection.removeAllRanges();selection.addRange(range);toast('Select and copy the instruction');}});
// No simulated sessions: the ring is decorative until live sessions exist.
const workSessions=[];
let motionEnabled=!matchMedia('(prefers-reduced-motion: reduce)').matches;
function nodeState(session){return projects.find(p=>p.id===session.projectId).problems[session.problemIndex].done?'complete':'working';}
function refreshNetwork(){
 const host=$('#work-network');if(!host)return;
 if(!$('#swarm-art')){
  host.innerHTML=`<canvas id="swarm-art" aria-label="Dimensional ASCII swarm with agents moving on its surface" role="img"></canvas><div class="mobius-agents">${workSessions.map((a,i)=>`<button class="mobius-agent" data-agent="${a.id}" aria-label="${a.name}, ${projects.find(p=>p.id===a.projectId).name}, ${shortTasks[a.projectId][i-workSessions.findIndex(x=>x.projectId===a.projectId)]}" style="--agent-color:${['#b8dbef','#d8c1a1','#bbd6c4','#c4c0e2'][projects.findIndex(p=>p.id===a.projectId)]}"></button>`).join('')}</div><div id="agent-hover" class="agent-hover" role="tooltip" hidden></div>`;
 }
 startSwarmArt();
}
let artFrame=0,artPhase=0,artRotation=0,artPulseTime=0,artLastTime=0,hoveredAgent=null;
const fieldPointer={x:-1000,y:-1000,active:false,energy:0};
const shape={major:1.03,tube:.37,flatten:.76};
const artPoints=[];
function shapeCentre(u){
 const r=shape.major+.075*Math.cos(3*u)+.045*Math.sin(5*u);
 return [r*Math.cos(u),r*Math.sin(u),0];
}
function ribbonPoint(u,phi){
 const centre=shapeCentre(u),tube=shape.tube+.04*Math.sin(3*u+phi);
 return [centre[0]+tube*Math.cos(phi)*Math.cos(u),centre[1]+tube*Math.cos(phi)*Math.sin(u),centre[2]+tube*Math.sin(phi)];
}
function agentVolumePosition(index,phase){
 const u=index/workSessions.length*Math.PI*2+phase*(.82+(index%4)*.12),centre=shapeCentre(u);
 const radial=shape.tube*1.15*Math.sin(u*1.8+index*.73+phase*.6);
 return [centre[0]+radial*Math.cos(u),centre[1]+radial*Math.sin(u),centre[2]+shape.tube*.95*Math.cos(u*1.5+index*.61+phase*.8)];
}
function buildShape(){
 artPoints.length=0;
 for(let u=0;u<Math.PI*2;u+=.026){for(let phi=0;phi<Math.PI*2;phi+=.065){
  const p=ribbonPoint(u,phi),pu=ribbonPoint(u+.001,phi),pv=ribbonPoint(u,phi+.001);
  const a=pu.map((x,i)=>x-p[i]),b=pv.map((x,i)=>x-p[i]);
  let n=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
  const len=Math.hypot(...n);n=n.map(x=>x/len);
  const centre=shapeCentre(u),radial=p.map((x,i)=>x-centre[i]);
  if(n.reduce((sum,x,i)=>sum+x*radial[i],0)<0)n=n.map(x=>-x);
  artPoints.push({p,n});
 }}
}
buildShape();
function startSwarmArt(){
 cancelAnimationFrame(artFrame);
 const canvas=$('#swarm-art'),ctx=canvas.getContext('2d'),host=$('#work-network');
 const w=host.clientWidth,h=host.clientHeight,dpr=Math.min(devicePixelRatio||1,2);
 canvas.width=Math.round(w*dpr);canvas.height=Math.round(h*dpr);ctx.setTransform(dpr,0,0,dpr,0,0);
 const buttons=[...host.querySelectorAll('.mobius-agent')];
 const agentColours=buttons.map(b=>{const hex=getComputedStyle(b).getPropertyValue('--agent-color').trim();return [parseInt(hex.slice(1,3),16),parseInt(hex.slice(3,5),16),parseInt(hex.slice(5,7),16)];});
 const ca=Math.cos(.79),sa=Math.sin(.79),cb=Math.cos(.20),sb=Math.sin(.20);
 let spinC=Math.cos(artRotation),spinS=Math.sin(artRotation);
 function transform(p){const x=p[0]*spinC-p[1]*spinS,y=p[0]*spinS+p[1]*spinC,z=p[2],y1=y*ca-z*sa,z1=y*sa+z*ca;return [x*cb+z1*sb,y1,-x*sb+z1*cb];}
 const scale=Math.min(w*.34,h*.47);
 function project(p){const r=transform(p),k=3.8/(3.8-r[2]);return {x:w*.5+r[0]*scale*k,y:h*.49+r[1]*scale*shape.flatten*k,z:r[2]};}
 let skip=0;
 function draw(time){
  const dt=Math.min((time-artLastTime)/1000||0,.05);artLastTime=time;
  if(motionEnabled&&!document.hidden&&!hoveredAgent){artPhase+=dt*.055;artRotation+=dt*.04;artPulseTime+=dt;}
  spinC=Math.cos(artRotation);spinS=Math.sin(artRotation);
  fieldPointer.energy+=(Number(fieldPointer.active)-fieldPointer.energy)*(1-Math.exp(-dt*9));
  if(++skip%2&&motionEnabled){artFrame=requestAnimationFrame(draw);return;}
  ctx.clearRect(0,0,w,h);
  const cw=w<500?4.5:6,ch=w<500?7:9,cols=Math.ceil(w/cw),rows=Math.ceil(h/ch),depth=new Float32Array(cols*rows).fill(-999),light=new Float32Array(cols*rows);
  const agentPositions=buttons.map((_,i)=>project(agentVolumePosition(i,artPhase)));
  const agentFields=agentPositions.flatMap((q,i)=>[
   {...q,colour:agentColours[i],strength:.75+(.5+.5*Math.sin(artPulseTime*1.1+i*.89))*.3,radius:(w<500?22:30)*(1+Math.sin(artPulseTime*2.4+i*.89)*.08)},
   {...project(agentVolumePosition(i,artPhase-.095)),colour:agentColours[i],strength:.16,radius:w<500?18:23}
  ]);
  for(const p of artPoints){const q=project(p.p),px=Math.floor(q.x/cw),py=Math.floor(q.y/ch);if(px<0||px>=cols||py<0||py>=rows)continue;const i=py*cols+px;if(q.z<=depth[i])continue;depth[i]=q.z;const n=transform(p.n);light[i]=Math.max(.08,Math.min(1,.24-n[0]*.25-n[1]*.43+n[2]*.67));}
  ctx.font=`${ch+1}px ui-monospace, SFMono-Regular, Consolas, monospace`;ctx.textAlign='center';ctx.textBaseline='middle';const chars=' .,:;=+*#';
  for(let i=0;i<depth.length;i++){
   if(depth[i]===-999)continue;
   const l=light[i],value=Math.round(44+l*101);let x=(i%cols)*cw+cw/2,y=Math.floor(i/cols)*ch+ch/2;
   let warpX=0,warpY=0,red=0,green=0,blue=0,glyphEnergy=0;
   for(const field of agentFields){
    const dx=x-field.x,dy=y-field.y,d2=dx*dx+dy*dy;
    if(d2>=field.radius*field.radius)continue;
    const distance=Math.sqrt(d2)||1,influence=Math.pow(1-distance/field.radius,2)*field.strength/(1+Math.abs(depth[i]-field.z)*2);
    const force=11;warpX+=(dx-dy*.5)/distance*force*influence;warpY+=(dy+dx*.5)/distance*force*influence;
    glyphEnergy+=influence;red+=field.colour[0]*influence*.65;green+=field.colour[1]*influence*.65;blue+=field.colour[2]*influence*.65;
   }
   x+=Math.max(-35,Math.min(35,warpX));y+=Math.max(-35,Math.min(35,warpY));
   const dx=x-fieldPointer.x,dy=y-fieldPointer.y,d=Math.hypot(dx,dy),radius=w<500?65:90;
   if(d<radius&&fieldPointer.energy>.001){const push=Math.pow(1-d/radius,2)*36*fieldPointer.energy;x+=(dx/(d||1))*push;y+=(dy/(d||1))*push;}
   ctx.fillStyle=`rgb(${Math.min(255,Math.round(value-8+red))},${Math.min(255,Math.round(value+green))},${Math.min(255,Math.round(value+13+blue))})`;
   ctx.fillText(chars[Math.min(chars.length-1,Math.max(1,Math.floor(Math.min(1,l+glyphEnergy*.22)*(chars.length-1))))],x,y);
  }
  // Agents are glyphs too: a hot core cell, a body of neighbouring cells and a fading tail of cells along the path.
  const agentCells=new Map();
  const heatCell=(x,y,heat,colour)=>{const px=Math.floor(x/cw),py=Math.floor(y/ch);if(px<0||px>=cols||py<0||py>=rows||heat<=.05)return;const i=py*cols+px,current=agentCells.get(i);if(!current||current.heat<heat)agentCells.set(i,{heat,colour});};
  for(let i=0;i<agentPositions.length;i++){
   const q=agentPositions[i],colour=agentColours[i],a=workSessions[i];
   const surface=depth[Math.floor(q.y/ch)*cols+Math.floor(q.x/cw)],visibility=Number.isFinite(surface)&&q.z<surface-.075?.55:1;
   const body=(hoveredAgent===a.id?1.45:1.12)+.06*Math.sin(artPulseTime*1.3+i),cx=Math.floor(q.x/cw),cy=Math.floor(q.y/ch);
   for(let dy=-2;dy<=2;dy++)for(let dx=-3;dx<=3;dx++){const gx=(cx+dx+.5)*cw,gy=(cy+dy+.5)*ch,dist=Math.hypot((gx-q.x)/cw,(gy-q.y)/ch*1.15);heatCell(gx,gy,Math.max(0,body-dist*.6)*visibility,colour);}
   for(let k=1;k<=12;k++){const p=project(agentVolumePosition(i,artPhase-.026*k));heatCell(p.x,p.y,(1-k/13)*.5*visibility,colour);}
  }
  const agentChars=[['@',.95],['#',.65],['*',.42],['+',.27],[':',.14],['.',0]];
  for(const [i,cell] of agentCells){
   const x=(i%cols)*cw+cw/2,y=Math.floor(i/cols)*ch+ch/2,heat=Math.min(1,cell.heat),toWhite=heat>.9?1:Math.pow(heat,3)*.5;
   const grey=(cell.colour[0]+cell.colour[1]+cell.colour[2])/3,vivid=cell.colour.map(v=>Math.max(60,Math.min(255,grey+(v-grey)*2.6-30)));
   const r=Math.round(vivid[0]+(255-vivid[0])*toWhite),g=Math.round(vivid[1]+(255-vivid[1])*toWhite),b=Math.round(vivid[2]+(255-vivid[2])*toWhite);
   ctx.font=`${heat>.45?'bold ':''}${ch+1}px ui-monospace, SFMono-Regular, Consolas, monospace`;
   if(heat>.45){ctx.shadowColor=`rgba(${vivid.map(Math.round).join(',')},.95)`;ctx.shadowBlur=6+heat*12;}else ctx.shadowBlur=0;
   ctx.fillStyle=`rgba(${r},${g},${b},${(.35+.65*heat).toFixed(3)})`;
   ctx.fillText(agentChars.find(([,min])=>heat>=min)[0],x,y);
  }
  ctx.shadowBlur=0;ctx.font=`${ch+1}px ui-monospace, SFMono-Regular, Consolas, monospace`;
  buttons.forEach((button,i)=>{
   const q=agentPositions[i];
   button.style.left=`${q.x}px`;button.style.top=`${q.y}px`;const gx=Math.floor(q.x/cw),gy=Math.floor(q.y/ch),surface=depth[gy*cols+gx];button.style.opacity=Number.isFinite(surface)&&q.z<surface-.075?'.42':'1';button.style.zIndex=q.z<0?'2':'3';button.style.setProperty('--depth-scale',String(Math.max(.72,Math.min(1.13,.91+q.z*.16))));
   const a=workSessions[i];button.classList.toggle('is-complete',nodeState(a)==='complete');
   if(hoveredAgent===a.id){const tip=$('#agent-hover'),tw=tip.offsetWidth||230,th=tip.offsetHeight||90;tip.style.left=`${Math.max(8,Math.min(w-tw-8,q.x+15))}px`;tip.style.top=`${Math.max(8,Math.min(h-th-8,q.y-th-13))}px`;}
  });
  artFrame=requestAnimationFrame(draw);
 }
 artFrame=requestAnimationFrame(draw);
}
function showAgentTooltip(id){
 const a=workSessions.find(s=>s.id===id);if(!a)return;
 hoveredAgent=id;
 const p=projects.find(p=>p.id===a.projectId),q=p.problems[a.problemIndex],tip=$('#agent-hover');
 tip.innerHTML=`<strong>${a.name}</strong><span>${p.name} · ${nodeState(a)==='working'?'Working':'Completed'}</span><p>${esc(q.title)}</p>`;tip.hidden=false;
 document.querySelectorAll('.mobius-agent').forEach(b=>{b.setAttribute('aria-expanded',String(b.dataset.agent===id));b.setAttribute('aria-describedby',b.dataset.agent===id?'agent-hover':'');});
}
function hideAgentTooltip(){hoveredAgent=null;$('#agent-hover').hidden=true;}
function openAgent(id){
 const a=workSessions.find(s=>s.id===id);if(!a)return;
 const p=projects.find(p=>p.id===a.projectId),q=p.problems[a.problemIndex],state=nodeState(a);
 const dialog=$('#agent-dialog');
 $('#agent-detail').innerHTML=`<h2 id="agent-title">${a.name}</h2><p class="agent-state"><span class="agent-node-dot">${state==='complete'?'✓':''}</span>${state==='working'?'Working':'Completed'} on <button class="inline-project-link" data-agent-project="${p.id}">${p.name} ↗</button></p><h3 class="agent-task-title">${esc(q.title)}</h3><p class="agent-task-description">${esc(q.note)}</p><div class="agent-report"><span>Last report · ${a.minutes} min ago</span><p>${esc(state==='complete'?`Finished investigating ${q.title.toLowerCase()}. The example is ready for review.`:`Investigating ${q.title.toLowerCase()}. Preparing a small, repeatable example.`)}</p></div><p class="agent-disclosure">Example work session. This is not a live agent connection.</p>`;
 if($('#project-dialog').open)$('#project-dialog').close();if(!dialog.open)dialog.showModal();
}
$('#work-network').addEventListener('pointermove',e=>{if(e.pointerType==='touch')return;const rect=$('#work-network').getBoundingClientRect();fieldPointer.x=e.clientX-rect.left;fieldPointer.y=e.clientY-rect.top;fieldPointer.active=true;});
$('#work-network').addEventListener('pointerleave',()=>{fieldPointer.active=false;});
$('#work-network').addEventListener('pointerover',e=>{const button=e.target.closest('[data-agent]');if(button)showAgentTooltip(button.dataset.agent);});
$('#work-network').addEventListener('pointerout',e=>{if(e.target.closest('[data-agent]')&&!e.relatedTarget?.closest('[data-agent]'))hideAgentTooltip();});
$('#work-network').addEventListener('focusin',e=>{const button=e.target.closest('[data-agent]');if(button)showAgentTooltip(button.dataset.agent);});
$('#work-network').addEventListener('focusout',hideAgentTooltip);
$('#work-network').addEventListener('click',e=>{const button=e.target.closest('[data-agent]');if(button){hideAgentTooltip();openAgent(button.dataset.agent);}});
$('#agent-detail').addEventListener('click',e=>{const b=e.target.closest('[data-agent-project]');if(b){$('#agent-dialog').close();openProject(b.dataset.agentProject);}});
matchMedia('(prefers-reduced-motion: reduce)').addEventListener('change',e=>{motionEnabled=!e.matches;refreshNetwork();});
let lastNetworkWidth=0;new ResizeObserver(entries=>{const width=entries[0].contentRect.width;if(Math.abs(width-lastNetworkWidth)>1){lastNetworkWidth=width;refreshNetwork();}}).observe($('#work-network'));
refreshNetwork();

renderProjects();renderActivity();
async function loadBoard(){
 try{
 const responses=await Promise.all([fetch('/api/projects'),fetch('/api/tasks')]);
 if(responses.some(r=>!r.ok))throw new Error('API unavailable');
 const [items,tasks]=await Promise.all(responses.map(r=>r.json()));liveTasks=tasks;
 projects.splice(0,projects.length,...items.map(p=>({...p,agents:[],agentNames:[],problems:[],color:'',symbol:p.name[0],question:p.description})));
 for(const p of projects)projectBriefs[p.id]={category:'OPEN',description:p.description};
 pickerMenu.innerHTML='<button type="button" data-pick-project="">anything</button>'+projects.map(p=>`<button type="button" data-pick-project="${esc(p.id)}">${esc(p.name)}</button>`).join('');
 renderProjects();renderActivity();
 }catch{ $('#projects').innerHTML='<p class="empty">Could not load projects. Please reload.</p>'; }
}
document.addEventListener('click',async event=>{const button=event.target.closest('[data-live-task]');if(!button)return;try{const response=await fetch('/api/tasks/'+encodeURIComponent(button.dataset.liveTask));if(!response.ok)throw new Error();const t=await response.json();$('#project-detail').innerHTML=`<h2 id="project-title">${esc(t.title)}</h2><p>${esc(t.description)}</p><p>${esc(t.status)}</p>`+t.updates.map(u=>`<div class="activity-item"><p>${esc(u.content)}</p>${safeLink(u.link)?`<a href="${esc(u.link)}" target="_blank" rel="noopener noreferrer">View work ↗</a>`:''}</div>`).join('');if(!$('#project-dialog').open)$('#project-dialog').showModal();}catch{toast('Could not load task');}});
loadBoard();
