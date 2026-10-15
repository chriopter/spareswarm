'use strict';
(async()=>{
 const target=document.querySelector('#task-detail');
 const pathMatch=location.pathname.match(/^\/tasks\/([^/]+)\/?$/);
 const id=pathMatch?decodeURIComponent(pathMatch[1]):new URLSearchParams(location.search).get('id');
 if(id&&!pathMatch)history.replaceState(null,'','/tasks/'+encodeURIComponent(id)+location.hash);
 if(!id){target.textContent='No task specified.';return;}
 try{
  const [taskResponse,projectsResponse]=await Promise.all([fetch('/api/tasks/'+encodeURIComponent(id)),fetch('/api/projects')]);
  if(!taskResponse.ok)throw new Error(taskResponse.status===404?'Task not found.':'Could not load task.');
  const task=await taskResponse.json();const projects=projectsResponse.ok?await projectsResponse.json():[];
  document.title=task.title+' · SpareSwarm';
  target.innerHTML=taskMarkup(task,projects.find(project=>project.id===task.project_id));
 }catch(error){target.textContent=error.message||'Could not load task.';}
})();
