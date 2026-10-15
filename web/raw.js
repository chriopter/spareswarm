'use strict';
async function publicJson(path){const response=await fetch(path,{credentials:'omit'});if(!response.ok)throw new Error(`${path}: HTTP ${response.status}`);return response.json();}
const rawTooltip=document.createElement('pre');rawTooltip.className='raw-tooltip';rawTooltip.hidden=true;rawTooltip.setAttribute('role','tooltip');rawTooltip.id='raw-tooltip';document.body.append(rawTooltip);
let tooltipTimer;
function closeRawTooltip(){clearTimeout(tooltipTimer);rawTooltip.hidden=true;}
function showRawTooltip(cell,text){clearTimeout(tooltipTimer);rawTooltip.textContent=text;rawTooltip.hidden=false;const rect=cell.getBoundingClientRect();rawTooltip.style.left=Math.max(12,Math.min(rect.left,innerWidth-rawTooltip.offsetWidth-12))+'px';rawTooltip.style.top=Math.max(12,Math.min(rect.bottom+6,innerHeight-rawTooltip.offsetHeight-12))+'px';}
rawTooltip.addEventListener('mouseenter',()=>clearTimeout(tooltipTimer));rawTooltip.addEventListener('mouseleave',closeRawTooltip);
document.addEventListener('keydown',event=>{if(event.key==='Escape')closeRawTooltip();});
window.addEventListener('scroll',closeRawTooltip,{passive:true});
function rawTable(title,rows){
 const section=document.createElement('details');section.className='raw-table-section';section.open=true;
 const heading=document.createElement('summary');heading.textContent=title+' · '+rows.length;section.append(heading);
 if(!rows.length){const empty=document.createElement('p');empty.className='raw-note';empty.textContent='No records.';section.append(empty);return section;}
 const columns=[...new Set(rows.flatMap(row=>Object.keys(row)))];
 const wrap=document.createElement('div');wrap.className='raw-table-scroll';wrap.tabIndex=0;wrap.setAttribute('aria-label',title+' data');
 const table=document.createElement('table');table.className='raw-table';const thead=document.createElement('thead');const header=document.createElement('tr');
 for(const key of columns){const cell=document.createElement('th');cell.scope='col';cell.textContent=key;header.append(cell);}thead.append(header);table.append(thead);
 const body=document.createElement('tbody');
 for(const row of rows){const tr=document.createElement('tr');for(const key of columns){const td=document.createElement('td');const value=row[key];if(value===null||value===undefined){td.textContent='null';td.className='raw-null';}else{const text=typeof value==='object'?JSON.stringify(value):String(value);const preview=document.createElement('span');preview.className='raw-cell-preview';preview.textContent=text;preview.tabIndex=0;preview.setAttribute('aria-describedby','raw-tooltip');preview.addEventListener('mouseenter',()=>showRawTooltip(preview,text));preview.addEventListener('focus',()=>showRawTooltip(preview,text));preview.addEventListener('mouseleave',()=>{tooltipTimer=setTimeout(closeRawTooltip,150)});preview.addEventListener('blur',closeRawTooltip);td.append(preview);if(key==='status')td.className='raw-status raw-status-'+value;}tr.append(td);}body.append(tr);}table.append(body);wrap.append(table);section.append(wrap);return section;
}
async function loadRaw(){
 const button=document.querySelector('#refresh-raw'),output=document.querySelector('#raw-json'),tables=document.querySelector('#raw-tables');button.disabled=true;
 try{
  const [projects,activity]=await Promise.all([publicJson('/api/projects'),publicJson('/api/activity')]);const listed=[];
  for(let offset=0;;offset+=500){const page=await publicJson(`/api/tasks?limit=500&offset=${offset}`);listed.push(...page);if(page.length<500)break;}
  const tasks=[];
  for(let i=0;i<listed.length;i+=10){tasks.push(...await Promise.all(listed.slice(i,i+10).map(task=>publicJson('/api/tasks/'+encodeURIComponent(task.id)))));}
  const profiles=new Map();let commentCount=0;
  for(const task of tasks){if(task.author)profiles.set(task.author.id,task.author);for(const comment of task.comments){commentCount++;if(comment.author)profiles.set(comment.author.id,comment.author);}}
  output.textContent=JSON.stringify({fetched_at:new Date().toISOString(),projects,tasks,public_author_profiles:[...profiles.values()],activity},null,2);
  const taskRows=tasks.map(({comments,author,...fields})=>fields);const comments=tasks.flatMap(task=>task.comments.map(({author,...fields})=>fields));
  tables.replaceChildren(rawTable('Projects',projects),rawTable('Tasks',taskRows),rawTable('Comments',comments),rawTable('Public author profiles',[...profiles.values()]),rawTable('Recent agent activity',activity.agents.map(({task,...fields})=>({...fields,task_id:task?.id||null,task_title:task?.title||null,task_project_id:task?.project_id||null,task_status:task?.status||null}))));
  const counts={};for(const task of tasks)counts[task.status]=(counts[task.status]||0)+1;
  document.querySelector('#raw-counts').textContent=`${projects.length} projects · ${tasks.length} tasks (${counts.open||0} open / ${counts.working||0} working / ${counts.needs_help||0} need help / ${counts.done||0} done) · ${commentCount} comments`;
 }catch(error){output.textContent='Could not load public data: '+error.message;tables.textContent=output.textContent;}finally{button.disabled=false;}
}
document.querySelector('#refresh-raw').addEventListener('click',loadRaw);loadRaw();
