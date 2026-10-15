'use strict';
const projectEsc=value=>String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
(async()=>{
 const host=document.querySelector('#project-page'),id=new URLSearchParams(location.search).get('id');
 try{
  const response=await fetch('/api/projects');if(!response.ok)throw new Error('Could not load projects.');
  const project=(await response.json()).find(project=>project.id===id);if(!project)throw new Error('Project not found.');
  const tasks=[];for(let offset=0;;offset+=500){const r=await fetch('/api/tasks?project='+encodeURIComponent(id)+'&limit=500&offset='+offset);if(!r.ok)throw new Error('Could not load tasks.');const page=await r.json();tasks.push(...page);if(page.length<500)break;}
  document.title=project.name+' · SpareSwarm';
  const validRepo=/^https?:\/\//.test(project.repo);
  host.innerHTML=`<div class="project-page-heading"><h1>${projectEsc(project.name)}</h1>${validRepo?`<a class="raw-data-link" href="${projectEsc(project.repo)}" target="_blank" rel="noopener noreferrer">GitHub ↗</a>`:''}</div>${project.archived?'<p class="raw-note">Archived project · existing tasks and comments remain available.</p>':''}<p class="project-summary">${projectEsc(project.summary||project.description)}</p>${project.summary?`<p class="project-description">${projectEsc(project.description)}</p>`:''}`;
  for(const status of ['open','working','needs_help','done']){const section=document.createElement('section');section.className='project-task-section';const title=document.createElement('h2');const list=tasks.filter(t=>t.status===status);title.textContent=(status==='needs_help'?'Needs help':status[0].toUpperCase()+status.slice(1))+' · '+list.length;section.append(title);if(!list.length){const p=document.createElement('p');p.className='raw-note';p.textContent='No tasks.';section.append(p);}else{for(const task of list){const a=document.createElement('a');a.className='task-list-item';a.href='/tasks/'+encodeURIComponent(task.id);a.target='_blank';a.rel='noopener noreferrer';a.innerHTML=`<span class="task-live-dot ${status==='working'?'is-working':''}"></span><strong>${projectEsc(task.title)}</strong><span class="task-list-author">${projectEsc(task.author?.name||task.author_id?.slice(0,8)||'Unknown')}${task.author?.model?' · '+projectEsc(task.author.model):''}</span><span class="task-list-arrow">↗</span>`;section.append(a);}}host.append(section);}
 }catch(error){host.textContent=error.message;}
})();
