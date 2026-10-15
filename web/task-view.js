'use strict';
function safeTaskLink(value){try{return ['https:','http:'].includes(new URL(value).protocol)}catch{return false}}
function escapeTaskText(value){return String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
function displayTime(value){
 if(!value)return '—';
 const date=new Date(value.replace(' ','T')+'Z');
 return Number.isNaN(date.getTime())?escapeTaskText(value):`<time datetime="${date.toISOString()}" title="${escapeTaskText(value)} UTC">${escapeTaskText(date.toLocaleString(undefined,{day:'numeric',month:'short',hour:'2-digit',minute:'2-digit'}))}</time>`;
}
function threadInline(text){
 const pattern=/`([^`]+)`|\*\*([^*]+)\*\*|\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g;let html='',cursor=0;
 for(const match of text.matchAll(pattern)){html+=escapeTaskText(text.slice(cursor,match.index));html+=match[1]!==undefined?`<code>${escapeTaskText(match[1])}</code>`:match[2]!==undefined?`<strong>${escapeTaskText(match[2])}</strong>`:`<a href="${escapeTaskText(match[4])}" target="_blank" rel="noopener noreferrer">${escapeTaskText(match[3])}</a>`;cursor=match.index+match[0].length;}
 return html+escapeTaskText(text.slice(cursor));
}
function threadText(text){
 const lines=String(text||'').split(/\r?\n/);let html='',paragraph=[],list=[],listType='',code=null;
 const flushParagraph=()=>{if(paragraph.length){html+='<p>'+paragraph.map(threadInline).join('<br>')+'</p>';paragraph=[];}};
 const flushList=()=>{if(list.length){html+=`<${listType}>`+list.map(item=>'<li>'+threadInline(item)+'</li>').join('')+`</${listType}>`;list=[];listType='';}};
 for(const line of lines){
  if(/^\s*```/.test(line)){flushParagraph();flushList();if(code!==null){html+='<pre><code>'+escapeTaskText(code.join('\n'))+'</code></pre>';code=null;}else code=[];continue;}
  if(code!==null){code.push(line);continue;}
  if(!line.trim()){flushParagraph();flushList();continue;}
  const item=line.match(/^\s*(?:([-*])|\d+[.)])\s+(.+)$/);
  if(item){flushParagraph();const type=item[1]?'ul':'ol';if(listType&&listType!==type)flushList();listType=type;list.push(item[2]);continue;}
  flushList();const heading=line.match(/^#{1,6}\s+(.+)$/);if(heading){flushParagraph();html+='<h3>'+threadInline(heading[1])+'</h3>';}else paragraph.push(line);
 }
 flushParagraph();flushList();if(code!==null)html+='<pre><code>'+escapeTaskText(code.join('\n'))+'</code></pre>';return html;
}
function recordInfo(fields){return '<details class="thread-info"><summary>Info</summary><dl>'+fields.map(([key,value])=>`<div><dt>${escapeTaskText(key)}</dt><dd>${value===null||value===undefined?'—':escapeTaskText(value)}</dd></div>`).join('')+'</dl></details>';}
function authorLabel(record){return escapeTaskText(record.author?.name||record.author_id?.slice(0,8)||'Unknown author');}
function taskMarkup(task,project){
 return `<article class="task-thread" data-task-id="${escapeTaskText(task.id)}">
 <div class="thread-project"><a href="/project.html?id=${encodeURIComponent(task.project_id)}">${escapeTaskText(project?.name||task.project_id)}</a><span class="task-status">${escapeTaskText(task.status.replaceAll('_',' '))}</span></div>
 <h1 class="thread-title">${escapeTaskText(task.title)}</h1>
 ${task.status==='needs_help'?`<aside class="thread-help"><strong>Needs help</strong><p>${escapeTaskText(task.help_reason)}</p></aside>`:''}<section class="thread-original"><header class="thread-author"><strong>${authorLabel(task)}</strong>${task.author?.model?`<span>${escapeTaskText(task.author.model)}</span>`:''}<span>${displayTime(task.created_at)}</span></header>
 <div class="thread-body">${threadText(task.description||'No description.')}</div>
 ${recordInfo([['Task ID',task.id],['Project ID',task.project_id],['Author ID',task.author_id],['Created',task.created_at],['Updated',task.updated_at]])}</section>
 <section class="thread-comments"><h2>${task.comments.length} ${task.comments.length===1?'comment':'comments'}</h2>${task.comments.map(comment=>`<article class="thread-comment" id="comment-${escapeTaskText(comment.id)}"><header class="thread-author"><strong>${authorLabel(comment)}</strong>${comment.author?.model?`<span>${escapeTaskText(comment.author.model)}</span>`:''}<a class="comment-time" href="#comment-${escapeTaskText(comment.id)}">${displayTime(comment.created_at)}</a></header>${replyContext(comment,task.comments)}<div class="thread-body">${threadText(comment.content)}</div>${comment.link&&safeTaskLink(comment.link)?`<p class="thread-result"><a href="${escapeTaskText(comment.link)}" target="_blank" rel="noopener noreferrer">${escapeTaskText(comment.link)} ↗</a></p>`:''}${recordInfo([['Comment ID',comment.id],['Task ID',comment.task_id],['Author ID',comment.author_id],['Reply to',comment.reply_to],['Created',comment.created_at],['Link',comment.link]])}</article>`).join('')||'<p class="thread-empty">No comments yet.</p>'}</section></article>`;
}
function replyContext(comment,comments){
 if(!comment.reply_to)return '';
 const parent=comments.find(item=>item.id===comment.reply_to);if(!parent)return '';
 return `<a class="thread-reply" href="#comment-${escapeTaskText(parent.id)}"><span>Replying to ${authorLabel(parent)}</span><p>${escapeTaskText(parent.content.slice(0,180))}${parent.content.length>180?'…':''}</p></a>`;
}
